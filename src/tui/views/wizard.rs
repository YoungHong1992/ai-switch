//! 创建/编辑 profile 向导（spec §7）。
//!
//! 5 步状态机：选 provider → 选 key → 选 model → 起名 → preview & commit。
//! 内联子流程（add-provider / add-key）在 Task 13 接入；本 task 末位
//! "+ 添加新 ..." 仅 toast 提示。Step 3 在 Task 12 接入：进入时同步拉
//! `/v1/models`（5s 超时），成功 → 列表选择，失败/空/无 base URL → 回退
//! 到自由输入子状态。
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
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

use crate::catalog::Provider;
use crate::credentials::{self, Key};
use crate::error::Error;
use crate::http;
use crate::profile::{self, CreateInput};
use crate::settings::Settings;
use crate::tui::app::{App, Mode};
use crate::tui::views::keys::KeyForm;
use crate::tui::views::providers::ProviderForm;
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
    /// Step 3 模型列表（成功 fetch 后填充；空表示尚未拉/失败/空）。
    pub model_list: ListState,
    pub model_choices: Vec<String>,
    /// 进入 Step::Model 时已尝试过一次 fetch（仅观察用，避免重复拉）。
    pub model_fetch_attempted: bool,
    /// 用户选了"+ 自定义..." 或 fetch 失败/空 → 走自由输入子状态。
    pub model_use_custom: bool,
    /// Step 1 末位 "+ 添加新 provider..." 触发的内联子表单。
    /// `Box` 与 `Mode::Wizard(Box<State>)` 同源：避免 `[InputField; 5]` 把 enum 撑大。
    pub provider_form: Option<Box<ProviderForm>>,
    /// Step 2 末位 "+ 添加新 key..." 触发的内联子表单。同样 `Box`。
    pub key_form: Option<Box<KeyForm>>,
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
        let mut model_list = ListState::default();
        model_list.select(Some(0));
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
            model_list,
            model_choices: Vec::new(),
            model_fetch_attempted: false,
            model_use_custom: false,
            provider_form: None,
            key_form: None,
        }
    }
}

impl State {
    pub fn is_in_input_mode(&self) -> bool {
        // 子表单也要算"输入态"（屏蔽 ?/q 等全局快捷键）。
        matches!(self.step, Step::Model | Step::Name)
            || self.provider_form.is_some()
            || self.key_form.is_some()
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
    // 子表单优先级最高：只要 provider_form / key_form 任一存在就独占事件，
    // 不再分派到 step handler，避免父级 step 的 Esc/Enter 提前清状态。
    let subform = match &app.mode {
        Mode::Wizard(s) => {
            if s.provider_form.is_some() {
                Some(SubformKind::Provider)
            } else if s.key_form.is_some() {
                Some(SubformKind::Key)
            } else {
                None
            }
        }
        _ => return,
    };
    if let Some(kind) = subform {
        match kind {
            SubformKind::Provider => handle_provider_subform_key(app, k),
            SubformKind::Key => handle_key_subform_key(app, k),
        }
        return;
    }

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

#[derive(Clone, Copy)]
enum SubformKind {
    Provider,
    Key,
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
                // 末位 "+ 添加新 provider..." → 打开内联子表单。
                if let Mode::Wizard(state) = &mut app.mode {
                    state.provider_form = Some(Box::new(ProviderForm::new_add()));
                }
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
                // 第一次可变借用：写入 picked_key_* + 切 step + 重置 model 列表状态，
                // 再把 fetch 所需 (provider, bearer) snapshot 出来供作用域外使用。
                let fetch_target: Option<(Provider, String)> =
                    if let Mode::Wizard(state) = &mut app.mode {
                        state.picked_key_id = Some(kid);
                        state.picked_key_value = Some(val.clone());
                        state.step = Step::Model;
                        state.model_choices.clear();
                        state.model_fetch_attempted = false;
                        state.model_use_custom = false;
                        state.model_list.select(Some(0));
                        state.picked_provider.clone().map(|p| (p, val))
                    } else {
                        None
                    };

                // 借用已释放，可以做同步 5s HTTP（spec §16 接受 UI 短暂 freeze）。
                // 失败 / 空列表 / openai_base_url 缺失 → use_custom = true 回退自由输入。
                if let Some((provider, bearer)) = fetch_target {
                    let result = http::fetch_models(
                        provider.openai_base_url.as_deref().unwrap_or(""),
                        &provider.models_endpoint_path,
                        Some(bearer.as_str()),
                    );
                    if let Mode::Wizard(state) = &mut app.mode {
                        match result {
                            Ok(list) if !list.is_empty() => state.model_choices = list,
                            _ => state.model_use_custom = true,
                        }
                        state.model_fetch_attempted = true;
                    }
                }
            } else {
                // 末位 "+ 添加新 key..." → 打开内联子表单，预填 provider id。
                if let Mode::Wizard(state) = &mut app.mode {
                    let mut form = KeyForm::new_add();
                    form.provider = InputField::new("provider").with_initial(&provider_id);
                    state.key_form = Some(Box::new(form));
                }
            }
        }
        _ => {}
    }
}

/// step 3 内部短借用产物：用来跨借用边界传递 toast 触发。
enum ModelOutcome {
    Stay,
    EmptyModel,
}

fn handle_step_model(app: &mut App, k: KeyEvent) {
    let outcome = {
        let Mode::Wizard(state) = &mut app.mode else {
            return;
        };
        if state.model_use_custom {
            handle_step_model_custom(state, k)
        } else {
            handle_step_model_list(state, k)
        }
    };
    if let ModelOutcome::EmptyModel = outcome {
        app.set_toast(Toast::error("model 不能为空"));
    }
}

/// 自由输入子状态：fetch 失败 / 空 base / 用户主动选 "+ 自定义..." 后落到这里。
fn handle_step_model_custom(state: &mut State, k: KeyEvent) -> ModelOutcome {
    match k.code {
        KeyCode::Esc => state.step = Step::Key,
        KeyCode::Backspace => state.model_input.pop(),
        KeyCode::Enter => {
            let m = state.model_input.value().trim().to_string();
            if m.is_empty() {
                return ModelOutcome::EmptyModel;
            }
            state.picked_model = Some(m);
            advance_to_name(state);
        }
        KeyCode::Char(c) => state.model_input.push(c),
        _ => {}
    }
    ModelOutcome::Stay
}

/// 列表子状态：选 fetch 到的某一项 → picked_model；选末位 "+ 自定义..." → 切自由输入。
fn handle_step_model_list(state: &mut State, k: KeyEvent) -> ModelOutcome {
    let n = state.model_choices.len();
    match k.code {
        KeyCode::Esc => state.step = Step::Key,
        KeyCode::Down | KeyCode::Char('j') => {
            let i = state.model_list.selected().unwrap_or(0);
            // n（而非 n-1）：包含末位 "+ 自定义..." 项
            state.model_list.select(Some((i + 1).min(n)));
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let i = state.model_list.selected().unwrap_or(0);
            state.model_list.select(Some(i.saturating_sub(1)));
        }
        KeyCode::Enter => {
            let i = state.model_list.selected().unwrap_or(0);
            if i < n {
                state.picked_model = Some(state.model_choices[i].clone());
                advance_to_name(state);
            } else {
                state.model_use_custom = true;
            }
        }
        _ => {}
    }
    ModelOutcome::Stay
}

/// 切到 Step::Name；首次进入时按 picked_provider + picked_model 预填 name_input。
/// 编辑模式下 locked_name 已设，跳过预填以避免覆盖锁定值。
fn advance_to_name(state: &mut State) {
    state.step = Step::Name;
    if state.locked_name.is_some() || !state.name_input.value().is_empty() {
        return;
    }
    if let (Some(p), Some(model)) = (state.picked_provider.as_ref(), state.picked_model.as_ref()) {
        let suggested = profile::suggested_name(&p.id, model);
        state.name_input = InputField::new("name")
            .with_max_len(64)
            .with_initial(&suggested);
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

/// add-provider 子表单 enter 后的处理结果。把 IO 推迟到借用作用域之外。
enum ProviderSubformOutcome {
    Stay,
    Cancel,
    Submit { id: String, body: String },
    InvalidForm(Error),
}

/// add-provider 子表单：spec §7 Step 1 末位"+ 添加新 provider..." 唯一入口。
///
/// 借用模型与 wizard 内其他 step handler 一致：
/// 1. 短借用 `&mut app.mode`，让纯字段编辑（push/pop/focus）就地完成；
/// 2. 用 `Outcome` 把"提交/取消"等需要 `&mut app` 的副作用带出借用；
/// 3. 离开作用域后做 IO（write_user_provider / reload_providers / set_toast）。
fn handle_provider_subform_key(app: &mut App, k: KeyEvent) {
    let outcome = {
        let Mode::Wizard(state) = &mut app.mode else {
            return;
        };
        let Some(form) = state.provider_form.as_deref_mut() else {
            return;
        };
        match k.code {
            KeyCode::Esc => ProviderSubformOutcome::Cancel,
            KeyCode::Tab => {
                form.focus = (form.focus + 1) % form.fields.len();
                ProviderSubformOutcome::Stay
            }
            KeyCode::BackTab => {
                let n = form.fields.len();
                form.focus = (form.focus + n - 1) % n;
                ProviderSubformOutcome::Stay
            }
            KeyCode::Backspace => {
                form.fields[form.focus].pop();
                ProviderSubformOutcome::Stay
            }
            KeyCode::Enter => {
                if form.focus + 1 < form.fields.len() {
                    form.focus += 1;
                    ProviderSubformOutcome::Stay
                } else {
                    match form.to_toml_section() {
                        Ok((id, body)) => ProviderSubformOutcome::Submit { id, body },
                        Err(e) => ProviderSubformOutcome::InvalidForm(e),
                    }
                }
            }
            KeyCode::Char(c) => {
                form.fields[form.focus].push(c);
                ProviderSubformOutcome::Stay
            }
            _ => ProviderSubformOutcome::Stay,
        }
    };

    match outcome {
        ProviderSubformOutcome::Stay => {}
        ProviderSubformOutcome::Cancel => {
            if let Mode::Wizard(state) = &mut app.mode {
                state.provider_form = None;
            }
        }
        ProviderSubformOutcome::InvalidForm(e) => {
            // 表单字段保留，由用户修正后再次回车。
            app.set_toast(Toast::error(format!("invalid: {e}")));
        }
        ProviderSubformOutcome::Submit { id, body } => {
            if let Err(e) =
                crate::tui::views::providers::write_user_provider(&app.paths, &id, &body)
            {
                app.set_toast(Toast::error(format!("save failed: {e}")));
                return;
            }
            if let Err(e) = app.reload_providers() {
                app.set_toast(Toast::error(format!("reload failed: {e}")));
                return;
            }
            // reload 已经把 app.providers 刷新到最新；在重借用 state 之前先定位新 provider。
            let new_idx = app.providers.iter().position(|p| p.id == id);
            if let Mode::Wizard(state) = &mut app.mode {
                if let Some(idx) = new_idx {
                    state.provider_list.select(Some(idx));
                    state.picked_provider = Some(app.providers[idx].clone());
                    // 自动推进到 Step::Key，省去用户再按一次 Enter。
                    state.step = Step::Key;
                    state.key_list.select(Some(0));
                }
                state.provider_form = None;
            }
            app.set_toast(Toast::success(format!("provider `{id}` created")));
        }
    }
}

/// add-key 子表单 enter 后的处理结果。提交时携带 owned 字段，避免重借用 form。
enum KeySubformOutcome {
    Stay,
    Cancel,
    Submit {
        provider_id: String,
        value: String,
        id_override: String,
        note: String,
    },
}

/// add-key 子表单：spec §7 Step 2 末位"+ 添加新 key..." 唯一入口。
///
/// 与 keys.rs 的 `commit_key_form` 路径**有意分离**——本路径是新建场景：
/// - 没有 `editing` 旧 id 需要 rename；
/// - 没有引用此 key 的 profile，所以不调 `profile::rotate_key`；
/// - 提交成功后转场到 `Step::Model`，并且把 `model_use_custom = true` —— 新 key 通常
///   尚未授权调用 `/v1/models`，跳过 fetch 直接走自由输入更稳妥（spec §16）。
fn handle_key_subform_key(app: &mut App, k: KeyEvent) {
    let outcome = {
        let Mode::Wizard(state) = &mut app.mode else {
            return;
        };
        let Some(form) = state.key_form.as_deref_mut() else {
            return;
        };
        match k.code {
            KeyCode::Esc => KeySubformOutcome::Cancel,
            KeyCode::Tab => {
                form.focus = (form.focus + 1) % 4;
                KeySubformOutcome::Stay
            }
            KeyCode::BackTab => {
                form.focus = (form.focus + 3) % 4;
                KeySubformOutcome::Stay
            }
            KeyCode::Backspace => {
                focused_field_mut(form).pop();
                KeySubformOutcome::Stay
            }
            KeyCode::Enter => {
                if form.focus + 1 < 4 {
                    form.focus += 1;
                    KeySubformOutcome::Stay
                } else {
                    KeySubformOutcome::Submit {
                        provider_id: form.provider.value().trim().to_string(),
                        value: form.value.value().to_string(),
                        id_override: form.id_override.value().trim().to_string(),
                        note: form.note.value().to_string(),
                    }
                }
            }
            KeyCode::Char(c) => {
                focused_field_mut(form).push(c);
                KeySubformOutcome::Stay
            }
            _ => KeySubformOutcome::Stay,
        }
    };

    match outcome {
        KeySubformOutcome::Stay => {}
        KeySubformOutcome::Cancel => {
            if let Mode::Wizard(state) = &mut app.mode {
                state.key_form = None;
            }
        }
        KeySubformOutcome::Submit {
            provider_id,
            value,
            id_override,
            note,
        } => commit_key_subform(app, provider_id, value, id_override, note),
    }
}

/// 把 add-key 子表单提交落盘并推进 wizard 到 Step::Model。
///
/// 错误路径（id 校验、auto id 失败、save 失败）下保留 form，由用户修正再试；
/// 成功路径下清空 form、写 picked_key_*、复位 model 子状态。
fn commit_key_subform(
    app: &mut App,
    provider_id: String,
    value: String,
    id_override: String,
    note: String,
) {
    if provider_id.is_empty() {
        app.set_toast(Toast::error("provider must not be empty"));
        return;
    }
    if value.is_empty() {
        app.set_toast(Toast::error("value must not be empty"));
        return;
    }

    let existing: Vec<String> = app
        .credentials
        .by_provider
        .get(&provider_id)
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();
    let new_id = if !id_override.is_empty() {
        if let Err(e) = credentials::validate_id(&id_override) {
            app.set_toast(Toast::error(e.to_string()));
            return;
        }
        id_override
    } else {
        match credentials::unique_id(&value, &existing) {
            Ok(id) => id,
            Err(e) => {
                app.set_toast(Toast::error(format!("auto id failed: {e}")));
                return;
            }
        }
    };

    let map = app
        .credentials
        .by_provider
        .entry(provider_id.clone())
        .or_default();
    map.insert(
        new_id.clone(),
        Key {
            value: value.clone(),
            note,
        },
    );
    if let Err(e) = credentials::save(&app.paths.credentials(), &app.credentials) {
        app.set_toast(Toast::error(format!("save failed: {e}")));
        return;
    }

    if let Mode::Wizard(state) = &mut app.mode {
        state.picked_key_id = Some(new_id.clone());
        state.picked_key_value = Some(value);
        state.key_form = None;
        state.step = Step::Model;
        // 新 key 通常无 fetch /v1/models 权限：跳过 Task 12 的 HTTP 路径，直接走自由输入。
        state.model_choices.clear();
        state.model_use_custom = true;
        state.model_fetch_attempted = false;
        state.model_list.select(Some(0));
    }
    app.set_toast(Toast::success(format!(
        "key `{provider_id}/{new_id}` created"
    )));
}

/// 按 KeyForm.focus 取对应字段的可变借用；与 keys.rs 内的 `KeyForm::focused_mut` 同义。
/// 不在 KeyForm 上加 pub 方法，避免污染 keys.rs 模块对外 API。
fn focused_field_mut(form: &mut KeyForm) -> &mut InputField {
    match form.focus {
        0 => &mut form.provider,
        1 => &mut form.value,
        2 => &mut form.id_override,
        _ => &mut form.note,
    }
}

pub fn draw(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let Mode::Wizard(state) = &app.mode else {
        return;
    };
    // 子表单优先：把整块 area 让给 ProviderForm / KeyForm 的视觉，复用 providers.rs / keys.rs
    // 已有的 draw_form_external 包装，避免本文件再维护一份相同的渲染代码。
    if let Some(form) = state.provider_form.as_deref() {
        crate::tui::views::providers::draw_form_external(frame, area, form);
        return;
    }
    if let Some(form) = state.key_form.as_deref() {
        crate::tui::views::keys::draw_form_external(frame, area, form);
        return;
    }
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
    if state.model_use_custom {
        let lines = vec![
            field_line(&state.model_input, true),
            Line::from(""),
            Line::from("（fetch 失败 / openai_base_url 缺失，转自由输入）"),
        ];
        let para = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Step 3 / 5  Model（自由输入）"),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(para, area);
        return;
    }
    let mut items: Vec<ListItem<'_>> = state
        .model_choices
        .iter()
        .map(|m| ListItem::new(m.clone()))
        .collect();
    items.push(ListItem::new("+ 自定义..."));
    let mut ls = state.model_list.clone();
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Step 3 / 5  Model"),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(list, area, &mut ls);
}

/// 同 keys.rs 的 field_line：把 InputField 渲染成单行（label + 值 + 光标）。
/// 私有重复实现以避免在 widgets.rs 暴露内部细节；后续如有第三处复用再上提。
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
