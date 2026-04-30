//! 共享小组件。所有 view 通过这里复用 Toast / 输入框 / 确认对话框。

use std::time::Instant;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

#[derive(Debug, Clone, Copy)]
pub enum ToastKind {
    Info,
    Error,
    Success,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub kind: ToastKind,
    pub message: String,
    pub created: Instant,
}

impl Toast {
    pub const TTL_SECS: u64 = 3;

    pub fn info(msg: impl Into<String>) -> Self {
        Self::new(ToastKind::Info, msg)
    }
    pub fn error(msg: impl Into<String>) -> Self {
        Self::new(ToastKind::Error, msg)
    }
    pub fn success(msg: impl Into<String>) -> Self {
        Self::new(ToastKind::Success, msg)
    }
    fn new(kind: ToastKind, msg: impl Into<String>) -> Self {
        Self {
            kind,
            message: msg.into(),
            created: Instant::now(),
        }
    }

    pub fn expired(&self) -> bool {
        self.created.elapsed().as_secs() >= Self::TTL_SECS
    }
}

pub fn draw_toast(frame: &mut Frame<'_>, area: Rect, toast: &Toast) {
    let style = match toast.kind {
        ToastKind::Info => Style::default().fg(Color::White),
        ToastKind::Error => Style::default().fg(Color::White).bg(Color::Red),
        ToastKind::Success => Style::default().fg(Color::Black).bg(Color::Green),
    };
    let para = Paragraph::new(toast.message.clone())
        .style(style)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });
    frame.render_widget(para, area);
}

/// 单行输入框，支持掩码（用于 key value）与最大长度。
#[derive(Debug, Clone)]
pub struct InputField {
    pub label: String,
    pub buffer: String,
    pub mask: bool,
    pub max_len: usize,
}

impl InputField {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            buffer: String::new(),
            mask: false,
            max_len: 256,
        }
    }
    pub fn masked(mut self) -> Self {
        self.mask = true;
        self
    }
    pub fn with_max_len(mut self, n: usize) -> Self {
        self.max_len = n;
        self
    }
    pub fn with_initial(mut self, s: &str) -> Self {
        self.buffer = s.into();
        self
    }
    pub fn push(&mut self, c: char) {
        if self.buffer.chars().count() < self.max_len {
            self.buffer.push(c);
        }
    }
    pub fn pop(&mut self) {
        self.buffer.pop();
    }
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
    pub fn value(&self) -> &str {
        &self.buffer
    }
    pub fn render(&self, focused: bool) -> Paragraph<'_> {
        let display = if self.mask {
            "*".repeat(self.buffer.chars().count())
        } else {
            self.buffer.clone()
        };
        let cursor = if focused { "_" } else { "" };
        let line = Line::from(vec![
            Span::styled(
                format!("{}: ", self.label),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(display),
            Span::styled(cursor, Style::default().fg(Color::Yellow)),
        ]);
        Paragraph::new(line)
    }
}

/// 居中对齐的确认对话框。返回值由调用方根据 y/n 自行处理。
pub fn draw_confirm(frame: &mut Frame<'_>, area: Rect, title: &str, body: &str) {
    let popup = centered_rect(60, 30, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title.to_string())
        .style(Style::default().fg(Color::Yellow));
    let para = Paragraph::new(format!("{body}\n\n[y] 确认  [n] 取消"))
        .block(block)
        .wrap(Wrap { trim: true });
    frame.render_widget(para, popup);
}

/// 屏幕居中切矩形（百分比宽高）。
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_push_pop_clear() {
        let mut f = InputField::new("Name").with_max_len(3);
        f.push('a');
        f.push('b');
        f.push('c');
        f.push('d'); // 超长被丢
        assert_eq!(f.value(), "abc");
        f.pop();
        assert_eq!(f.value(), "ab");
        f.clear();
        assert_eq!(f.value(), "");
    }

    #[test]
    fn toast_expiry_is_relative_to_now() {
        let t = Toast::info("x");
        assert!(!t.expired()); // 刚创建不会过期
    }
}
