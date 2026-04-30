//! Keys 面板：跨 provider 列出 credentials.toml 中所有 key；增加 / 改 value / 改 note / 删除。
//!
//! 关键交互（spec §6.4）：改 value 触发：
//! 1. 重算脱敏 id（可能与原 id 一致也可能不同）
//! 2. 重命名 credentials.toml section
//! 3. profile::rename_key_id_in_index 同步索引
//! 4. profile::rotate_key 重渲染所有引用该 key 的 settings.json
//!
//! 借用形态（与 Task 9 providers.rs 一致）：
//! - `Mode::Keys(State)` 中重表单用 `Option<Box<KeyForm>>` 包裹，避免
//!   `clippy::large_enum_variant` 把整个 `Mode` 拉胖。
//! - `handle_key` 只取 `&mut App`，先短借用读出 phase，再分派；handler 函数
//!   重新 `if let Mode::Keys(...) = &mut app.mode` 取状态，避免长时间持有
//!   `&mut app.mode` 与后续 `app.set_toast` / `credentials::save` 等 `&mut app`
//!   操作冲突。
//! - 提交流程在借用作用域内拷贝出 `KeyFormCommit` 快照，离开作用域后才做 IO。
//!
//! `flat` 不在 `State` 缓存：每次需要的时候用 `flatten(app)` 重算，确保
//! 写操作后第一帧自动看到最新数据，没有 stale-cache 问题。

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

use crate::credentials::{self, Key};
use crate::error::Error;
use crate::profile;
use crate::tui::app::{App, Mode};
use crate::tui::widgets::{InputField, Toast};

/// Keys 面板状态。
///
/// 子状态优先级（同一时刻只能有一个生效）：`form` > `confirm_delete` > 列表。
#[derive(Debug, Default)]
pub struct State {
    pub list: ListState,
    pub form: Option<Box<KeyForm>>,
    pub confirm_delete: Option<(String, String)>,
}

impl State {
    pub fn is_in_input_mode(&self) -> bool {
        self.form.is_some()
    }
}

/// 视图内的扁平化行（每个 (provider, key_id) 一个）。
#[derive(Debug, Clone)]
pub struct KeyRow {
    pub provider: String,
    pub key_id: String,
    pub note: String,
    pub value_redacted: String,
}

/// 增/改 key 的小表单。
///
/// 字段为 `pub`，方便 Task 13 wizard 的 add-key 子流程复用此 struct。
#[derive(Debug)]
pub struct KeyForm {
    /// `Some((provider, old_key_id))` 表示编辑既有；`None` 表示新增。
    pub editing: Option<(String, String)>,
    pub provider: InputField,
    pub value: InputField,
    pub id_override: InputField,
    pub note: InputField,
    pub focus: usize,
}

impl KeyForm {
    pub fn new_add() -> Self {
        Self {
            editing: None,
            provider: InputField::new("provider"),
            value: InputField::new("value").masked(),
            id_override: InputField::new("id (auto if empty when len>=12)"),
            note: InputField::new("note"),
            focus: 0,
        }
    }

    pub fn from_existing(provider: &str, key_id: &str, key: &Key) -> Self {
        Self {
            editing: Some((provider.into(), key_id.into())),
            provider: InputField::new("provider").with_initial(provider),
            value: InputField::new("value").masked().with_initial(&key.value),
            id_override: InputField::new("id (auto if empty when len>=12)").with_initial(key_id),
            note: InputField::new("note").with_initial(&key.note),
            // 编辑流默认聚焦 value：spec §6.4 的"改 value 联动 rotate"是主路径。
            focus: 1,
        }
    }

    fn focused_mut(&mut self) -> &mut InputField {
        match self.focus {
            0 => &mut self.provider,
            1 => &mut self.value,
            2 => &mut self.id_override,
            _ => &mut self.note,
        }
    }

    /// 把当前表单序列化成可独立持有的拷贝；离开 `&mut app.mode` 借用后用于提交。
    fn snapshot(&self) -> KeyFormCommit {
        KeyFormCommit {
            editing: self.editing.clone(),
            provider: self.provider.value().to_string(),
            value: self.value.value().to_string(),
            id_override: self.id_override.value().to_string(),
            note: self.note.value().to_string(),
        }
    }
}

/// 表单提交时的字段快照。携带它进入 `commit_key_form` 后即不再持有 `&mut app.mode`。
struct KeyFormCommit {
    editing: Option<(String, String)>,
    provider: String,
    value: String,
    id_override: String,
    note: String,
}

/// 当前面板交互阶段；`handle_key` 短借用读出后再分派。
#[derive(Clone, Copy)]
enum Phase {
    Form,
    Confirm,
    List,
}

/// 表单子状态执行结果，提取出来以释放 `&mut app.mode` 借用，再做后续 IO。
enum FormOutcome {
    Stay,
    Close,
    Commit(KeyFormCommit),
}

pub fn handle_key(app: &mut App, k: KeyEvent) {
    let phase = match &app.mode {
        Mode::Keys(s) => {
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
            let total = flatten(app).len();
            if let Mode::Keys(state) = &mut app.mode
                && total > 0
            {
                let i = state.list.selected().unwrap_or(0);
                state.list.select(Some((i + 1).min(total - 1)));
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let total = flatten(app).len();
            if let Mode::Keys(state) = &mut app.mode
                && total > 0
            {
                let i = state.list.selected().unwrap_or(0);
                state.list.select(Some(i.saturating_sub(1)));
            }
        }
        KeyCode::Char('a') | KeyCode::Char('+') => {
            if let Mode::Keys(state) = &mut app.mode {
                state.form = Some(Box::new(KeyForm::new_add()));
            }
        }
        KeyCode::Char('e') => match currently_selected_key(app) {
            Some((provider, key_id, key)) => {
                if let Mode::Keys(state) = &mut app.mode {
                    state.form = Some(Box::new(KeyForm::from_existing(&provider, &key_id, &key)));
                }
            }
            None => app.set_toast(Toast::info("no key selected")),
        },
        KeyCode::Char('x') => match currently_selected_key(app) {
            Some((provider, key_id, _)) => {
                if let Mode::Keys(state) = &mut app.mode {
                    state.confirm_delete = Some((provider, key_id));
                }
            }
            None => app.set_toast(Toast::info("no key selected")),
        },
        _ => {}
    }
}

/// 取当前选中的 (provider, key_id, Key)，全部克隆为 owned，便于离开借用后使用。
fn currently_selected_key(app: &App) -> Option<(String, String, Key)> {
    let Mode::Keys(state) = &app.mode else {
        return None;
    };
    let i = state.list.selected()?;
    let rows = flatten(app);
    let row = rows.get(i)?;
    let key = app
        .credentials
        .by_provider
        .get(&row.provider)
        .and_then(|m| m.get(&row.key_id))?;
    Some((row.provider.clone(), row.key_id.clone(), key.clone()))
}

fn handle_form_key(app: &mut App, k: KeyEvent) {
    // 1. 在短借用里完成所有纯字段级别的更新；返回 outcome 表示后续动作。
    let outcome = {
        let Mode::Keys(state) = &mut app.mode else {
            return;
        };
        let Some(form) = state.form.as_deref_mut() else {
            return;
        };

        match k.code {
            KeyCode::Esc => FormOutcome::Close,
            KeyCode::Tab => {
                form.focus = (form.focus + 1) % 4;
                FormOutcome::Stay
            }
            KeyCode::BackTab => {
                form.focus = (form.focus + 3) % 4;
                FormOutcome::Stay
            }
            KeyCode::Backspace => {
                form.focused_mut().pop();
                FormOutcome::Stay
            }
            KeyCode::Enter => {
                if form.focus + 1 < 4 {
                    form.focus += 1;
                    FormOutcome::Stay
                } else {
                    FormOutcome::Commit(form.snapshot())
                }
            }
            KeyCode::Char(c) => {
                form.focused_mut().push(c);
                FormOutcome::Stay
            }
            _ => FormOutcome::Stay,
        }
    };

    // 2. 借用已离开作用域，可以自由 mutate app（IO + toast）。
    match outcome {
        FormOutcome::Stay => {}
        FormOutcome::Close => {
            if let Mode::Keys(state) = &mut app.mode {
                state.form = None;
            }
        }
        FormOutcome::Commit(snap) => match commit_key_form(app, snap) {
            Ok((provider, kid)) => {
                app.set_toast(Toast::success(format!("key `{provider}/{kid}` saved")));
                if let Mode::Keys(state) = &mut app.mode {
                    state.form = None;
                }
                clamp_selection_to_current(app);
            }
            Err(e) => app.set_toast(Toast::error(format!("save failed: {e}"))),
        },
    }
}

fn handle_confirm_key(app: &mut App, k: KeyEvent) {
    let target = match &app.mode {
        Mode::Keys(state) => state.confirm_delete.clone(),
        _ => None,
    };
    let Some((provider, key_id)) = target else {
        return;
    };

    match k.code {
        KeyCode::Char('y') => {
            match delete_key(app, &provider, &key_id) {
                Ok(()) => {
                    app.set_toast(Toast::success(format!("key `{provider}/{key_id}` removed")))
                }
                Err(e) => app.set_toast(Toast::error(format!("delete failed: {e}"))),
            }
            if let Mode::Keys(state) = &mut app.mode {
                state.confirm_delete = None;
            }
            clamp_selection_to_current(app);
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            if let Mode::Keys(state) = &mut app.mode {
                state.confirm_delete = None;
            }
        }
        _ => {}
    }
}

/// 写盘后把 `state.list.selected()` 收紧到当前 `flatten(app)` 的可选范围内，
/// 避免删除最后一行后选中越界。
fn clamp_selection_to_current(app: &mut App) {
    let len = flatten(app).len();
    if let Mode::Keys(state) = &mut app.mode {
        if len == 0 {
            state.list.select(None);
        } else if let Some(i) = state.list.selected()
            && i >= len
        {
            state.list.select(Some(len - 1));
        } else if state.list.selected().is_none() {
            state.list.select(Some(0));
        }
    }
}

/// 提交 key 表单（add 或 edit）；驱动 credentials 落盘 + 索引重命名 + 全量 rotate。
///
/// 设计要点：
/// - `add` 流程**不**调 rotate（无 profile 引用此 key）。
/// - `edit` 流程：删旧 section、写新 section、save → 同 provider 内若 id 改了则
///   `rename_key_id_in_index` → 调 `rotate_key` 把新 value 推到所有引用 settings.json。
///   即便 value 未变，rotate 也只是把同一字符串重写一次，无副作用。
/// - 跨 provider edit（罕见路径）只迁移 credentials.toml；引用旧 (provider, key_id)
///   的 profile 会 stale，本 V1 不自动修复，由 doctor 阶段提示。
fn commit_key_form(app: &mut App, snap: KeyFormCommit) -> Result<(String, String), Error> {
    let provider = snap.provider.trim().to_string();
    let value = snap.value;
    let note = snap.note;
    let id_override = snap.id_override.trim().to_string();

    if provider.is_empty() {
        return Err(Error::InvalidKeyId {
            id: String::new(),
            reason: "provider must not be empty".into(),
        });
    }
    if value.is_empty() {
        return Err(Error::InvalidKeyId {
            id: String::new(),
            reason: "value must not be empty".into(),
        });
    }

    // 计算新 id：显式优先；自动需排除编辑流的旧 id（避免跟自己撞）。
    let new_id = if !id_override.is_empty() {
        credentials::validate_id(&id_override)?;
        id_override
    } else {
        let existing: Vec<String> = app
            .credentials
            .by_provider
            .get(&provider)
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default();
        let existing: Vec<String> = match &snap.editing {
            Some((p, old)) if p == &provider => existing.into_iter().filter(|x| x != old).collect(),
            _ => existing,
        };
        credentials::unique_id(&value, &existing)?
    };

    let key = Key {
        value: value.clone(),
        note,
    };

    // 写 credentials：先在新 provider 下移除可能的同 provider 旧 id，再 insert。
    {
        let map = app
            .credentials
            .by_provider
            .entry(provider.clone())
            .or_default();
        if let Some((old_p, old_id)) = &snap.editing
            && old_p == &provider
        {
            map.remove(old_id);
        }
        map.insert(new_id.clone(), key);
    }
    // 跨 provider 编辑：从旧 provider 下移除原 key，并清理空 map。
    if let Some((old_p, old_id)) = &snap.editing
        && old_p != &provider
        && let Some(old_map) = app.credentials.by_provider.get_mut(old_p)
    {
        old_map.remove(old_id);
        if old_map.is_empty() {
            app.credentials.by_provider.remove(old_p);
        }
    }
    credentials::save(&app.paths.credentials(), &app.credentials)?;

    // edit 流程：同步索引 + 重渲染所有引用此 key 的 settings.json。
    if let Some((old_p, old_id)) = &snap.editing {
        if old_p == &provider && old_id != &new_id {
            profile::rename_key_id_in_index(&app.paths, &provider, old_id, &new_id)?;
        }
        let _affected = profile::rotate_key(&app.paths, &provider, &new_id, &value)?;
    }

    Ok((provider, new_id))
}

fn delete_key(app: &mut App, provider: &str, key_id: &str) -> Result<(), Error> {
    if let Some(map) = app.credentials.by_provider.get_mut(provider) {
        map.remove(key_id);
        if map.is_empty() {
            app.credentials.by_provider.remove(provider);
        }
    }
    credentials::save(&app.paths.credentials(), &app.credentials)?;
    Ok(())
}

/// 把 credentials.toml 扁平化为视图行；按 (provider, key_id) 字典序。
pub fn flatten(app: &App) -> Vec<KeyRow> {
    let mut rows = Vec::new();
    for (provider, kmap) in &app.credentials.by_provider {
        for (kid, k) in kmap {
            rows.push(KeyRow {
                provider: provider.clone(),
                key_id: kid.clone(),
                note: k.note.clone(),
                value_redacted: redact(&k.value),
            });
        }
    }
    rows.sort_by(|a, b| a.provider.cmp(&b.provider).then(a.key_id.cmp(&b.key_id)));
    rows
}

/// 脱敏显示：
/// - len <= 8 全 `*`
/// - len > 8 取前 4 + "..." + 后 4
fn redact(v: &str) -> String {
    let n = v.chars().count();
    if n <= 8 {
        "*".repeat(n)
    } else {
        let head: String = v.chars().take(4).collect();
        let tail: String = v
            .chars()
            .rev()
            .take(4)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        format!("{head}...{tail}")
    }
}

pub fn draw(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let Mode::Keys(state) = &app.mode else {
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
        Paragraph::new("Keys (K) — [a] add  [e] edit value  [x] delete  [Esc] back"),
        chunks[0],
    );

    if let Some(form) = state.form.as_deref() {
        draw_form(frame, chunks[1], form);
    } else if let Some((provider, key_id)) = &state.confirm_delete {
        crate::tui::widgets::draw_confirm(
            frame,
            chunks[1],
            "Delete key",
            &format!(
                "Will delete `{provider}/{key_id}`. Profiles referencing it will fail to authenticate. Continue?"
            ),
        );
    } else {
        draw_list(frame, chunks[1], app, state);
    }
    frame.render_widget(Paragraph::new("[Esc] back"), chunks[2]);
}

fn draw_list(frame: &mut Frame<'_>, area: Rect, app: &App, state: &State) {
    let rows = flatten(app);
    let items: Vec<ListItem<'_>> = rows
        .iter()
        .map(|r| {
            ListItem::new(format!(
                "{}  {}  {}  ({})",
                r.provider, r.key_id, r.value_redacted, r.note
            ))
        })
        .collect();
    // ListState 内的 selected() 对照本帧的 rows clamp 一次，
    // 这样从 Profiles 第一次进入时（selected=None 且 rows 非空）能直接亮起一行。
    let mut ls = state.list.clone();
    if rows.is_empty() {
        ls.select(None);
    } else {
        let max = rows.len() - 1;
        let i = ls.selected().unwrap_or(0).min(max);
        ls.select(Some(i));
    }
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Keys"))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(list, area, &mut ls);
}

fn draw_form(frame: &mut Frame<'_>, area: Rect, form: &KeyForm) {
    // 直接用 InputField::render() 会得到 4 个 Paragraph，无法塞进同一个块；
    // 所以这里用 `field_line` 助手按需把每个字段画成一行。
    let lines = vec![
        field_line(&form.provider, form.focus == 0),
        field_line(&form.value, form.focus == 1),
        field_line(&form.id_override, form.focus == 2),
        field_line(&form.note, form.focus == 3),
    ];
    let title = if form.editing.is_some() {
        "Edit key (changing value re-renders all referencing profiles)"
    } else {
        "Add key"
    };
    let p = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::White));
    frame.render_widget(p, area);
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
    use std::collections::BTreeMap;

    #[test]
    fn flatten_orders_by_provider_then_id() {
        let mut store = credentials::Store::default();
        let mut ds = BTreeMap::new();
        ds.insert(
            "kb".into(),
            Key {
                value: "1234567890123".into(),
                note: "".into(),
            },
        );
        ds.insert(
            "ka".into(),
            Key {
                value: "abcdefghij1234".into(),
                note: "".into(),
            },
        );
        store.by_provider.insert("zz".into(), ds.clone());
        store.by_provider.insert("aa".into(), ds);
        // build minimal app
        let p = crate::paths::Paths::with_root(std::env::temp_dir().join("ais-flatten-test"));
        let app = App {
            paths: p,
            providers: vec![],
            credentials: store,
            index: Default::default(),
            mode: Mode::Keys(State::default()),
            toast: None,
            running: true,
            launch_target: None,
        };
        let rows = flatten(&app);
        assert_eq!(rows[0].provider, "aa");
        assert_eq!(rows[0].key_id, "ka");
        assert_eq!(rows.last().unwrap().provider, "zz");
    }

    #[test]
    fn redact_long() {
        assert_eq!(redact("abcdefghijklmnop"), "abcd...mnop");
    }

    #[test]
    fn redact_short() {
        assert_eq!(redact("abc"), "***");
    }
}
