//! 应用状态：单 App + Mode 枚举。所有事件经 handle_event 分派到对应 view。
//!
//! 设计原则：
//! - 数据加载一次性发生在 App::load_state；mode 切换不重读，确保 TUI 反应迅速。
//! - 写操作（创建 profile / 改 key / 删 provider）在调用数据层 API 后调用 reload_state
//!   局部刷新（reload_credentials / reload_index 等）。
//! - launch_target = Some(name) 表示用户在 Profiles 视图按了 Enter，事件循环退出后
//!   main.rs 会用它启动 claude（execvp/spawn）。

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::catalog::Provider;
use crate::credentials;
use crate::error::Error;
use crate::paths::Paths;
use crate::profile::Index;
use crate::providers;
use crate::tui::widgets::Toast;

/// 顶层模式。每个变体内部状态由各 view 模块定义；这里 forward-declare。
#[derive(Debug)]
pub enum Mode {
    Profiles(super::views::profiles::State),
    Providers(super::views::providers::State),
    Keys(super::views::keys::State),
    Doctor(super::views::doctor::State),
    /// Boxed: `wizard::State` 持有多张 `ListState` + `InputField` + `Provider`，
    /// 直接放进 `Mode` 会触发 `clippy::large_enum_variant`（参见 keys.rs 注释中
    /// 同套手法）。Box 后 Mode 体积稳定，访问端通过 `Deref` 自动取到字段。
    Wizard(Box<super::views::wizard::State>),
    Help,
}

/// App 顶层事件（事件循环把 crossterm 事件先包成这个）。
#[derive(Debug, Clone)]
pub enum AppEvent {
    Tick,
    Key(KeyEvent),
}

pub struct App {
    pub paths: Paths,
    pub providers: Vec<Provider>,
    pub credentials: credentials::Store,
    pub index: Index,
    pub mode: Mode,
    pub toast: Option<Toast>,
    pub running: bool,
    pub launch_target: Option<String>,
}

impl App {
    pub fn new(paths: Paths) -> Result<Self, Error> {
        let providers = providers::load_all(&paths.providers())?;
        let credentials = credentials::load(&paths.credentials())?;
        let index = Index::load(&paths.claude_index())?;
        let initial = super::views::profiles::State::new(&index);
        Ok(Self {
            paths,
            providers,
            credentials,
            index,
            mode: Mode::Profiles(initial),
            toast: None,
            running: true,
            launch_target: None,
        })
    }

    pub fn reload_index(&mut self) -> Result<(), Error> {
        self.index = Index::load(&self.paths.claude_index())?;
        Ok(())
    }

    pub fn reload_credentials(&mut self) -> Result<(), Error> {
        self.credentials = credentials::load(&self.paths.credentials())?;
        Ok(())
    }

    pub fn reload_providers(&mut self) -> Result<(), Error> {
        self.providers = providers::load_all(&self.paths.providers())?;
        Ok(())
    }

    /// 顶层事件分派：先处理 mode-agnostic 全局键，再分发到 view。
    pub fn handle_event(&mut self, ev: AppEvent) {
        // toast 自动过期
        if let Some(t) = &self.toast
            && t.expired()
        {
            self.toast = None;
        }
        match ev {
            AppEvent::Tick => {}
            AppEvent::Key(k) => self.handle_key(k),
        }
    }

    fn handle_key(&mut self, k: KeyEvent) {
        // Help overlay：任何键关闭
        if matches!(self.mode, Mode::Help) {
            // 在 Help 模式下，?, q, Esc 都关闭
            self.mode = Mode::Profiles(super::views::profiles::State::new(&self.index));
            return;
        }
        // 全局：Ctrl-C 强退
        if k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c') {
            self.running = false;
            return;
        }
        // 全局：? 打开 Help
        if k.code == KeyCode::Char('?') && !self.is_in_input_mode() {
            self.mode = Mode::Help;
            return;
        }
        // 分派到 view
        match &mut self.mode {
            Mode::Profiles(_) => super::views::profiles::handle_key(self, k),
            Mode::Providers(_) => super::views::providers::handle_key(self, k),
            Mode::Keys(_) => super::views::keys::handle_key(self, k),
            Mode::Doctor(_) => super::views::doctor::handle_key(self, k),
            Mode::Wizard(_) => super::views::wizard::handle_key(self, k),
            Mode::Help => unreachable!("handled above"),
        }
    }

    /// 当前是否在某个文本输入框焦点下；为 true 时 ?/q 等不应当作快捷键拦截。
    /// view 可在自己的 State 里设 input_focused 标志，再通过这里总览汇报。
    pub fn is_in_input_mode(&self) -> bool {
        match &self.mode {
            Mode::Wizard(s) => s.is_in_input_mode(),
            Mode::Keys(s) => s.is_in_input_mode(),
            Mode::Providers(s) => s.is_in_input_mode(),
            _ => false,
        }
    }

    pub fn set_toast(&mut self, t: Toast) {
        self.toast = Some(t);
    }
    pub fn clear_toast(&mut self) {
        self.toast = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::views::profiles::State as ProfilesState;

    fn temp_paths() -> Paths {
        let id = format!(
            "ais-app-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let p = std::env::temp_dir().join(id);
        std::fs::create_dir_all(&p).unwrap();
        Paths::with_root(p)
    }

    #[test]
    fn new_starts_in_profiles_mode() {
        let p = temp_paths();
        let app = App::new(p).unwrap();
        assert!(matches!(app.mode, Mode::Profiles(_)));
        assert!(app.running);
        assert!(app.launch_target.is_none());
    }

    #[test]
    fn ctrl_c_stops_running() {
        let p = temp_paths();
        let mut app = App::new(p).unwrap();
        let ev = AppEvent::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        app.handle_event(ev);
        assert!(!app.running);
    }

    #[test]
    fn question_mark_opens_help() {
        let p = temp_paths();
        let mut app = App::new(p).unwrap();
        let ev = AppEvent::Key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
        app.handle_event(ev);
        assert!(matches!(app.mode, Mode::Help));
    }

    #[test]
    fn help_any_key_closes() {
        let p = temp_paths();
        let mut app = App::new(p).unwrap();
        app.mode = Mode::Help;
        let ev = AppEvent::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        app.handle_event(ev);
        assert!(matches!(app.mode, Mode::Profiles(_)));
    }

    #[test]
    fn reload_index_after_external_write() {
        let p = temp_paths();
        let mut app = App::new(p.clone()).unwrap();
        // 模拟外部写入 index
        std::fs::create_dir_all(p.claude_dir()).unwrap();
        std::fs::write(
            p.claude_index(),
            "[foo]\nprovider=\"x\"\nkey_id=\"y\"\nmodel=\"z\"\ncreated_at=\"2026-04-30T00:00:00Z\"\n",
        )
        .unwrap();
        app.reload_index().unwrap();
        assert!(app.index.entries.contains_key("foo"));
    }

    // 强迫 ProfilesState::new 路径覆盖到（保 reload_index 的 dummy 用例不优化掉）
    #[test]
    fn profiles_state_initializable() {
        let p = temp_paths();
        let app = App::new(p).unwrap();
        let _ = ProfilesState::new(&app.index);
    }
}
