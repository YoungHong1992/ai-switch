//! Providers 面板：列出 builtins + 用户自定义；用户自定义可增删改。
//!
//! 具体 CRUD：
//! - 增：从 catalog 末尾"+ 添加新 provider..."进入子表单
//! - 改：仅用户自定义 provider 可改 base_url / openai_base_url / auth / models_endpoint_path
//! - 删：仅用户自定义可删；删之前提示"已被 N 个 profile 使用"（基于 index 反查）

use std::collections::BTreeSet;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

use crate::catalog::{self, AuthScheme, Provider};
use crate::error::Error;
use crate::tui::app::{App, Mode};
use crate::tui::widgets::{InputField, Toast};

/// Providers 面板状态。
///
/// 子状态优先级（同一时刻只能有一个生效）：`form` > `confirm_delete` > 列表。
///
/// `form` 用 `Box` 包裹是为了把 `Mode::Providers(State)` 的占空控制在合理范围内
/// （否则 [InputField; 5] 会让 `Mode` 枚举触发 `clippy::large_enum_variant`）。
#[derive(Debug, Default)]
pub struct State {
    pub list: ListState,
    pub form: Option<Box<ProviderForm>>,
    pub confirm_delete: Option<String>,
}

impl State {
    pub fn is_in_input_mode(&self) -> bool {
        self.form.is_some()
    }
}

/// 增/改 user provider 的小表单。
///
/// 字段顺序固定：id / display_name / anthropic_base_url / openai_base_url / models_endpoint_path。
#[derive(Debug)]
pub struct ProviderForm {
    /// `Some(id)` 表示编辑既有；`None` 表示新增。
    pub editing_id: Option<String>,
    pub fields: [InputField; 5],
    pub focus: usize,
}

impl ProviderForm {
    pub fn new_add() -> Self {
        Self {
            editing_id: None,
            fields: [
                InputField::new("id"),
                InputField::new("display_name"),
                InputField::new("anthropic_base_url"),
                InputField::new("openai_base_url"),
                InputField::new("models_endpoint_path").with_initial("/v1/models"),
            ],
            focus: 0,
        }
    }

    pub fn from_existing(p: &Provider) -> Self {
        Self {
            editing_id: Some(p.id.clone()),
            fields: [
                InputField::new("id").with_initial(&p.id),
                InputField::new("display_name").with_initial(&p.display_name),
                InputField::new("anthropic_base_url")
                    .with_initial(p.anthropic_base_url.as_deref().unwrap_or("")),
                InputField::new("openai_base_url")
                    .with_initial(p.openai_base_url.as_deref().unwrap_or("")),
                InputField::new("models_endpoint_path").with_initial(&p.models_endpoint_path),
            ],
            focus: 0,
        }
    }

    /// 把表单序列化成 `(id, body)`，body 即 `[id]` section 大括号下的全部行。
    ///
    /// V1 简化：auth 固定为 "Bearer"（X-Api-Key 走 M5 之后的 dropdown）。
    pub fn to_toml_section(&self) -> Result<(String, String), Error> {
        let id = self.fields[0].value().trim().to_string();
        if id.is_empty() {
            return Err(Error::InvalidKeyId {
                id: id.clone(),
                reason: "provider id 不能为空".into(),
            });
        }

        let display_name = self.fields[1].value().trim();
        let anth = self.fields[2].value().trim();
        let oa = self.fields[3].value().trim();
        let models = self.fields[4].value().trim();

        let mut body = String::new();
        if !display_name.is_empty() {
            body.push_str(&format!("display_name = \"{display_name}\"\n"));
        }
        if !anth.is_empty() {
            body.push_str(&format!("anthropic_base_url = \"{anth}\"\n"));
        }
        if !oa.is_empty() {
            body.push_str(&format!("openai_base_url = \"{oa}\"\n"));
        }
        body.push_str("auth = \"Bearer\"\n");
        body.push_str(&format!("models_endpoint_path = \"{models}\"\n"));

        Ok((id, body))
    }
}

/// 当前 providers 面板处于哪种交互阶段。
///
/// `handle_key` 先读 app.mode 拿到 phase，再短时借用 dispatch 到对应 handler，
/// 避免长时间持有 `&mut app.mode` 导致后续 `&mut app` 操作冲突。
#[derive(Clone, Copy)]
enum Phase {
    Form,
    Confirm,
    List,
}

pub fn handle_key(app: &mut App, k: KeyEvent) {
    let phase = match &app.mode {
        Mode::Providers(s) => {
            if s.form.is_some() {
                Phase::Form
            } else if s.confirm_delete.is_some() {
                Phase::Confirm
            } else {
                Phase::List
            }
        }
        _ => return,
    };
    match phase {
        Phase::Form => handle_form_key(app, k),
        Phase::Confirm => handle_confirm_key(app, k),
        Phase::List => handle_list_key(app, k),
    }
}

fn handle_list_key(app: &mut App, k: KeyEvent) {
    match k.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = Mode::Profiles(crate::tui::views::profiles::State::new(&app.index));
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let total = app.providers.len() + 1; // 含末位 "+ 添加新 provider..."
            if let Mode::Providers(state) = &mut app.mode {
                let i = state.list.selected().unwrap_or(0);
                let next = (i + 1).min(total.saturating_sub(1));
                state.list.select(Some(next));
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if let Mode::Providers(state) = &mut app.mode {
                let i = state.list.selected().unwrap_or(0);
                state.list.select(Some(i.saturating_sub(1)));
            }
        }
        KeyCode::Enter | KeyCode::Char('a') => {
            let providers_len = app.providers.len();
            if let Mode::Providers(state) = &mut app.mode {
                let i = state.list.selected().unwrap_or(0);
                if i == providers_len {
                    state.form = Some(Box::new(ProviderForm::new_add()));
                }
            }
        }
        KeyCode::Char('e') => match currently_selected_user_provider(app) {
            Some(p) => {
                if let Mode::Providers(state) = &mut app.mode {
                    state.form = Some(Box::new(ProviderForm::from_existing(&p)));
                }
            }
            None => app.set_toast(Toast::info("内置 provider 不可编辑")),
        },
        KeyCode::Char('x') => match currently_selected_user_provider(app) {
            Some(p) => {
                if let Mode::Providers(state) = &mut app.mode {
                    state.confirm_delete = Some(p.id);
                }
            }
            None => app.set_toast(Toast::info("内置 provider 不可删除")),
        },
        _ => {}
    }
}

/// 取当前选中的 user provider（即非内置）；无效或选中末位 "+ 添加" 时返回 None。
fn currently_selected_user_provider(app: &App) -> Option<Provider> {
    let Mode::Providers(state) = &app.mode else {
        return None;
    };
    let i = state.list.selected()?;
    if i >= app.providers.len() {
        return None;
    }
    let p = &app.providers[i];
    let builtin_ids: BTreeSet<String> = catalog::builtins().into_iter().map(|p| p.id).collect();
    // V1 简化：仅允许编辑/删除非内置 id；override 内置的能力放到 M5 之后。
    if builtin_ids.contains(&p.id) {
        None
    } else {
        Some(p.clone())
    }
}

fn handle_confirm_key(app: &mut App, k: KeyEvent) {
    let id = match &app.mode {
        Mode::Providers(s) => match &s.confirm_delete {
            Some(id) => id.clone(),
            None => return,
        },
        _ => return,
    };

    match k.code {
        KeyCode::Char('y') => {
            match remove_user_provider(app, &id) {
                Ok(()) => app.set_toast(Toast::success(format!("provider `{id}` removed"))),
                Err(e) => app.set_toast(Toast::error(format!("remove failed: {e}"))),
            }
            if let Mode::Providers(s) = &mut app.mode {
                s.confirm_delete = None;
            }
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            if let Mode::Providers(s) = &mut app.mode {
                s.confirm_delete = None;
            }
        }
        _ => {}
    }
}

/// 表单子状态执行结果，提取出来以释放 `&mut app.mode` 借用，再做后续 IO。
enum FormOutcome {
    Stay,
    Close,
    TryCommit,
}

fn handle_form_key(app: &mut App, k: KeyEvent) {
    // 1. 在短借用里完成所有纯字段级别的更新；返回 outcome 表示后续动作。
    let outcome = {
        let Mode::Providers(state) = &mut app.mode else {
            return;
        };
        let Some(form) = state.form.as_deref_mut() else {
            return;
        };

        match k.code {
            KeyCode::Esc => FormOutcome::Close,
            KeyCode::Tab => {
                form.focus = (form.focus + 1) % form.fields.len();
                FormOutcome::Stay
            }
            KeyCode::BackTab => {
                let n = form.fields.len();
                form.focus = (form.focus + n - 1) % n;
                FormOutcome::Stay
            }
            KeyCode::Backspace => {
                form.fields[form.focus].pop();
                FormOutcome::Stay
            }
            KeyCode::Enter => {
                if form.focus + 1 < form.fields.len() {
                    form.focus += 1;
                    FormOutcome::Stay
                } else {
                    FormOutcome::TryCommit
                }
            }
            KeyCode::Char(c) => {
                form.fields[form.focus].push(c);
                FormOutcome::Stay
            }
            _ => FormOutcome::Stay,
        }
    };

    // 2. 持有的 &mut state/&mut form 已离开作用域，可以自由 mutate app。
    match outcome {
        FormOutcome::Stay => {}
        FormOutcome::Close => {
            if let Mode::Providers(state) = &mut app.mode {
                state.form = None;
            }
        }
        FormOutcome::TryCommit => commit_active_form(app),
    }
}

/// 把当前 form 序列化并落盘；成功则关闭 form 并发 toast。
fn commit_active_form(app: &mut App) {
    // 在不持有可变借用的前提下序列化 form。
    let serialized = match &app.mode {
        Mode::Providers(s) => s.form.as_deref().map(ProviderForm::to_toml_section),
        _ => None,
    };
    let Some(result) = serialized else { return };

    match result {
        Ok((id, body)) => {
            if let Err(e) = write_user_provider(&app.paths, &id, &body) {
                app.set_toast(Toast::error(format!("save failed: {e}")));
                return;
            }
            if let Err(e) = app.reload_providers() {
                app.set_toast(Toast::error(format!("save failed: {e}")));
                return;
            }
            if let Mode::Providers(s) = &mut app.mode {
                s.form = None;
            }
            app.set_toast(Toast::success(format!("provider `{id}` saved")));
        }
        Err(e) => app.set_toast(Toast::error(format!("save failed: {e}"))),
    }
}

/// 把一个 user provider 段写回 providers.toml：先删旧段再 append 新段。
///
/// 使用按行扫描的 `strip_section`（而非 toml 重序列化），以保留用户手写注释。
///
/// `pub(crate)` 起：wizard.rs 的 add-provider 子流程会绕过本面板的 `commit_active_form`
/// 直接调用此函数把新建段落落盘，然后驱动 `App::reload_providers` 同步内存视图。
pub(crate) fn write_user_provider(
    paths: &crate::paths::Paths,
    id: &str,
    body: &str,
) -> Result<(), Error> {
    use std::fs;

    if let Some(parent) = paths.providers().parent() {
        fs::create_dir_all(parent).map_err(|e| Error::Io {
            path: parent.to_path_buf(),
            source: e,
        })?;
    }

    let mut existing = if paths.providers().exists() {
        fs::read_to_string(paths.providers()).map_err(|e| Error::Io {
            path: paths.providers(),
            source: e,
        })?
    } else {
        String::new()
    };

    existing = strip_section(&existing, id);
    if !existing.is_empty() && !existing.ends_with('\n') {
        existing.push('\n');
    }
    existing.push_str(&format!("[{id}]\n{body}\n"));

    fs::write(paths.providers(), existing).map_err(|e| Error::Io {
        path: paths.providers(),
        source: e,
    })
}

fn remove_user_provider(app: &mut App, id: &str) -> Result<(), Error> {
    use std::fs;

    if !app.paths.providers().exists() {
        return Ok(());
    }
    let s = fs::read_to_string(app.paths.providers()).map_err(|e| Error::Io {
        path: app.paths.providers(),
        source: e,
    })?;
    let stripped = strip_section(&s, id);
    fs::write(app.paths.providers(), stripped).map_err(|e| Error::Io {
        path: app.paths.providers(),
        source: e,
    })?;
    app.reload_providers()?;
    Ok(())
}

/// 简陋 toml 段移除：按行扫描，跳过 `[id]` 直到下一个 `[` 或 EOF。
///
/// 不依赖 toml 库重新序列化（避免破坏用户手写注释；本 V1 决策）。
/// 注意：`line.trim()` 比较 head，对前后空白宽容；section 内的注释行/空行
/// 在 skipping=true 下一律跳过，直到再遇到行首为 `[` 的行。
fn strip_section(input: &str, id: &str) -> String {
    let head = format!("[{id}]");
    let mut out = String::new();
    let mut skipping = false;
    for line in input.lines() {
        if line.trim() == head {
            skipping = true;
            continue;
        }
        if skipping && line.trim_start().starts_with('[') {
            skipping = false;
        }
        if !skipping {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

pub fn draw(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let Mode::Providers(state) = &app.mode else {
        return;
    };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);
    frame.render_widget(
        Paragraph::new("Providers (p) — [a] add  [e] edit  [x] delete  [Esc] back"),
        chunks[0],
    );

    if let Some(form) = state.form.as_deref() {
        draw_form(frame, chunks[1], form);
    } else if let Some(id) = &state.confirm_delete {
        crate::tui::widgets::draw_confirm(
            frame,
            chunks[1],
            "Delete provider",
            &format!("将删除 `{id}` 段。同名 profile 可能因此无法启动。继续？"),
        );
    } else {
        draw_list(frame, chunks[1], app, state);
    }
    frame.render_widget(Paragraph::new("[Esc] back"), chunks[2]);
}

fn draw_list(frame: &mut Frame<'_>, area: Rect, app: &App, state: &State) {
    let builtin_ids: BTreeSet<String> = catalog::builtins().into_iter().map(|p| p.id).collect();
    let mut items: Vec<ListItem<'_>> = app
        .providers
        .iter()
        .map(|p| {
            let badge = if builtin_ids.contains(&p.id) {
                "[builtin]"
            } else {
                "[user]"
            };
            let auth = match p.auth {
                AuthScheme::Bearer => "Bearer",
                AuthScheme::XApiKey => "XApiKey",
            };
            let url = p.anthropic_base_url.as_deref().unwrap_or("(none)");
            ListItem::new(format!("{badge} {} — auth={auth} url={url}", p.id))
        })
        .collect();
    items.push(ListItem::new("+ 添加新 provider..."));
    let mut ls = state.list.clone();
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Providers"))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(list, area, &mut ls);
}

fn draw_form(frame: &mut Frame<'_>, area: Rect, form: &ProviderForm) {
    // 注意：InputField::render 返回 Paragraph，但表单想要把多个字段渲染成 Paragraph
    // 的多行；因此这里复刻 InputField 的渲染逻辑直接产出 Line。
    let lines: Vec<Line<'_>> = form
        .fields
        .iter()
        .enumerate()
        .map(|(i, field)| field_line(field, i == form.focus))
        .collect();
    let title = if form.editing_id.is_some() {
        "Edit user provider"
    } else {
        "Add user provider"
    };
    let p = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title.to_string()),
        )
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::White));
    frame.render_widget(p, area);
}

/// 给跨模块复用的薄包装：wizard.rs 的 add-provider 子流程把整块 area 让给
/// `ProviderForm`，直接调用本函数即可拿到与 Providers 面板内联表单一致的外观。
pub(crate) fn draw_form_external(frame: &mut Frame<'_>, area: Rect, form: &ProviderForm) {
    draw_form(frame, area, form);
}

fn field_line<'a>(field: &'a InputField, focused: bool) -> Line<'a> {
    let display = if field.mask {
        "*".repeat(field.buffer.chars().count())
    } else {
        field.buffer.clone()
    };
    let cursor = if focused { "_" } else { "" };
    Line::from(vec![
        Span::styled(
            format!("{}: ", field.label),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(display),
        Span::styled(cursor, Style::default().fg(Color::Yellow)),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_removes_only_target_section() {
        let s = "[a]\nx = 1\n\n[b]\ny = 2\n\n[c]\nz = 3\n";
        let after = strip_section(s, "b");
        assert!(after.contains("[a]"));
        assert!(!after.contains("[b]"));
        assert!(after.contains("[c]"));
    }

    #[test]
    fn form_to_toml_section_emits_present_fields() {
        let mut form = ProviderForm::new_add();
        form.fields[0] = InputField::new("id").with_initial("my-relay");
        form.fields[2] = InputField::new("anthropic_base_url")
            .with_initial("https://relay.example.com/anthropic");
        let (id, body) = form.to_toml_section().unwrap();
        assert_eq!(id, "my-relay");
        assert!(body.contains("anthropic_base_url"));
        assert!(body.contains("auth = \"Bearer\""));
        assert!(body.contains("models_endpoint_path = \"/v1/models\""));
    }

    #[test]
    fn form_rejects_empty_id() {
        let form = ProviderForm::new_add();
        assert!(form.to_toml_section().is_err());
    }
}
