//! Profiles 主视图：左 list 右 details + 顶/底状态栏。
//!
//! 行为（spec §9.1）：
//! - 左列高亮一个 profile，右列显示其 settings.json 的 env 块（key 脱敏）。
//! - 顶栏：ais 版本 / claude 版本 / 配置目录。
//! - 底栏：可用快捷键速览。
//!
//! 快捷键：
//! - 导航：↑↓/jk、p / K / d 切面板、? help、q/Esc quit。
//! - 写/启动：n new、e edit、r rename（弹出输入框）、x delete（弹出确认框）、Enter launch。
//!   r 与 x 在主视图上叠加 overlay；overlay 处于活动期时主键位被屏蔽。

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

use crate::profile::Index;
use crate::settings::Settings;
use crate::tui::app::{App, Mode};

#[derive(Debug, Default)]
pub struct State {
    pub list: ListState,
    pub names: Vec<String>,
    /// 弹出 rename 输入框时持有的 InputField；非 None 即处于"重命名输入态"。
    pub rename_input: Option<crate::tui::widgets::InputField>,
    /// 与 `rename_input` 同步出现：原名，用于 commit 时定位旧 settings/index 项。
    pub renaming_from: Option<String>,
    /// 弹出删除确认框时持有的"待删 profile 名"；非 None 即处于"确认删除态"。
    pub confirm_delete: Option<String>,
}

impl State {
    pub fn new(index: &Index) -> Self {
        let names: Vec<String> = index.entries.keys().cloned().collect();
        let mut list = ListState::default();
        if !names.is_empty() {
            list.select(Some(0));
        }
        Self {
            list,
            names,
            ..Self::default()
        }
    }

    /// 仅 rename 输入框算"输入态"——拦截全局 ?/q 等捷径。
    /// 删除确认框只接受 y/n/Esc，并不消费字母键，所以无需进入输入态。
    pub fn is_in_input_mode(&self) -> bool {
        self.rename_input.is_some()
    }

    pub fn selected_name(&self) -> Option<&str> {
        self.list
            .selected()
            .and_then(|i| self.names.get(i).map(String::as_str))
    }
}

pub fn handle_key(app: &mut App, k: KeyEvent) {
    // 先处理两个 overlay：rename 输入框 / delete 确认框。
    // 这里要避免把 `app.mode` 长期借出去——后续要用 `&mut App` 调写盘 helper。
    // 只在"读"阶段拿引用，其余阶段先复制出来用值。

    // ===== rename overlay =====
    let in_rename = matches!(
        &app.mode,
        Mode::Profiles(s) if s.rename_input.is_some()
    );
    if in_rename {
        handle_rename_overlay(app, k);
        return;
    }

    // ===== delete-confirm overlay =====
    let pending_delete = match &app.mode {
        Mode::Profiles(s) => s.confirm_delete.clone(),
        _ => None,
    };
    if let Some(name) = pending_delete {
        handle_delete_overlay(app, k, &name);
        return;
    }

    // ===== 主键位 =====
    let Mode::Profiles(state) = &mut app.mode else {
        return;
    };
    match k.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.running = false;
        }
        KeyCode::Down | KeyCode::Char('j') if !state.names.is_empty() => {
            let i = state.list.selected().unwrap_or(0);
            let next = (i + 1).min(state.names.len() - 1);
            state.list.select(Some(next));
        }
        KeyCode::Up | KeyCode::Char('k') if !state.names.is_empty() => {
            let i = state.list.selected().unwrap_or(0);
            state.list.select(Some(i.saturating_sub(1)));
        }
        KeyCode::Char('p') => {
            app.mode = Mode::Providers(crate::tui::views::providers::State::default());
        }
        KeyCode::Char('K') => {
            app.mode = Mode::Keys(crate::tui::views::keys::State::default());
        }
        KeyCode::Char('d') => {
            app.mode = Mode::Doctor(crate::tui::views::doctor::State);
        }
        KeyCode::Char('n') => {
            app.mode = Mode::Wizard(Box::default());
        }
        KeyCode::Char('e') => {
            if let Some(name) = state.selected_name().map(str::to_string) {
                let s = crate::tui::views::wizard::State::for_edit(&name, app);
                app.mode = Mode::Wizard(Box::new(s));
            }
        }
        KeyCode::Char('r') => {
            if let Some(name) = state.selected_name().map(str::to_string) {
                state.rename_input = Some(
                    crate::tui::widgets::InputField::new("rename")
                        .with_max_len(64)
                        .with_initial(&name),
                );
                state.renaming_from = Some(name);
            }
        }
        KeyCode::Char('x') => {
            if let Some(name) = state.selected_name().map(str::to_string) {
                state.confirm_delete = Some(name);
            }
        }
        KeyCode::Enter => {
            if let Some(name) = state.selected_name().map(str::to_string) {
                app.launch_target = Some(name);
                app.running = false;
            }
        }
        _ => {}
    }
}

/// rename 输入框处于活动状态时的事件：Enter 提交、Esc 取消、字符/退格编辑。
fn handle_rename_overlay(app: &mut App, k: KeyEvent) {
    match k.code {
        KeyCode::Esc => {
            if let Mode::Profiles(s) = &mut app.mode {
                s.rename_input = None;
                s.renaming_from = None;
            }
        }
        KeyCode::Backspace => {
            if let Mode::Profiles(s) = &mut app.mode
                && let Some(field) = s.rename_input.as_mut()
            {
                field.pop();
            }
        }
        KeyCode::Char(c) => {
            if let Mode::Profiles(s) = &mut app.mode
                && let Some(field) = s.rename_input.as_mut()
            {
                field.push(c);
            }
        }
        KeyCode::Enter => {
            // 1) 抽出 from / to，然后清掉 overlay 状态，避免后续 helper 调用时再次借用 state。
            let (from, to) = match &mut app.mode {
                Mode::Profiles(s) => {
                    let to = s
                        .rename_input
                        .as_ref()
                        .map(|f| f.value().trim().to_string())
                        .unwrap_or_default();
                    let from = s.renaming_from.clone().unwrap_or_default();
                    s.rename_input = None;
                    s.renaming_from = None;
                    (from, to)
                }
                _ => return,
            };
            // 2) 调用写盘 helper（需要 `&mut App`，所以必须先释放 state 的可变借用）。
            match rename_profile(app, &from, &to) {
                Ok(()) => app.set_toast(crate::tui::widgets::Toast::success(format!(
                    "renamed `{from}` -> `{to}`"
                ))),
                Err(e) => app.set_toast(crate::tui::widgets::Toast::error(format!(
                    "rename failed: {e}"
                ))),
            }
            // 3) 用最新 index 重建 list，让 UI 立即反映改名结果。
            if let Mode::Profiles(s) = &mut app.mode {
                *s = State::new(&app.index);
            }
        }
        _ => {}
    }
}

/// delete-confirm overlay：y 删除、n/Esc 取消、其它键忽略。
fn handle_delete_overlay(app: &mut App, k: KeyEvent, name: &str) {
    match k.code {
        KeyCode::Char('y') => {
            let result = crate::profile::delete(&app.paths, name);
            match result {
                Ok(()) => {
                    if let Err(e) = app.reload_index() {
                        app.set_toast(crate::tui::widgets::Toast::error(format!(
                            "deleted `{name}` but reload failed: {e}"
                        )));
                    } else {
                        app.set_toast(crate::tui::widgets::Toast::success(format!(
                            "deleted `{name}`"
                        )));
                    }
                }
                Err(e) => {
                    app.set_toast(crate::tui::widgets::Toast::error(format!(
                        "delete failed: {e}"
                    )));
                }
            }
            // 无论成功/失败：清掉确认状态，并用最新 index 重建列表。
            if let Mode::Profiles(s) = &mut app.mode {
                *s = State::new(&app.index);
            }
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            if let Mode::Profiles(s) = &mut app.mode {
                s.confirm_delete = None;
            }
        }
        _ => {}
    }
}

/// 把 `from` 重命名为 `to`：移动 settings_<name>.json 文件 + 重写 .ais-index.toml。
///
/// 故意从磁盘加载一份新 Index（而非直接改 `app.index`），以应对外部进程刚刚改过文件的情形；
/// 失败时尽力把已重命名的 settings 文件回滚回原名，避免索引与文件名分裂。
fn rename_profile(app: &mut App, from: &str, to: &str) -> Result<(), crate::Error> {
    crate::profile::validate_name(to)?;
    if from == to {
        return Ok(());
    }
    if app.index.entries.contains_key(to) {
        return Err(crate::Error::InvalidProfileName {
            name: to.into(),
            reason: "已存在同名 profile".into(),
        });
    }
    let from_path = app.paths.settings_for(from);
    let to_path = app.paths.settings_for(to);
    if to_path.exists() {
        return Err(crate::Error::InvalidProfileName {
            name: to.into(),
            reason: format!("目标文件已存在: {}", to_path.display()),
        });
    }
    std::fs::rename(&from_path, &to_path).map_err(|e| crate::Error::Io {
        path: from_path.clone(),
        source: e,
    })?;

    // 写索引；失败则把 settings 文件名改回去，保持文件系统与索引一致。
    let mut idx = match crate::profile::Index::load(&app.paths.claude_index()) {
        Ok(i) => i,
        Err(e) => {
            let _ = std::fs::rename(&to_path, &from_path);
            return Err(e);
        }
    };
    if let Some(entry) = idx.entries.remove(from) {
        idx.entries.insert(to.into(), entry);
    }
    if let Err(e) = idx.save(&app.paths.claude_index()) {
        let _ = std::fs::rename(&to_path, &from_path);
        return Err(e);
    }
    app.reload_index()?;
    Ok(())
}

pub fn draw(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let Mode::Profiles(state) = &app.mode else {
        return;
    };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // top status
            Constraint::Min(0),    // body
            Constraint::Length(2), // bottom keybindings
        ])
        .split(area);

    draw_top(frame, chunks[0], app);
    draw_body(frame, chunks[1], app, state);
    draw_bottom(frame, chunks[2]);

    // 两个 overlay：互斥（同一时刻最多有一个在进行中），按出现顺序覆盖到主视图上。
    if let Some(field) = &state.rename_input {
        let popup = crate::tui::widgets::centered_rect(60, 20, area);
        let from = state.renaming_from.as_deref().unwrap_or("");
        let title = format!("Rename `{from}`  (Enter 确认 / Esc 取消)");
        let body = field
            .render(true)
            .block(Block::default().borders(Borders::ALL).title(title));
        frame.render_widget(ratatui::widgets::Clear, popup);
        frame.render_widget(body, popup);
    }
    if let Some(name) = &state.confirm_delete {
        crate::tui::widgets::draw_confirm(
            frame,
            area,
            "Delete profile",
            &format!("将删除 settings_{name}.json 与索引项（key 不动）。继续？"),
        );
    }
}

fn draw_top(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let line = Line::from(vec![
        Span::styled("ais ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(env!("CARGO_PKG_VERSION")),
        Span::raw("  ── "),
        Span::raw(format!("{}", app.paths.root.display())),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

fn draw_body(frame: &mut Frame<'_>, area: Rect, app: &App, state: &State) {
    let h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    // 左：list
    let items: Vec<ListItem<'_>> = state
        .names
        .iter()
        .map(|n| {
            let entry = app.index.entries.get(n);
            let sub = entry
                .map(|e| {
                    format!(
                        "  provider={}  key={}  model={}",
                        e.provider, e.key_id, e.model
                    )
                })
                .unwrap_or_default();
            ListItem::new(vec![
                Line::from(Span::styled(n.clone(), Style::default().fg(Color::Cyan))),
                Line::from(Span::raw(sub)),
            ])
        })
        .collect();
    let mut list_state = state.list.clone();
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Profiles"))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(list, h[0], &mut list_state);

    // 右：details
    let detail_text = match state.selected_name() {
        Some(name) => render_details(name, app),
        None => "(no profiles — press [n] to create one)".to_string(),
    };
    let para = Paragraph::new(detail_text)
        .block(Block::default().borders(Borders::ALL).title("Details"))
        .wrap(Wrap { trim: false });
    frame.render_widget(para, h[1]);
}

fn render_details(name: &str, app: &App) -> String {
    let path = app.paths.settings_for(name);
    let mut s = format!("Path:  {}\n\n", path.display());
    match Settings::load(&path) {
        Ok(settings) => {
            s.push_str("env:\n");
            for (k, v) in &settings.env {
                let display = if k == "ANTHROPIC_API_KEY" {
                    redact(v)
                } else {
                    v.clone()
                };
                s.push_str(&format!("  {k} = {display}\n"));
            }
            if let Some(entry) = app.index.entries.get(name) {
                s.push_str(&format!(
                    "\nProvider: {}\nKey ID:   {}\nModel:    {}\nCreated:  {}\n",
                    entry.provider,
                    entry.key_id,
                    entry.model,
                    entry.created_at.format("%Y-%m-%d %H:%M:%S UTC")
                ));
            }
        }
        Err(e) => {
            s.push_str(&format!("(failed to read: {e})\n"));
        }
    }
    s
}

fn redact(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= 8 {
        "*".repeat(chars.len())
    } else {
        let head: String = chars.iter().take(4).collect();
        let tail: String = chars
            .iter()
            .rev()
            .take(4)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        format!("{head}...{tail}")
    }
}

fn draw_bottom(frame: &mut Frame<'_>, area: Rect) {
    let line1 = Line::from(vec![
        keybinding_span("↑↓"),
        Span::raw(" move  "),
        keybinding_span("Enter"),
        Span::raw(" launch  "),
        keybinding_span("n"),
        Span::raw(" new  "),
        keybinding_span("e"),
        Span::raw(" edit  "),
        keybinding_span("r"),
        Span::raw(" rename  "),
        keybinding_span("x"),
        Span::raw(" delete"),
    ]);
    let line2 = Line::from(vec![
        keybinding_span("p"),
        Span::raw(" providers  "),
        keybinding_span("K"),
        Span::raw(" keys  "),
        keybinding_span("d"),
        Span::raw(" doctor  "),
        keybinding_span("?"),
        Span::raw(" help  "),
        keybinding_span("q"),
        Span::raw(" quit"),
    ]);
    frame.render_widget(Paragraph::new(vec![line1, line2]), area);
}

fn keybinding_span(label: &str) -> Span<'_> {
    Span::styled(
        format!("[{label}]"),
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_long_key() {
        assert_eq!(redact("sk-aaaaaaaaafswv"), "sk-a...fswv");
    }

    #[test]
    fn redact_short_key_is_all_stars() {
        assert_eq!(redact("short"), "*****");
    }

    #[test]
    fn state_new_selects_first_when_nonempty() {
        use crate::profile::IndexEntry;
        use chrono::Utc;
        let mut idx = Index::default();
        idx.entries.insert(
            "a".into(),
            IndexEntry {
                provider: "p".into(),
                key_id: "k".into(),
                model: "m".into(),
                created_at: Utc::now(),
            },
        );
        let s = State::new(&idx);
        assert_eq!(s.list.selected(), Some(0));
        assert_eq!(s.names, vec!["a".to_string()]);
    }

    #[test]
    fn state_new_selects_none_when_empty() {
        let idx = Index::default();
        let s = State::new(&idx);
        assert_eq!(s.list.selected(), None);
        assert!(s.names.is_empty());
    }
}
