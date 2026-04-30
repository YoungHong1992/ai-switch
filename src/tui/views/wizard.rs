//! Plan B Task 11 落地真实 UI；本文件当前为占位以让 app.rs 编译通过。

use crossterm::event::KeyEvent;

use crate::profile::Index;
use crate::tui::app::App;

#[derive(Debug, Default)]
pub struct State;

impl State {
    pub fn new(_index: &Index) -> Self {
        Self
    }
    pub fn is_in_input_mode(&self) -> bool {
        false
    }
}

pub fn handle_key(_app: &mut App, _k: KeyEvent) {}

pub fn draw(
    _frame: &mut ratatui::Frame<'_>,
    _area: ratatui::layout::Rect,
    _app: &crate::tui::app::App,
) {
}
