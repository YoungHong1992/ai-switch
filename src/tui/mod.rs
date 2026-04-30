//! TUI 入口（Task 7 落地终端生命周期与事件循环）

pub mod app;
pub mod views;
pub mod widgets;

use crate::Result;

/// TUI 退出码：launch_target 非 None 时，main.rs 在 TUI 退出后用它启动 claude；
/// None 表示用户用 q/Esc 正常退出。
pub fn run() -> Result<Option<String>> {
    // Plan B Task 7 实现；当前为占位。
    Ok(None)
}
