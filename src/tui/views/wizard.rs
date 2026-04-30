//! 创建/编辑 profile 向导（spec §7）。
//!
//! 5 步状态机：选 provider → 选 key → 选 model → 起名 → preview & commit。
//! 内联子流程（add-provider / add-key）在 Task 13 接入；本 task 末位
//! "+ 添加新 ..." 仅 toast 提示。模型拉取在 Task 12 接入；本 task step 3
//! 简化为自由输入 InputField。
//!
//! 借用形态（与 providers.rs / keys.rs 一致）：
//! - `handle_key` 先读 `app.mode` 拿到当前 step，再分派到对应 handler。
//! - 各 handler 内只在最小作用域里 `if let Mode::Wizard(state) = &mut app.mode`
//!   持有可变借用：先把后续 IO / `app.set_toast` 需要的数据拷贝出来，
//!   再退出借用作用域执行写操作，避免与 `&mut app` 冲突。
//! - 不把整个 `Mode::Wizard(...)` 内容拍到栈上：State 含 `Provider` + 2 张
//!   `ListState` + 2 张 `InputField`，整体 ~400 字节，会触发
//!   `clippy::large_enum_variant`，因此 `Mode::Wizard` 持有 `Box<State>`
//!   （仅 8 字节）。`Box` 的 `Deref` 让 view 内部代码无感。

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

use crate::catalog::Provider;
use crate::error::Error;
use crate::profile::{self, CreateInput};
use crate::settings::Settings;
use crate::tui::app::{App, Mode};
use crate::tui::widgets::{InputField, Toast};

/// 向导整体状态。
///
/// 字段都是 `pub`：Task 12 / 13 / 14 会继续往里加（`model_list`、
/// `provider_form`、`key_form` 等），所以现在不锁字段集，仅承诺当前已知字段。
#[derive(Debug)]
pub struct State {
    pub step: Step,
    /// 编辑模式下锁住 name；为 `Some` 时 Step::Name 不接受输入。
    pub locked_name: Option<String>,
    pub picked_provider: Option<Provider>,
    pub picked_key_id: Option<String>,
    pub picked_key_value: Option<String>,
    pub picked_model: Option<String>,
    pub provider_list: ListState,
    pub key_list: ListState,
    pub model_input: InputField,
    pub name_input: InputField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Provider,
    Key,
    Model,
    Name,
    Preview,
}

impl Default for State {
    fn default() -> Self {
        let mut provider_list = ListState::default();
        provider_list.select(Some(0));
        let mut key_list = ListState::default();
        key_list.select(Some(0));
        Self {
            step: Step::Provider,
            locked_name: None,
            picked_provider: None,
            picked_key_id: None,
            picked_key_value: None,
            picked_model: None,
            provider_list,
            key_list,
            model_input: InputField::new("model").with_max_len(128),
            name_input: InputField::new("name").with_max_len(64),
        }
    }
}

impl State {
    pub fn is_in_input_mode(&self) -> bool {
        matches!(self.step, Step::Model | Step::Name)
    }

    /// 进入编辑流：从 `app.index` + `app.credentials` 把 entry 完整反向填回，
    /// 锁住 name。Step 仍从 Provider 起步，方便用户复核每一步选项。
    pub fn for_edit(name: &str, app: &App) -> Self {
        let mut s = Self {
            locked_name: Some(name.into()),
            ..Self::default()
        };

        let Some(entry) = app.index.entries.get(name) else {
            return s;
        };

        if let Some((idx, p)) = app
            .providers
            .iter()
            .enumerate()
            .find(|(_, p)| p.id == entry.provider)
        {
            s.provider_list.select(Some(idx));
            s.picked_provider = Some(p.clone());
        }
        s.picked_key_id = Some(entry.key_id.clone());

        // 关键：从 credentials 把 key value 也带上，否则 commit 时会把 settings.json
        // 的 ANTHROPIC_API_KEY 写成空串。
        if let Some(map) = app.credentials.by_provider.get(&entry.provider)
            && let Some(k) = map.get(&entry.key_id)
        {
            s.picked_key_value = Some(k.value.clone());
        }

        s.picked_model = Some(entry.model.clone());
        s.model_input = InputField::new("model")
            .with_max_len(128)
            .with_initial(&entry.model);
        s.name_input = InputField::new("name").with_max_len(64).with_initial(name);
        s
    }
}

pub fn handle_key(app: &mut App, k: KeyEvent) {
    let step = match &app.mode {
        Mode::Wizard(s) => s.step,
        _ => return,
    };
    match step {
        Step::Provider => handle_step_provider(app, k),
        Step::Key => handle_step_key(app, k),
        Step::Model => handle_step_model(app, k),
        Step::Name => handle_step_name(app, k),
        Step::Preview => handle_step_preview(app, k),
    }
}

fn back_to_profiles(app: &mut App) {
    app.mode = Mode::Profiles(crate::tui::views::profiles::State::new(&app.index));
}

fn handle_step_provider(app: &mut App, k: KeyEvent) {
    let providers_len = app.providers.len();
    // 列表总行数 = providers + 1 个末位 "+ 添加新 provider..."
    let total = providers_len + 1;

    match k.code {
        KeyCode::Esc => back_to_profiles(app),
        KeyCode::Down | KeyCode::Char('j') => {
            if let Mode::Wizard(state) = &mut app.mode {
                let i = state.provider_list.selected().unwrap_or(0);
                state
                    .provider_list
                    .select(Some((i + 1).min(total.saturating_sub(1))));
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if let Mode::Wizard(state) = &mut app.mode {
                let i = state.provider_list.selected().unwrap_or(0);
                state.provider_list.select(Some(i.saturating_sub(1)));
            }
        }
        KeyCode::Enter => {
            let selected = match &app.mode {
                Mode::Wizard(s) => s.provider_list.selected().unwrap_or(0),
                _ => return,
            };
            if selected < providers_len {
                let provider = app.providers[selected].clone();
                if let Mode::Wizard(state) = &mut app.mode {
                    state.picked_provider = Some(provider);
                    state.step = Step::Key;
                    state.key_list.select(Some(0));
                }
            } else {
                // 末位 "+ 添加新 provider..."：Task 13 接入子流程
                app.set_toast(Toast::info("add-provider 子流程将在 Task 13 接入"));
            }
        }
        _ => {}
    }
}

fn handle_step_key(app: &mut App, k: KeyEvent) {
    // 先在不持有可变借用的情况下抓出 picked_provider 的 id 与当前选中索引。
    let pre = match &app.mode {
        Mode::Wizard(s) => s
            .picked_provider
            .as_ref()
            .map(|p| (p.id.clone(), s.key_list.selected().unwrap_or(0))),
        _ => return,
    };
    let (provider_id, selected) = match pre {
        Some(t) => t,
        None => {
            // 没有 picked_provider（理论上不该到这里）；安全回退到 Step::Provider
            if let Mode::Wizard(state) = &mut app.mode {
                state.step = Step::Provider;
            }
            return;
        }
    };

    // 把当前 provider 下的 keys 整理出来（按 BTreeMap 字典序）。
    let keys: Vec<(String, String)> = app
        .credentials
        .by_provider
        .get(&provider_id)
        .map(|m| {
            m.iter()
                .map(|(id, k)| (id.clone(), k.value.clone()))
                .collect()
        })
        .unwrap_or_default();
    let total = keys.len() + 1; // + 末位 "+ 添加新 key..."

    match k.code {
        KeyCode::Esc => {
            if let Mode::Wizard(state) = &mut app.mode {
                state.step = Step::Provider;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Mode::Wizard(state) = &mut app.mode {
                let i = state.key_list.selected().unwrap_or(0);
                state
                    .key_list
                    .select(Some((i + 1).min(total.saturating_sub(1))));
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if let Mode::Wizard(state) = &mut app.mode {
                let i = state.key_list.selected().unwrap_or(0);
                state.key_list.select(Some(i.saturating_sub(1)));
            }
        }
        KeyCode::Enter => {
            if selected < keys.len() {
                let (kid, val) = keys[selected].clone();
                if let Mode::Wizard(state) = &mut app.mode {
                    state.picked_key_id = Some(kid);
                    state.picked_key_value = Some(val);
                    state.step = Step::Model;
                }
            } else {
                app.set_toast(Toast::info("add-key 子流程将在 Task 13 接入"));
            }
        }
        _ => {}
    }
}

/// step 3 内部短借用产物：用来跨借用边界传递动作。
enum ModelOutcome {
    Stay,
    Back,
    EmptyModel,
    Commit(String),
}

fn handle_step_model(app: &mut App, k: KeyEvent) {
    let outcome = {
        let Mode::Wizard(state) = &mut app.mode else {
            return;
        };
        match k.code {
            KeyCode::Esc => ModelOutcome::Back,
            KeyCode::Backspace => {
                state.model_input.pop();
                ModelOutcome::Stay
            }
            KeyCode::Char(c) => {
                state.model_input.push(c);
                ModelOutcome::Stay
            }
            KeyCode::Enter => {
                let m = state.model_input.value().trim().to_string();
                if m.is_empty() {
                    ModelOutcome::EmptyModel
                } else {
                    ModelOutcome::Commit(m)
                }
            }
            _ => ModelOutcome::Stay,
        }
    };

    match outcome {
        ModelOutcome::Stay => {}
        ModelOutcome::Back => {
            if let Mode::Wizard(state) = &mut app.mode {
                state.step = Step::Key;
            }
        }
        ModelOutcome::EmptyModel => app.set_toast(Toast::error("model 不能为空")),
        ModelOutcome::Commit(model) => {
            if let Mode::Wizard(state) = &mut app.mode {
                state.picked_model = Some(model.clone());
                state.step = Step::Name;
                // 仅当 name 既没锁定也没有任何已填值时，按 suggested_name 预填。
                if state.locked_name.is_none()
                    && state.name_input.value().is_empty()
                    && let Some(p) = state.picked_provider.as_ref()
                {
                    let suggested = profile::suggested_name(&p.id, &model);
                    state.name_input = InputField::new("name")
                        .with_max_len(64)
                        .with_initial(&suggested);
                }
            }
        }
    }
}

/// step 4 内部短借用产物。
enum NameOutcome {
    Stay,
    Back,
    SubmitCandidate(String),
    ValidateError(Error),
}

fn handle_step_name(app: &mut App, k: KeyEvent) {
    // 编辑模式：name 已锁定，仅 Esc/Enter 切 step。
    let locked = matches!(&app.mode, Mode::Wizard(s) if s.locked_name.is_some());
    if locked {
        match k.code {
            KeyCode::Esc => {
                if let Mode::Wizard(state) = &mut app.mode {
                    state.step = Step::Model;
                }
            }
            KeyCode::Enter => {
                if let Mode::Wizard(state) = &mut app.mode {
                    state.step = Step::Preview;
                }
            }
            _ => {}
        }
        return;
    }

    let outcome = {
        let Mode::Wizard(state) = &mut app.mode else {
            return;
        };
        match k.code {
            KeyCode::Esc => NameOutcome::Back,
            KeyCode::Backspace => {
                state.name_input.pop();
                NameOutcome::Stay
            }
            KeyCode::Char(c) => {
                state.name_input.push(c);
                NameOutcome::Stay
            }
            KeyCode::Enter => {
                let name = state.name_input.value().trim().to_string();
                match profile::validate_name(&name) {
                    Ok(()) => NameOutcome::SubmitCandidate(name),
                    Err(e) => NameOutcome::ValidateError(e),
                }
            }
            _ => NameOutcome::Stay,
        }
    };

    match outcome {
        NameOutcome::Stay => {}
        NameOutcome::Back => {
            if let Mode::Wizard(state) = &mut app.mode {
                state.step = Step::Model;
            }
        }
        NameOutcome::ValidateError(e) => app.set_toast(Toast::error(e.to_string())),
        NameOutcome::SubmitCandidate(name) => {
            if app.index.entries.contains_key(&name) {
                // 重名 → 用 suggested_name_with_key 自动换建议
                let suggestion = match &app.mode {
                    Mode::Wizard(s) => match (
                        s.picked_provider.as_ref(),
                        s.picked_model.as_ref(),
                        s.picked_key_id.as_ref(),
                    ) {
                        (Some(p), Some(m), Some(kid)) => {
                            Some(profile::suggested_name_with_key(&p.id, m, kid))
                        }
                        _ => None,
                    },
                    _ => None,
                };
                if let Some(s) = suggestion {
                    if let Mode::Wizard(state) = &mut app.mode {
                        state.name_input =
                            InputField::new("name").with_max_len(64).with_initial(&s);
                    }
                    app.set_toast(Toast::info("name 重复，已自动换建议"));
                } else {
                    app.set_toast(Toast::error("name 重复且无法生成建议"));
                }
            } else if let Mode::Wizard(state) = &mut app.mode {
                state.step = Step::Preview;
            }
        }
    }
}

fn handle_step_preview(app: &mut App, k: KeyEvent) {
    match k.code {
        KeyCode::Esc => {
            if let Mode::Wizard(state) = &mut app.mode {
                state.step = Step::Name;
            }
        }
        KeyCode::Enter => match commit_wizard(app) {
            Ok(name) => {
                app.set_toast(Toast::success(format!("profile `{name}` saved")));
                back_to_profiles(app);
            }
            Err(e) => app.set_toast(Toast::error(format!("commit failed: {e}"))),
        },
        _ => {}
    }
}

/// 提交快照：把当前 wizard state 序列化为独立 owned 字段，离开 `app.mode`
/// 借用作用域后再做 IO，避免借用冲突。
struct CommitSnapshot {
    name: String,
    provider_id: String,
    anthropic_url: String,
    key_id: String,
    key_value: String,
    model: String,
}

fn commit_wizard(app: &mut App) -> Result<String, Error> {
    let snap = match &app.mode {
        Mode::Wizard(s) => snapshot_for_commit(s)?,
        _ => {
            return Err(Error::ProfileNotFound {
                name: "<wizard>".into(),
            });
        }
    };
    profile::create(
        &app.paths,
        CreateInput {
            name: &snap.name,
            provider_id: &snap.provider_id,
            key_id: &snap.key_id,
            model: &snap.model,
            anthropic_base_url: &snap.anthropic_url,
            api_key_value: &snap.key_value,
        },
    )?;
    app.reload_index()?;
    Ok(snap.name)
}

fn snapshot_for_commit(s: &State) -> Result<CommitSnapshot, Error> {
    let provider = s.picked_provider.clone().ok_or(Error::ProviderNotFound {
        id: "<unset>".into(),
    })?;
    let url = provider
        .anthropic_base_url
        .clone()
        .ok_or(Error::ProviderMissingAnthropicUrl {
            id: provider.id.clone(),
        })?;
    Ok(CommitSnapshot {
        name: s
            .locked_name
            .clone()
            .unwrap_or_else(|| s.name_input.value().to_string()),
        provider_id: provider.id,
        anthropic_url: url,
        key_id: s.picked_key_id.clone().unwrap_or_default(),
        key_value: s.picked_key_value.clone().unwrap_or_default(),
        model: s.picked_model.clone().unwrap_or_default(),
    })
}

pub fn draw(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let Mode::Wizard(state) = &app.mode else {
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
        Paragraph::new(format!("New/Edit profile — step: {:?}", state.step)),
        chunks[0],
    );
    match state.step {
        Step::Provider => draw_step_provider(frame, chunks[1], app, state),
        Step::Key => draw_step_key(frame, chunks[1], app, state),
        Step::Model => draw_step_model(frame, chunks[1], state),
        Step::Name => draw_step_name(frame, chunks[1], state),
        Step::Preview => draw_step_preview(frame, chunks[1], state),
    }
    frame.render_widget(Paragraph::new("[Enter] next  [Esc] back"), chunks[2]);
}

fn draw_step_provider(frame: &mut Frame<'_>, area: Rect, app: &App, state: &State) {
    let mut items: Vec<ListItem<'_>> = app
        .providers
        .iter()
        .map(|p| ListItem::new(format!("{} — {}", p.id, p.display_name)))
        .collect();
    items.push(ListItem::new("+ 添加新 provider..."));
    let mut ls = state.provider_list.clone();
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Step 1 / 5  Provider"),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(list, area, &mut ls);
}

fn draw_step_key(frame: &mut Frame<'_>, area: Rect, app: &App, state: &State) {
    let provider = match &state.picked_provider {
        Some(p) => p,
        None => return,
    };
    let mut items: Vec<ListItem<'_>> = app
        .credentials
        .by_provider
        .get(&provider.id)
        .map(|m| {
            m.iter()
                .map(|(id, k)| ListItem::new(format!("{} ({})", id, k.note)))
                .collect()
        })
        .unwrap_or_default();
    items.push(ListItem::new("+ 添加新 key..."));
    let mut ls = state.key_list.clone();
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Step 2 / 5  Key（{} 下）", provider.id)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(list, area, &mut ls);
}

fn draw_step_model(frame: &mut Frame<'_>, area: Rect, state: &State) {
    let p = state.model_input.render(true).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Step 3 / 5  Model（自由输入；Task 12 增强为自动拉取）"),
    );
    frame.render_widget(p, area);
}

fn draw_step_name(frame: &mut Frame<'_>, area: Rect, state: &State) {
    let title = if state.locked_name.is_some() {
        "Step 4 / 5  Name（编辑模式 — 已锁定）"
    } else {
        "Step 4 / 5  Name"
    };
    let p = state
        .name_input
        .render(true)
        .block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(p, area);
}

fn draw_step_preview(frame: &mut Frame<'_>, area: Rect, state: &State) {
    let provider = state.picked_provider.as_ref();
    let model = state.picked_model.as_deref().unwrap_or("");
    let key_value = state.picked_key_value.as_deref().unwrap_or("");
    let name = state
        .locked_name
        .clone()
        .unwrap_or_else(|| state.name_input.value().to_string());
    let url = provider
        .and_then(|p| p.anthropic_base_url.as_deref())
        .unwrap_or("");
    let preview_settings = Settings::render(url, key_value, model);
    let json = serde_json::to_string_pretty(&preview_settings).unwrap_or_default();
    let body = format!(
        "Will write: settings_{name}.json\n\nProvider: {}\nKey ID:   {}\nModel:    {}\n\n--- settings.json ---\n{json}\n",
        provider.map(|p| p.id.as_str()).unwrap_or(""),
        state.picked_key_id.as_deref().unwrap_or(""),
        model,
    );
    let p = Paragraph::new(body)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Step 5 / 5  Preview"),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(p, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog;

    #[test]
    fn state_default_starts_at_step_provider() {
        let s = State::default();
        assert_eq!(s.step, Step::Provider);
        assert!(s.locked_name.is_none());
    }

    #[test]
    fn picked_after_provider_advances_to_key() {
        // 模拟一次 provider step 推进
        let p = catalog::find("deepseek").unwrap();
        let s = State {
            picked_provider: Some(p),
            step: Step::Key,
            ..State::default()
        };
        assert_eq!(s.step, Step::Key);
        assert!(s.picked_provider.is_some());
    }

    #[test]
    fn for_edit_locks_name_and_prefills() {
        use crate::profile::IndexEntry;
        use chrono::Utc;
        let p = crate::paths::Paths::with_root(std::env::temp_dir().join("ais-edit-pf"));
        let mut app = App {
            paths: p,
            providers: vec![catalog::find("deepseek").unwrap()],
            credentials: Default::default(),
            index: Default::default(),
            mode: Mode::Wizard(Box::default()),
            toast: None,
            running: true,
            launch_target: None,
        };
        app.index.entries.insert(
            "x".into(),
            IndexEntry {
                provider: "deepseek".into(),
                key_id: "sk-a...fswv".into(),
                model: "deepseek-chat".into(),
                created_at: Utc::now(),
            },
        );
        let s = State::for_edit("x", &app);
        assert_eq!(s.locked_name.as_deref(), Some("x"));
        assert_eq!(s.picked_model.as_deref(), Some("deepseek-chat"));
        assert!(s.picked_provider.is_some());
    }
}
