//! TUI 入口：终端 raw mode / alt screen 进出 + 事件循环。
//!
//! 退出语义：
//! - 用户按 q/Esc 或 Ctrl-C → run() 返回 Ok(None)
//! - 用户按 Enter 启动某 profile → run() 返回 Ok(Some(profile_name))，
//!   main.rs 负责后续 claude::launch（execvp 永不返回 / Windows spawn）。
//! - 任何 Error::* 上传给 main.rs，main.rs 打 stderr 并 exit 1。

use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::Result;
use crate::paths::Paths;
use crate::tui::app::{App, AppEvent, Mode};

pub mod app;
pub mod views;
pub mod widgets;

const TICK_MS: u64 = 100;

pub fn run() -> Result<Option<String>> {
    let paths = Paths::from_home()?;
    let mut app = App::new(paths)?;
    let mut terminal = setup_terminal()?;
    let res = event_loop(&mut terminal, &mut app);
    teardown_terminal(&mut terminal).ok();
    match res {
        Ok(()) => Ok(app.launch_target.take()),
        Err(e) => Err(e),
    }
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode().map_err(io_to_err)?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).map_err(io_to_err)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend).map_err(io_to_err)
}

fn teardown_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode().map_err(io_to_err)?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen).map_err(io_to_err)?;
    terminal.show_cursor().map_err(io_to_err)?;
    Ok(())
}

fn event_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    let tick_rate = Duration::from_millis(TICK_MS);
    let mut last_tick = Instant::now();
    while app.running && app.launch_target.is_none() {
        terminal.draw(|f| draw(f, app)).map_err(io_to_err)?;
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_default();
        if event::poll(timeout).map_err(io_to_err)?
            && let Event::Key(k) = event::read().map_err(io_to_err)?
        {
            app.handle_event(AppEvent::Key(k));
        }
        if last_tick.elapsed() >= tick_rate {
            app.handle_event(AppEvent::Tick);
            last_tick = Instant::now();
        }
    }
    Ok(())
}

fn draw(frame: &mut ratatui::Frame<'_>, app: &App) {
    let area = frame.area();
    match &app.mode {
        Mode::Profiles(_) => views::profiles::draw(frame, area, app),
        Mode::Providers(_) => views::providers::draw(frame, area, app),
        Mode::Keys(_) => views::keys::draw(frame, area, app),
        Mode::Wizard(_) => views::wizard::draw(frame, area, app),
        Mode::Doctor(_) => views::doctor::draw(frame, area, app),
        Mode::Help => widgets::draw_help(frame, area),
    }
    if let Some(t) = &app.toast {
        let toast_area = ratatui::layout::Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(3),
            width: area.width,
            height: 1,
        };
        widgets::draw_toast(frame, toast_area, t);
    }
}

fn io_to_err(e: io::Error) -> crate::Error {
    crate::Error::Io {
        path: std::path::PathBuf::from("<terminal>"),
        source: e,
    }
}
