//! Profiles 主视图：左 list 右 details + 顶/底状态栏。
//!
//! 行为（spec §9.1）：
//! - 左列高亮一个 profile，右列显示其 settings.json 的 env 块（key 脱敏）。
//! - 顶栏：ais 版本 / claude 版本 / 配置目录。
//! - 底栏：可用快捷键速览。
//!
//! 仅响应导航/切面板的快捷键；写操作快捷键（n/e/r/x/Enter）在 Task 14 接入。

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
}

impl State {
    pub fn new(index: &Index) -> Self {
        let names: Vec<String> = index.entries.keys().cloned().collect();
        let mut list = ListState::default();
        if !names.is_empty() {
            list.select(Some(0));
        }
        Self { list, names }
    }

    pub fn is_in_input_mode(&self) -> bool {
        false
    }

    pub fn selected_name(&self) -> Option<&str> {
        self.list
            .selected()
            .and_then(|i| self.names.get(i).map(String::as_str))
    }
}

pub fn handle_key(app: &mut App, k: KeyEvent) {
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
            app.mode = Mode::Providers(crate::tui::views::providers::State);
        }
        KeyCode::Char('K') => {
            app.mode = Mode::Keys(crate::tui::views::keys::State);
        }
        KeyCode::Char('d') => {
            app.mode = Mode::Doctor(crate::tui::views::doctor::State);
        }
        // n/e/r/x/Enter 由 Task 14 接入
        _ => {}
    }
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
