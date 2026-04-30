//! Doctor 面板（spec §9.4）：6 项只读检查；只报告，不自动修复。
//!
//! 设计要点：
//! - State 持有 `report` + `computed` 标志；首次进入面板时 `computed = false`，
//!   首个 key event 触发 `compute()`，把结果写回 State 并立即返回（不消耗这次按键）。
//!   一次绘制阶段会显示 "Computing..."，紧接着的 keystroke 触发计算，再下一帧才显示结果。
//! - `compute()` 顺序固定：[PATH 探测] → [version 解析] → [root 可写] → [credentials mode]
//!   → [index 一致性] → [providers 可加载]，与 spec §9.4 一一对应。
//! - 跨平台：`credentials.toml` 的 0o600 校验仅在 Unix 启用；其它平台报告为 Pass(skipped)。

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::claude;
use crate::providers;
use crate::tui::app::{App, Mode};

/// Doctor 面板状态。
///
/// `report` 在 `computed = false` 时为空 Vec；`compute()` 完成后被填充并 `computed = true`。
/// 切回 Profiles 时整个 State 被丢弃，因此无需手动 reset。
#[derive(Debug, Default)]
pub struct State {
    pub report: Vec<Item>,
    pub computed: bool,
}

#[derive(Debug, Clone)]
pub struct Item {
    pub label: String,
    pub status: Status,
    pub detail: String,
}

#[derive(Debug, Clone, Copy)]
pub enum Status {
    Pass,
    Fail,
}

impl State {
    /// Doctor 面板没有文本输入框；恒返回 false，让全局 ?/q 等快捷键正常生效。
    pub fn is_in_input_mode(&self) -> bool {
        false
    }
}

pub fn handle_key(app: &mut App, k: KeyEvent) {
    // 第一次按键：先把检查跑掉，本次按键不再做其它分派。
    // 借用形态：先 `&app.mode` 短借用判定 needs_compute，再独立调用 `compute(app)`
    // 以避免 `&mut app.mode` 与 `&App` 共存。
    let needs_compute = matches!(&app.mode, Mode::Doctor(s) if !s.computed);
    if needs_compute {
        let report = compute(app);
        if let Mode::Doctor(s) = &mut app.mode {
            s.report = report;
            s.computed = true;
        }
        return;
    }
    if matches!(k.code, KeyCode::Esc | KeyCode::Char('q')) {
        app.mode = Mode::Profiles(crate::tui::views::profiles::State::new(&app.index));
    }
}

fn compute(app: &App) -> Vec<Item> {
    vec![
        check_claude_in_path(),
        check_claude_version(),
        check_root_writable(&app.paths.root),
        check_credentials_perm(&app.paths.credentials()),
        check_index_consistency(app),
        check_providers_loadable(&app.paths.providers()),
    ]
}

fn check_claude_in_path() -> Item {
    match claude::probe_path() {
        Ok(p) => Item {
            label: "claude in PATH".into(),
            status: Status::Pass,
            detail: p.display().to_string(),
        },
        Err(e) => Item {
            label: "claude in PATH".into(),
            status: Status::Fail,
            detail: e.to_string(),
        },
    }
}

fn check_claude_version() -> Item {
    match claude::probe_path().and_then(|p| claude::probe_version(&p)) {
        Ok(v) => Item {
            label: "claude --version".into(),
            status: Status::Pass,
            detail: v,
        },
        Err(e) => Item {
            label: "claude --version".into(),
            status: Status::Fail,
            detail: e.to_string(),
        },
    }
}

fn check_root_writable(root: &std::path::Path) -> Item {
    use std::fs;
    let probe = root.join(".doctor-probe");
    let res = (|| -> std::io::Result<()> {
        fs::create_dir_all(root)?;
        fs::write(&probe, b"ok")?;
        fs::remove_file(&probe)?;
        Ok(())
    })();
    match res {
        Ok(()) => Item {
            label: "~/.ai-switch/ writable".into(),
            status: Status::Pass,
            detail: root.display().to_string(),
        },
        Err(e) => Item {
            label: "~/.ai-switch/ writable".into(),
            status: Status::Fail,
            detail: e.to_string(),
        },
    }
}

fn check_credentials_perm(path: &std::path::Path) -> Item {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if !path.exists() {
            return Item {
                label: "credentials.toml mode".into(),
                status: Status::Pass,
                detail: "(absent — will be created at first add)".into(),
            };
        }
        match std::fs::metadata(path) {
            Ok(m) => {
                let mode = m.permissions().mode() & 0o777;
                if mode & 0o077 == 0 {
                    Item {
                        label: "credentials.toml mode".into(),
                        status: Status::Pass,
                        detail: format!("0{mode:o}"),
                    }
                } else {
                    Item {
                        label: "credentials.toml mode".into(),
                        status: Status::Fail,
                        detail: format!("0{mode:o} (expected 0600)"),
                    }
                }
            }
            Err(e) => Item {
                label: "credentials.toml mode".into(),
                status: Status::Fail,
                detail: e.to_string(),
            },
        }
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Item {
            label: "credentials.toml mode".into(),
            status: Status::Pass,
            detail: "(skipped on this platform)".into(),
        }
    }
}

fn check_index_consistency(app: &App) -> Item {
    let dir = app.paths.claude_dir();
    let mut on_disk: Vec<String> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for ent in rd.flatten() {
            let name = ent.file_name().to_string_lossy().to_string();
            if let Some(stripped) = name
                .strip_prefix("settings_")
                .and_then(|s| s.strip_suffix(".json"))
            {
                on_disk.push(stripped.to_string());
            }
        }
    }
    let in_index: std::collections::BTreeSet<&str> =
        app.index.entries.keys().map(String::as_str).collect();
    let on_disk_set: std::collections::BTreeSet<&str> =
        on_disk.iter().map(String::as_str).collect();

    let orphans: Vec<&&str> = on_disk_set.difference(&in_index).collect();
    let dangling: Vec<&&str> = in_index.difference(&on_disk_set).collect();

    if orphans.is_empty() && dangling.is_empty() {
        Item {
            label: ".ais-index.toml consistency".into(),
            status: Status::Pass,
            detail: format!(
                "{} entries, {} files; matched",
                in_index.len(),
                on_disk.len()
            ),
        }
    } else {
        let mut detail = String::new();
        if !orphans.is_empty() {
            detail.push_str(&format!("orphans (on disk, not in index): {orphans:?}; "));
        }
        if !dangling.is_empty() {
            detail.push_str(&format!("dangling (in index, no file): {dangling:?}"));
        }
        Item {
            label: ".ais-index.toml consistency".into(),
            status: Status::Fail,
            detail,
        }
    }
}

fn check_providers_loadable(path: &std::path::Path) -> Item {
    match providers::load_all(path) {
        Ok(list) => Item {
            label: "providers.toml loadable".into(),
            status: Status::Pass,
            detail: format!("{} providers (builtins + user)", list.len()),
        },
        Err(e) => Item {
            label: "providers.toml loadable".into(),
            status: Status::Fail,
            detail: e.to_string(),
        },
    }
}

pub fn draw(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let Mode::Doctor(state) = &app.mode else {
        return;
    };
    let body = if state.computed {
        let mut out = String::new();
        for item in &state.report {
            let mark = match item.status {
                Status::Pass => "[OK]",
                Status::Fail => "[FAIL]",
            };
            out.push_str(&format!("{mark} {} — {}\n", item.label, item.detail));
        }
        out
    } else {
        "Computing...".to_string()
    };
    let para = Paragraph::new(body)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Doctor (d) — [Esc] back"),
        )
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: false });
    frame.render_widget(para, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_writable_passes_for_temp_dir() {
        let p = std::env::temp_dir().join(format!("ais-doctor-{}", std::process::id()));
        std::fs::create_dir_all(&p).unwrap();
        let item = check_root_writable(&p);
        assert!(matches!(item.status, Status::Pass));
        std::fs::remove_dir_all(&p).ok();
    }

    #[test]
    fn root_writable_fails_when_parent_is_a_regular_file() {
        // 跨平台构造：在临时目录里放一个普通文件作 "blocker"，让 root 指向该文件下的子路径。
        // create_dir_all 在任何平台都无法在普通文件之下创建目录，必失败。
        // 不使用 "/this/should/not/exist" 等绝对路径，因为它在 Windows 会解读为当前盘根，
        // 且在以 root 运行的 Unix CI 上可能反而能创建成功。
        let base = std::env::temp_dir().join(format!("ais-doctor-blocker-{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        let blocker = base.join("not-a-dir");
        std::fs::write(&blocker, b"x").unwrap();
        let bogus_root = blocker.join("sub");

        let item = check_root_writable(&bogus_root);

        std::fs::remove_dir_all(&base).ok();
        assert!(matches!(item.status, Status::Fail));
    }

    #[test]
    fn providers_loadable_passes_with_no_user_file() {
        let p = std::env::temp_dir().join(format!("ais-doctor-prov-{}", std::process::id()));
        let item = check_providers_loadable(&p.join("providers.toml"));
        assert!(matches!(item.status, Status::Pass));
    }
}
