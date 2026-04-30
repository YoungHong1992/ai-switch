# ai-switch V1 Plan B — TUI (M3 + M4 + M5)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 spec §7、§9、Doctor (§9.4)、启动语义 (§8) 全部以 ratatui TUI 形式落地，使 `ais`（裸跑）进入主视图，覆盖创建向导、profile 编辑/重命名/删除/启动、provider/key CRUD、Doctor 6 项检查；保证退出后终端 termios 干净。Plan A 留下的"TUI not yet available"占位被替换。

**Architecture:** 单 `App` 状态机 + `Mode` 枚举（Profiles / Wizard / Keys / Providers / Doctor / Help），所有事件流入 `App::handle_event` 分派到对应 view 的 `update`，渲染流入 view 的 `draw`。HTTP 拉模型走同步 ureq + 5s 超时（不引入 worker 线程）；UI 在 fetch 期间短暂"freeze"是可接受的（spec §16）。`Enter` 启动 = TUI 把 `launch_target` 设上 → 退出事件循环 → 在 `main.rs` 里 `claude::launch()`（execvp on unix / spawn on windows）。

**Tech Stack:** Rust 2024 edition；新增依赖 `ratatui = "0.29"` + `crossterm = "0.28"` + `ureq = { version = "2", features = ["json"] }`。沿用 Plan A 已有的 `serde` / `serde_json` / `toml` / `thiserror` / `clap` / `which` / `directories` / `chrono`。不引入 `tokio` / `reqwest` / `keyring` / `tempfile`。

**对 spec 的引用规则：** 行为契约、UI mockup、Doctor 检查项以 `docs/superpowers/specs/2026-04-29-ai-switch-design.md` 为准；本 plan 把 spec 拆成 16 个 task，每个 task 自身可编译/通过测试可提交。

**对 Plan A 的承袭：** 所有数据层 API（`profile::create / delete / rotate_key / rename_key_id_in_index`、`credentials::{load, save, auto_id, unique_id, validate_id}`、`providers::load_all`、`settings::Settings`、`paths::Paths`、`claude::{probe_path, probe_version, launch}`）已经在 `feature/plan-a-foundations` 合入 main，本 plan 只**调用**它们，**不重写**。`profile::create` 唯一的扩展是 Task 3 的事务回滚。

---

## 文件结构（Plan B 完成后新增/修改的文件）

```
ai-switch/
├── Cargo.toml                                # 修改：+ ratatui / crossterm / ureq
├── src/
│   ├── lib.rs                                # 修改：pub mod http; pub mod tui;
│   ├── main.rs                               # 修改：None 分支 → tui::run()，结果驱动 launch
│   ├── http.rs                               # 新增：/v1/models 拉取
│   ├── profile.rs                            # 修改：create() 增加事务回滚
│   └── tui/
│       ├── mod.rs                            # 新增：run() + terminal lifecycle + event loop
│       ├── app.rs                            # 新增：App / Mode / AppEvent / 顶层 dispatch
│       ├── widgets.rs                        # 新增：Toast / InputField / ConfirmDialog
│       └── views/
│           ├── mod.rs                        # 新增：子模块声明
│           ├── profiles.rs                   # 新增：主视图
│           ├── providers.rs                  # 新增：p 面板
│           ├── keys.rs                       # 新增：k 面板（含 rotate 触发）
│           ├── wizard.rs                     # 新增：5 步向导 + 子流程
│           └── doctor.rs                     # 新增：d 面板
└── docs/superpowers/plans/
    └── 2026-04-30-ai-switch-v1-plan-b-tui.md ← 本文件
```

> **备注**：`src/main.rs` 的 launch 路径在 Plan A 已落地，本 plan 在 main.rs 里只新增"TUI 退出后是否需要 launch"的分支（基于 `tui::run()` 的返回值）。`claude::launch` 的 execvp/spawn 实现不动。

---

## Task 顺序速览

| # | Title | 主要文件 |
|---|---|---|
| 1 | 切分支 + Cargo 依赖 + 模块骨架 | `Cargo.toml`, `src/lib.rs`, `src/tui/{mod,app,widgets}.rs`, `src/tui/views/mod.rs`, `src/http.rs` |
| 2 | `http.rs` /v1/models 拉取 + 测试 | `src/http.rs` |
| 3 | `profile::create` 事务回滚 | `src/profile.rs` |
| 4 | `tui/widgets.rs` 共享组件 | `src/tui/widgets.rs` |
| 5 | `tui/app.rs` App + Mode + 事件分派 | `src/tui/app.rs` |
| 6 | `tui/views/profiles.rs` 只读主视图 | `src/tui/views/profiles.rs` |
| 7 | `tui/mod.rs` 终端生命周期 + 事件循环 | `src/tui/mod.rs` |
| 8 | `main.rs` 接 TUI；联调启动路径 | `src/main.rs` |
| 9 | `tui/views/providers.rs` p 面板 | `src/tui/views/providers.rs` |
| 10 | `tui/views/keys.rs` k 面板 + rotate 联动 | `src/tui/views/keys.rs` |
| 11 | `tui/views/wizard.rs` 状态机骨架 + step 1/4/5 | `src/tui/views/wizard.rs` |
| 12 | wizard step 3：HTTP 拉模型 + fallback | `src/tui/views/wizard.rs` |
| 13 | wizard 内联 add-provider / add-key 子流程 | `src/tui/views/wizard.rs` |
| 14 | profiles 视图接 `n` `e` `r` `x` `Enter` | `src/tui/views/profiles.rs`, `src/tui/app.rs` |
| 15 | `tui/views/doctor.rs` 6 项检查 | `src/tui/views/doctor.rs` |
| 16 | Help overlay + 空状态 + 人工验收清单 | `src/tui/{widgets.rs, app.rs}`, `README.md`（人工） |

每个 task 的 commit 与 push 与 Plan A 一致：本地通过 `cargo fmt --check` + `cargo clippy -- -D warnings` + `cargo test --workspace --all-targets` 三件套后再 commit。最后整体推 `feature/plan-b-tui` 并开 PR。

---

### Task 1: 切分支 + Cargo 依赖 + 模块骨架

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/lib.rs`
- Create: `src/http.rs`
- Create: `src/tui/mod.rs`
- Create: `src/tui/app.rs`
- Create: `src/tui/widgets.rs`
- Create: `src/tui/views/mod.rs`

> 当前已在 `feature/plan-b-tui` 分支上（brainstorming 完成时切的）；如不在，先 `git checkout -b feature/plan-b-tui main`。

- [ ] **Step 1: 验证分支与基线**

Run: `git status && git branch --show-current`
Expected: 工作区干净，分支为 `feature/plan-b-tui`。

- [ ] **Step 2: 修改 `Cargo.toml` 增加 ratatui / crossterm / ureq**

把 `[dependencies]` 段改成：

```toml
[dependencies]
serde       = { version = "1", features = ["derive"] }
serde_json  = "1"
toml        = "0.8"
thiserror   = "2"
anyhow      = "1"
clap        = { version = "4", features = ["derive"] }
which       = "7"
directories = "5"
chrono      = { version = "0.4", features = ["serde"] }
ratatui     = "0.29"
crossterm   = "0.28"
ureq        = { version = "2", features = ["json"] }
```

- [ ] **Step 3: 修改 `src/lib.rs` 暴露新模块**

```rust
pub mod catalog;
pub mod claude;
pub mod credentials;
pub mod error;
pub mod http;
pub mod paths;
pub mod profile;
pub mod providers;
pub mod settings;
pub mod tui;

pub use error::Error;

pub type Result<T> = std::result::Result<T, Error>;
```

- [ ] **Step 4: 创建 `src/http.rs` 占位**

```rust
//! /v1/models 拉取（Task 2 落地实际逻辑）
```

- [ ] **Step 5: 创建 `src/tui/mod.rs` 占位**

```rust
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
```

- [ ] **Step 6: 创建 `src/tui/app.rs` / `widgets.rs` / `views/mod.rs` 占位**

`src/tui/app.rs`：
```rust
//! App 状态与事件分派（Task 5 落地）
```

`src/tui/widgets.rs`：
```rust
//! 共享小组件（Task 4 落地）
```

`src/tui/views/mod.rs`：
```rust
//! 各 view 子模块声明（Task 6/9/10/11/15 逐个补齐）
```

- [ ] **Step 7: 验证编译**

Run: `cargo build 2>&1 | tail -10`
Expected: 编译通过；新增依赖被解析、占位文件无 warning。

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml Cargo.lock src/lib.rs src/http.rs src/tui/
git commit -m "chore(plan-b): bootstrap tui/http modules + ratatui/crossterm/ureq deps"
```

---

### Task 2: `http.rs` —— /v1/models 拉取 + 解析 + 测试

**Files:**
- Modify: `src/http.rs`
- Modify: `src/error.rs` (+ HttpFetch variant)

**对 spec 的引用：** §7 Step 3（5 秒超时 + 失败回退）、§5.2（端点路径默认 `/v1/models`）。

- [ ] **Step 1: 在 `src/error.rs` 增加 HttpFetch variant**

在现有 `Error` 枚举末尾（`Io` 之前）插入：

```rust
    #[error("http fetch failed: {url}: {message}")]
    HttpFetch { url: String, message: String },

    #[error("http response is not valid JSON ({url}): {message}")]
    HttpJson { url: String, message: String },
```

> 不直接持 `ureq::Error` 是因为 `ureq::Error` 不实现 `Send + Sync` 在某些上下文下不便。我们把消息字符串化即可——错误展示走 stderr/toast，不需要恢复原始链。

- [ ] **Step 2: 写 `src/http.rs` 完整内容**

```rust
//! 同步 HTTP 客户端：从 OpenAI 兼容 /v1/models 端点拉模型 id 列表。
//!
//! 设计取舍（spec §16 已声明可接受）：
//! - **同步阻塞**：用 ureq，5 秒 timeout；调用方 (wizard) 在 fetch 期间 UI 短暂 freeze。
//! - **不重试**：网络异常 / 解析失败 → 上层 wizard 切换到自由输入子状态。
//! - **不持 Authorization**：/v1/models 在多数 provider 下要鉴权，但 V1 假定 wizard
//!   step 3 时用户已选好 key（在 step 2），把 key value 透传 fetch_models 的 `bearer` 入参。

use std::time::Duration;

use serde::Deserialize;

use crate::error::Error;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// OpenAI-style /v1/models 响应：
/// ```json
/// {"data": [{"id": "deepseek-chat"}, {"id": "deepseek-coder"}]}
/// ```
#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<ModelEntry>,
}

#[derive(Debug, Deserialize)]
struct ModelEntry {
    id: String,
}

/// 拉取并返回模型 id 列表（按返回顺序保留）。
///
/// `base_url` + `endpoint_path` 拼成 URL；空字符串 base 视为非法，返回 HttpFetch。
/// `bearer` 为空时不带 Authorization 头（少数 provider 的 /v1/models 可匿名拉）。
pub fn fetch_models(
    base_url: &str,
    endpoint_path: &str,
    bearer: Option<&str>,
) -> Result<Vec<String>, Error> {
    let url = build_url(base_url, endpoint_path);
    let agent = ureq::AgentBuilder::new()
        .timeout(DEFAULT_TIMEOUT)
        .build();
    let mut req = agent.get(&url);
    if let Some(token) = bearer {
        req = req.set("Authorization", &format!("Bearer {token}"));
    }
    let resp = req.call().map_err(|e| Error::HttpFetch {
        url: url.clone(),
        message: e.to_string(),
    })?;
    let text = resp.into_string().map_err(|e| Error::HttpFetch {
        url: url.clone(),
        message: e.to_string(),
    })?;
    parse_models_response(&text).map_err(|msg| Error::HttpJson {
        url,
        message: msg,
    })
}

/// 拼 URL（trim 末尾 /，确保 endpoint 以 / 开头）。
fn build_url(base: &str, endpoint: &str) -> String {
    let base = base.trim_end_matches('/');
    if endpoint.starts_with('/') {
        format!("{base}{endpoint}")
    } else {
        format!("{base}/{endpoint}")
    }
}

/// 把 JSON 响应解析成模型 id 列表。
///
/// 把这一步独立出来纯函数化，便于离线单元测试。
pub fn parse_models_response(json: &str) -> Result<Vec<String>, String> {
    let parsed: ModelsResponse = serde_json::from_str(json)
        .map_err(|e| format!("parse error: {e}"))?;
    Ok(parsed.data.into_iter().map(|m| m.id).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_url_strips_trailing_slash() {
        assert_eq!(
            build_url("https://api.example.com/v1/", "/models"),
            "https://api.example.com/v1/models"
        );
        assert_eq!(
            build_url("https://api.example.com/v1", "models"),
            "https://api.example.com/v1/models"
        );
    }

    #[test]
    fn parse_extracts_ids_in_order() {
        let json = r#"{"data":[{"id":"a"},{"id":"b"},{"id":"c"}]}"#;
        assert_eq!(parse_models_response(json).unwrap(), vec!["a", "b", "c"]);
    }

    #[test]
    fn parse_empty_data_returns_empty_vec() {
        let json = r#"{"data":[]}"#;
        assert!(parse_models_response(json).unwrap().is_empty());
    }

    #[test]
    fn parse_invalid_json_returns_err() {
        assert!(parse_models_response("not json {{{").is_err());
    }

    #[test]
    fn parse_missing_data_field_returns_err() {
        assert!(parse_models_response(r#"{"foo":1}"#).is_err());
    }

    #[test]
    fn parse_ignores_extra_fields() {
        let json = r#"{"data":[{"id":"x","object":"model","owned_by":"demo"}],"object":"list"}"#;
        assert_eq!(parse_models_response(json).unwrap(), vec!["x"]);
    }
}
```

- [ ] **Step 3: 跑测试**

Run: `cargo test --lib http`
Expected: 6 tests pass。

- [ ] **Step 4: 验证 fmt + clippy 干净**

Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -20`
Expected: 无输出 + clippy `Finished`.

- [ ] **Step 5: Commit**

```bash
git add src/http.rs src/error.rs
git commit -m "feat(http): /v1/models fetcher + JSON parser (5s timeout)"
```

---

### Task 3: `profile::create` 事务回滚

**Files:**
- Modify: `src/profile.rs`

**对 spec 的引用：** §7 Step 5"事务失败：回滚已创建的文件"。

Plan A 的 `profile::create` 当前是先写 `settings_<name>.json` 再写 `.ais-index.toml`，任何一步失败都不回滚，会留下"孤儿 settings.json + 索引未更新"。Plan B 的向导依赖事务语义，先在数据层修。

- [ ] **Step 1: 把 `create()` 改成"先准备，后落盘；任意一步失败回滚"**

定位 `src/profile.rs` 当前的 `create` 函数，整体替换为：

```rust
/// Create a new profile: 写 settings_<name>.json + 追加索引项。
///
/// 事务语义（spec §7 Step 5）：
/// 1. 校验 name；
/// 2. 准备好 Settings 与 IndexEntry（不落盘）；
/// 3. 加载现有索引；新条目会冲掉同名旧条目，但需检测（重名应在调用方处理，本函数允许覆盖）；
/// 4. 先写 settings.json；
/// 5. 再写 index.toml；若 index 写失败，删除已写的 settings.json 后返回错误。
pub fn create(paths: &Paths, input: CreateInput) -> Result<(), Error> {
    validate_name(input.name)?;

    fs::create_dir_all(paths.claude_dir()).map_err(|e| Error::Io {
        path: paths.claude_dir(),
        source: e,
    })?;

    let settings = Settings::render(
        input.anthropic_base_url,
        input.api_key_value,
        input.model,
    );
    let settings_path = paths.settings_for(input.name);

    // 备份"原 settings 是否存在"信号，回滚时只删我们刚刚写出的版本。
    let preexisting = settings_path.exists();
    settings.save(&settings_path)?;

    let mut index = match Index::load(&paths.claude_index()) {
        Ok(i) => i,
        Err(e) => {
            // index 加载失败：回滚 settings 写入（仅当我们刚刚新建时）
            if !preexisting {
                let _ = fs::remove_file(&settings_path);
            }
            return Err(e);
        }
    };
    let entry = IndexEntry {
        provider: input.provider_id.into(),
        key_id: input.key_id.into(),
        model: input.model.into(),
        created_at: Utc::now(),
    };
    index.entries.insert(input.name.into(), entry);

    if let Err(e) = index.save(&paths.claude_index()) {
        // index 写盘失败：回滚 settings.json
        if !preexisting {
            let _ = fs::remove_file(&settings_path);
        }
        return Err(e);
    }
    Ok(())
}
```

- [ ] **Step 2: 在 `tests` 模块追加回滚测试**

定位 `src/profile.rs` 末尾 `#[cfg(test)] mod tests { ... }`，在 `rename_key_id_in_index_only_targets_provider` 测试之后追加：

```rust
    #[test]
    fn create_rolls_back_settings_when_index_save_fails() {
        // 模拟 index 写盘失败：把 claude_dir 下提前放一个**目录**叫 .ais-index.toml，
        // 这样后续 fs::write 就会因 IsADirectory 失败。
        let root = TempRoot(temp_root());
        let paths = Paths::with_root(root.0.join("ais"));
        fs::create_dir_all(paths.claude_dir()).unwrap();
        // 这一行制造写失败：把索引文件占成目录
        fs::create_dir_all(paths.claude_index()).unwrap();

        let result = create(
            &paths,
            CreateInput {
                name: "boom",
                provider_id: "deepseek",
                key_id: "sk-a...fswv",
                model: "deepseek-chat",
                anthropic_base_url: "u",
                api_key_value: "v",
            },
        );
        assert!(result.is_err(), "expected create() to fail when index path is a dir");

        // 回滚断言：settings.json 不应该残留
        let settings_path = paths.settings_for("boom");
        assert!(
            !settings_path.exists(),
            "expected settings.json to be rolled back; found at {}",
            settings_path.display()
        );
    }
```

- [ ] **Step 3: 跑测试**

Run: `cargo test --lib profile`
Expected: 9 tests pass（原 8 + 新 1）。

- [ ] **Step 4: 三件套自检**

Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings`
Expected: 全绿。

- [ ] **Step 5: Commit**

```bash
git add src/profile.rs
git commit -m "feat(profile): transactional rollback of settings.json when index save fails"
```

---

### Task 4: `tui/widgets.rs` —— 共享 Toast / InputField / ConfirmDialog

**Files:**
- Modify: `src/tui/widgets.rs`

只放纯渲染组件 + 极薄状态。事件由各 view 自己处理，widgets 只提供"画"与"持有输入缓冲"。

- [ ] **Step 1: 写 `src/tui/widgets.rs` 完整内容**

```rust
//! 共享小组件。所有 view 通过这里复用 Toast / 输入框 / 确认对话框。

use std::time::Instant;

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

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
        .title(title)
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
```

- [ ] **Step 2: 跑测试**

Run: `cargo test --lib tui::widgets`
Expected: 2 tests pass。

- [ ] **Step 3: 三件套自检**

Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -10`
Expected: 全绿。

- [ ] **Step 4: Commit**

```bash
git add src/tui/widgets.rs
git commit -m "feat(tui): shared widgets — Toast, InputField, ConfirmDialog"
```

---

### Task 5: `tui/app.rs` —— App + Mode + AppEvent + 顶层 dispatch

**Files:**
- Modify: `src/tui/app.rs`

App 是无 Generic 的简单状态机，所有 view 状态用枚举内嵌，事件分派在 `handle_event` 中按 mode 匹配。

- [ ] **Step 1: 写 `src/tui/app.rs` 完整内容**

```rust
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
    Wizard(super::views::wizard::State),
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
        if let Some(t) = &self.toast {
            if t.expired() {
                self.toast = None;
            }
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
```

> 注：上面 `super::views::profiles::State` / `handle_key` / `is_in_input_mode()` 等符号在 Task 6/9/10/11 落地前会让编译失败。本 task 只把 `app.rs` 占位为一个**有意编译失败**的中间态，便于 Task 6 起点对得上签名。**为了让 Task 1-5 全程编译，这里采用如下兜底**：在 `src/tui/views/mod.rs` 暂时声明 5 个空 view 子模块，每个 view 子模块仅暴露 `pub struct State; impl State { pub fn new(_: &Index) -> Self { Self } pub fn is_in_input_mode(&self) -> bool { false } } pub fn handle_key(_: &mut crate::tui::app::App, _: KeyEvent) {}`，由后续 task 补全。

- [ ] **Step 2: 同步把 `src/tui/views/mod.rs` 与 5 个空 view 子模块写出来**

`src/tui/views/mod.rs`：
```rust
pub mod doctor;
pub mod keys;
pub mod profiles;
pub mod providers;
pub mod wizard;
```

为每个 view 创建占位文件（`src/tui/views/{profiles,providers,keys,wizard,doctor}.rs`），每个内容相同模式（以 `profiles.rs` 为例）：

```rust
//! Plan B Task 6 落地真实 UI；本文件当前为占位以让 app.rs 编译通过。

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
```

`providers.rs`、`keys.rs`、`wizard.rs`、`doctor.rs` 写完全相同的占位（仅模块注释里写自己的 task 编号）。

- [ ] **Step 3: 验证编译 + 跑 app 测试**

Run: `cargo test --lib tui::app`
Expected: 6 tests pass。

- [ ] **Step 4: 三件套自检**

Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings`
Expected: 全绿。

- [ ] **Step 5: Commit**

```bash
git add src/tui/app.rs src/tui/views/
git commit -m "feat(tui): App + Mode + event dispatch + view stubs"
```

---

### Task 6: `tui/views/profiles.rs` 主视图（只读）

**Files:**
- Modify: `src/tui/views/profiles.rs`

本 task 只画 list + details + 状态栏，不接 `n/e/r/x/Enter`（Task 14 接）。`p/k/d` 切换其他面板的逻辑在这里加。

- [ ] **Step 1: 整体重写 `src/tui/views/profiles.rs`**

```rust
//! Profiles 主视图：左 list 右 details + 顶/底状态栏。
//!
//! 行为（spec §9.1）：
//! - 左列高亮一个 profile，右列显示其 settings.json 的 env 块（key 脱敏）。
//! - 顶栏：ais 版本 / claude 版本 / 配置目录。
//! - 底栏：可用快捷键速览。
//!
//! 仅响应导航/切面板的快捷键；写操作快捷键（n/e/r/x/Enter）在 Task 14 接入。

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

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
        self.list.selected().and_then(|i| self.names.get(i).map(String::as_str))
    }
}

pub fn handle_key(app: &mut App, k: KeyEvent) {
    let Mode::Profiles(state) = &mut app.mode else { return };
    match k.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.running = false;
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if !state.names.is_empty() {
                let i = state.list.selected().unwrap_or(0);
                let next = (i + 1).min(state.names.len() - 1);
                state.list.select(Some(next));
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if !state.names.is_empty() {
                let i = state.list.selected().unwrap_or(0);
                state.list.select(Some(i.saturating_sub(1)));
            }
        }
        KeyCode::Char('p') => {
            app.mode = Mode::Providers(crate::tui::views::providers::State::default());
        }
        KeyCode::Char('K') => {
            app.mode = Mode::Keys(crate::tui::views::keys::State::default());
        }
        KeyCode::Char('d') => {
            app.mode = Mode::Doctor(crate::tui::views::doctor::State::default());
        }
        // n/e/r/x/Enter 由 Task 14 接入
        _ => {}
    }
}

pub fn draw(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let Mode::Profiles(state) = &app.mode else { return };
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
        let tail: String = chars.iter().rev().take(4).collect::<String>().chars().rev().collect();
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
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
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
        use chrono::Utc;
        use crate::profile::IndexEntry;
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
```

> 关于 `K`（大写）作为 keys 面板的快捷键：spec §9.1 标的是 `k`，但 vim 风格上下移动也用 `k`，二者会撞。Plan B 决定主视图里 `k` 用作"上移"（与 `↑` 同义），用 `K` 进 keys 面板。文档会在 Task 16 的 Help overlay 中标明。Doctor 用 `d` 不撞。

- [ ] **Step 2: 跑测试**

Run: `cargo test --lib tui::views::profiles`
Expected: 4 tests pass。

- [ ] **Step 3: 三件套自检**

Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings`
Expected: 全绿。

- [ ] **Step 4: Commit**

```bash
git add src/tui/views/profiles.rs
git commit -m "feat(tui): profiles main view (read-only) + redacted key display"
```

---

### Task 7: `tui/mod.rs` —— 终端生命周期 + 事件循环

**Files:**
- Modify: `src/tui/mod.rs`

- [ ] **Step 1: 重写 `src/tui/mod.rs`**

```rust
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
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::paths::Paths;
use crate::tui::app::{App, AppEvent, Mode};
use crate::Result;

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

fn teardown_terminal(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    disable_raw_mode().map_err(io_to_err)?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen).map_err(io_to_err)?;
    terminal.show_cursor().map_err(io_to_err)?;
    Ok(())
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    let tick_rate = Duration::from_millis(TICK_MS);
    let mut last_tick = Instant::now();
    while app.running && app.launch_target.is_none() {
        terminal
            .draw(|f| draw(f, app))
            .map_err(io_to_err)?;
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_default();
        if event::poll(timeout).map_err(io_to_err)? {
            if let Event::Key(k) = event::read().map_err(io_to_err)? {
                app.handle_event(AppEvent::Key(k));
            }
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
```

- [ ] **Step 2: 给所有 view 子模块的占位补 `pub fn draw(_, _, _) {}` 入口**

由于 `tui::mod.rs` 的 `draw` 调用 `views::providers::draw / keys::draw / wizard::draw / doctor::draw`，给每个 view 占位文件追加：

```rust
pub fn draw(
    _frame: &mut ratatui::Frame<'_>,
    _area: ratatui::layout::Rect,
    _app: &crate::tui::app::App,
) {
}
```

> profiles.rs 已经在 Task 6 实现 draw，不动。

- [ ] **Step 3: 给 `widgets.rs` 追加 `draw_help`**

在 `src/tui/widgets.rs` 末尾追加：

```rust
pub fn draw_help(frame: &mut Frame<'_>, area: Rect) {
    let popup = centered_rect(70, 70, area);
    frame.render_widget(Clear, popup);
    let block = Block::default().borders(Borders::ALL).title("Help — ai-switch");
    let body = "\
Profiles 主视图:
  ↑↓ / j k       move
  Enter           launch selected profile
  n               new profile (wizard)
  e               edit selected (re-run wizard, name locked)
  r               rename selected
  x               delete selected (confirm with y)
  p               providers panel
  K               keys panel
  d               doctor panel
  ?               toggle this help
  q / Esc         quit

向导内:
  Tab / Shift-Tab next/prev step
  Enter           accept / next
  Esc             back / cancel
";
    let para = Paragraph::new(body)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(para, popup);
}
```

- [ ] **Step 4: 验证编译 + 跑测试**

Run: `cargo build && cargo test --lib`
Expected: 编译通过；现有所有测试 pass（Plan A 34 + Plan B 累积）。

> 注：`tui::run()` 没有自动测试覆盖（依赖真实 TTY），通过 Task 8 与人工验收覆盖。

- [ ] **Step 5: Commit**

```bash
git add src/tui/mod.rs src/tui/widgets.rs src/tui/views/
git commit -m "feat(tui): event loop + terminal lifecycle + help overlay"
```

---

### Task 8: `main.rs` —— 接 TUI；启动路径联调

**Files:**
- Modify: `src/main.rs`

**对 spec 的引用：** §8 启动语义（unix execvp / windows spawn 已在 Plan A 实现）；本 task 的新 piece 是"TUI 退出后如带回 launch_target，自动 launch"。

- [ ] **Step 1: 重写 `src/main.rs`**

```rust
use std::process::ExitCode;

use ais::{claude, paths::Paths, tui};
use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "ais",
    version,
    about = "Claude Code 配置切换工具",
    long_about = "ais — Claude Code 配置切换工具。\n\
                  裸跑 `ais` 进入 TUI；\n\
                  `ais claude <name>` 直接启动一个 profile 对应的 Claude Code。"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Debug, clap::Subcommand)]
enum Cmd {
    /// 启动 Claude Code，使用 ~/.ai-switch/claude/settings_<name>.json
    Claude {
        /// profile 名（例如 deepseek_deepseek-chat）
        name: String,
        /// 透传给 claude 的剩余参数
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        passthrough: Vec<String>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.cmd {
        None => match run_tui() {
            Ok(0) => ExitCode::from(0),
            Ok(code) => ExitCode::from(code as u8),
            Err(e) => {
                eprintln!("ais: {e}");
                ExitCode::from(1)
            }
        },
        Some(Cmd::Claude { name, passthrough }) => match run_claude(&name, &passthrough) {
            Ok(code) => ExitCode::from(code as u8),
            Err(e) => {
                eprintln!("ais: {e}");
                ExitCode::from(1)
            }
        },
    }
}

/// TUI 路径：进入主视图；用户按 Enter 选定一个 profile 后，TUI 退出并把 name 返回，
/// 这里再调 claude::launch（execvp on unix；windows spawn）。
fn run_tui() -> ais::Result<i32> {
    match tui::run()? {
        None => Ok(0),
        Some(name) => run_claude(&name, &[]),
    }
}

fn run_claude(name: &str, passthrough: &[String]) -> ais::Result<i32> {
    let paths = Paths::from_home()?;
    let settings_path = paths.settings_for(name);
    if !settings_path.exists() {
        return Err(ais::Error::ProfileNotFound { name: name.into() });
    }
    let claude_path = claude::probe_path()?;
    claude::launch(&claude_path, &settings_path, passthrough)
}
```

- [ ] **Step 2: 验证编译 + 占位 TUI 能跑**

Run: `cargo build`
Expected: pass。

> 此时 TUI 仍只能展示 Profiles 主视图（其他面板空）。`q` 退出后无 launch。手工验证：

```bash
AIS_HOME=$(mktemp -d) cargo run -q
```

按 `q` 退出，确认无残留 termios（控制台正常）。

- [ ] **Step 3: launch_smoke.rs 仍然全绿**

Run: `cargo test --test launch_smoke`
Expected: 2 tests pass（Plan A 已有；TUI 不影响 `ais claude <name>` 路径）。

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat(main): wire TUI entry; launch_target → claude::launch"
```

---

### Task 9: `tui/views/providers.rs` —— Providers 面板

**Files:**
- Modify: `src/tui/views/providers.rs`

**对 spec 的引用：** §5.3、§9.2。内置 provider 只读；用户自定义增删改。

- [ ] **Step 1: 整体重写 `src/tui/views/providers.rs`**

```rust
//! Providers 面板：列出 builtins + 用户自定义；用户自定义可增删改。
//!
//! 具体 CRUD：
//! - 增：从 catalog 末尾"+ 添加新 provider..."进入子表单
//! - 改：仅用户自定义 provider 可改 base_url / openai_base_url / auth / models_endpoint_path
//! - 删：仅用户自定义可删；删之前提示"已被 N 个 profile 使用"（基于 index 反查）

use std::collections::BTreeSet;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use crate::catalog::{self, AuthScheme, Provider};
use crate::error::Error;
use crate::tui::app::{App, Mode};
use crate::tui::widgets::{InputField, Toast};

#[derive(Debug, Default)]
pub struct State {
    pub list: ListState,
    pub form: Option<ProviderForm>,
    pub confirm_delete: Option<String>,
}

#[derive(Debug)]
pub struct ProviderForm {
    pub editing_id: Option<String>, // None=add, Some=edit existing user provider
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
        let mut f = Self::new_add();
        f.editing_id = Some(p.id.clone());
        f.fields[0] = InputField::new("id").with_initial(&p.id);
        f.fields[1] = InputField::new("display_name").with_initial(&p.display_name);
        f.fields[2] = InputField::new("anthropic_base_url")
            .with_initial(p.anthropic_base_url.as_deref().unwrap_or(""));
        f.fields[3] = InputField::new("openai_base_url")
            .with_initial(p.openai_base_url.as_deref().unwrap_or(""));
        f.fields[4] = InputField::new("models_endpoint_path").with_initial(&p.models_endpoint_path);
        f
    }
    /// 把表单序列化成 user toml 段。
    pub fn to_toml_section(&self) -> Result<(String, String), Error> {
        let id = self.fields[0].value().trim().to_string();
        if id.is_empty() {
            return Err(Error::InvalidKeyId {
                id: id.clone(),
                reason: "provider id 不能为空".into(),
            });
        }
        let mut body = String::new();
        let display_name = self.fields[1].value().trim();
        let anth = self.fields[2].value().trim();
        let oa = self.fields[3].value().trim();
        let models = self.fields[4].value().trim();

        if !display_name.is_empty() {
            body.push_str(&format!("display_name = \"{display_name}\"\n"));
        }
        if !anth.is_empty() {
            body.push_str(&format!("anthropic_base_url = \"{anth}\"\n"));
        }
        if !oa.is_empty() {
            body.push_str(&format!("openai_base_url = \"{oa}\"\n"));
        }
        // V1 默认 Bearer；后续若需要 X-Api-Key，提供 dropdown（M5 之后再加）
        body.push_str("auth = \"Bearer\"\n");
        body.push_str(&format!("models_endpoint_path = \"{models}\"\n"));
        Ok((id, body))
    }
}

impl State {
    pub fn is_in_input_mode(&self) -> bool {
        self.form.is_some()
    }
}

pub fn handle_key(app: &mut App, k: KeyEvent) {
    let Mode::Providers(state) = &mut app.mode else { return };

    // 在表单中：键被 form 接管
    if let Some(form) = &mut state.form {
        handle_form_key(app, form, k);
        return;
    }

    // 在确认删除对话框中
    if let Some(id) = state.confirm_delete.clone() {
        if k.code == KeyCode::Char('y') {
            match remove_user_provider(app, &id) {
                Ok(()) => app.set_toast(Toast::success(format!("provider `{id}` removed"))),
                Err(e) => app.set_toast(Toast::error(format!("remove failed: {e}"))),
            }
            if let Mode::Providers(s) = &mut app.mode {
                s.confirm_delete = None;
            }
        } else if matches!(k.code, KeyCode::Char('n') | KeyCode::Esc) {
            if let Mode::Providers(s) = &mut app.mode {
                s.confirm_delete = None;
            }
        }
        return;
    }

    // 列表导航 + a/e/x/Esc
    let total = app.providers.len() + 1; // 末位是"+ 添加新 provider..."
    match k.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = Mode::Profiles(crate::tui::views::profiles::State::new(&app.index));
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let i = state.list.selected().unwrap_or(0);
            let next = (i + 1).min(total - 1);
            state.list.select(Some(next));
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let i = state.list.selected().unwrap_or(0);
            state.list.select(Some(i.saturating_sub(1)));
        }
        KeyCode::Enter | KeyCode::Char('a') => {
            let i = state.list.selected().unwrap_or(0);
            if i == app.providers.len() {
                state.form = Some(ProviderForm::new_add());
            }
        }
        KeyCode::Char('e') => {
            if let Some(p) = currently_selected_user_provider(app, state) {
                state.form = Some(ProviderForm::from_existing(&p));
            } else {
                app.set_toast(Toast::info("内置 provider 不可编辑"));
            }
        }
        KeyCode::Char('x') => {
            if let Some(p) = currently_selected_user_provider(app, state) {
                state.confirm_delete = Some(p.id.clone());
            } else {
                app.set_toast(Toast::info("内置 provider 不可删除"));
            }
        }
        _ => {}
    }
}

fn currently_selected_user_provider(app: &App, state: &State) -> Option<Provider> {
    let i = state.list.selected()?;
    if i >= app.providers.len() {
        return None;
    }
    let p = &app.providers[i];
    let builtin_ids: BTreeSet<String> = catalog::builtins().into_iter().map(|p| p.id).collect();
    if builtin_ids.contains(&p.id) {
        // 用户 override 内置仍可编辑——但 V1 简化为：只允许编辑非内置 id
        // （M5 之后再放开 override 编辑）
        None
    } else {
        Some(p.clone())
    }
}

fn handle_form_key(app: &mut App, form: &mut ProviderForm, k: KeyEvent) {
    match k.code {
        KeyCode::Esc => {
            if let Mode::Providers(s) = &mut app.mode {
                s.form = None;
            }
        }
        KeyCode::Tab => {
            form.focus = (form.focus + 1) % form.fields.len();
        }
        KeyCode::BackTab => {
            form.focus = (form.focus + form.fields.len() - 1) % form.fields.len();
        }
        KeyCode::Backspace => {
            form.fields[form.focus].pop();
        }
        KeyCode::Enter => {
            // 最后一行回车 → 提交；其他行 → 下一行
            if form.focus + 1 < form.fields.len() {
                form.focus += 1;
            } else {
                match commit_provider_form(app, form) {
                    Ok(id) => {
                        if let Mode::Providers(s) = &mut app.mode {
                            s.form = None;
                        }
                        app.set_toast(Toast::success(format!("provider `{id}` saved")));
                    }
                    Err(e) => app.set_toast(Toast::error(format!("save failed: {e}"))),
                }
            }
        }
        KeyCode::Char(c) => {
            form.fields[form.focus].push(c);
        }
        _ => {}
    }
}

fn commit_provider_form(app: &mut App, form: &ProviderForm) -> Result<String, Error> {
    let (id, body) = form.to_toml_section()?;
    write_user_provider(&app.paths, &id, &body)?;
    app.reload_providers()?;
    Ok(id)
}

fn write_user_provider(
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
    // 简化策略：删旧段 + append 新段
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
/// 不依赖 toml 库重新序列化（避免破坏用户手写注释；本 V1 决策）。
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
    let Mode::Providers(state) = &app.mode else { return };
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

    if let Some(form) = &state.form {
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
    let mut lines: Vec<ratatui::text::Line<'_>> = Vec::new();
    for (i, field) in form.fields.iter().enumerate() {
        lines.push(field.render(i == form.focus).into());
    }
    let title = if form.editing_id.is_some() {
        "Edit user provider"
    } else {
        "Add user provider"
    };
    let p = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::White));
    frame.render_widget(p, area);
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
```

- [ ] **Step 2: 跑测试 + 三件套**

Run: `cargo test --lib tui::views::providers`
Expected: 3 tests pass。

Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -10`
Expected: 全绿。

- [ ] **Step 3: Commit**

```bash
git add src/tui/views/providers.rs
git commit -m "feat(tui): providers panel — list + add/edit/delete user providers"
```

---

### Task 10: `tui/views/keys.rs` —— Keys 面板（含 rotate 联动）

**Files:**
- Modify: `src/tui/views/keys.rs`

**对 spec 的引用：** §6.4 key 编辑/轮换 UX。改 value → 自动重算脱敏 id → 重命名 section → 改 index → 重渲染所有引用 settings.json。

- [ ] **Step 1: 整体重写 `src/tui/views/keys.rs`**

```rust
//! Keys 面板：跨 provider 列出 credentials.toml 中所有 key；增加 / 改 value / 改 note / 删除。
//!
//! 关键交互（spec §6.4）：改 value 触发 ais 自动：
//! 1. 重算脱敏 id（可能与原 id 一致也可能不同）
//! 2. 重命名 credentials.toml section
//! 3. profile::rename_key_id_in_index 同步索引
//! 4. profile::rotate_key 重渲染所有 settings.json 的 ANTHROPIC_API_KEY

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use crate::credentials::{self, Key};
use crate::error::Error;
use crate::profile;
use crate::tui::app::{App, Mode};
use crate::tui::widgets::{InputField, Toast};

#[derive(Debug, Default)]
pub struct State {
    pub list: ListState,
    pub flat: Vec<KeyRow>,           // 视图中扁平化的所有 (provider, key_id) 行
    pub form: Option<KeyForm>,
    pub confirm_delete: Option<(String, String)>, // (provider, key_id)
}

#[derive(Debug, Clone)]
pub struct KeyRow {
    pub provider: String,
    pub key_id: String,
    pub note: String,
    pub value_redacted: String,
}

#[derive(Debug)]
pub struct KeyForm {
    pub editing: Option<(String, String)>, // (provider, old_key_id) for edit
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
            focus: 1, // 编辑流默认聚焦 value
        }
    }
    fn fields_mut(&mut self) -> [&mut InputField; 4] {
        [&mut self.provider, &mut self.value, &mut self.id_override, &mut self.note]
    }
    fn focused(&mut self) -> &mut InputField {
        match self.focus {
            0 => &mut self.provider,
            1 => &mut self.value,
            2 => &mut self.id_override,
            _ => &mut self.note,
        }
    }
}

impl State {
    pub fn refresh(&mut self, app: &App) {
        self.flat = flatten(app);
        if self.list.selected().is_none() && !self.flat.is_empty() {
            self.list.select(Some(0));
        }
    }
    pub fn is_in_input_mode(&self) -> bool {
        self.form.is_some()
    }
}

fn flatten(app: &App) -> Vec<KeyRow> {
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

fn redact(v: &str) -> String {
    let n = v.chars().count();
    if n <= 8 {
        "*".repeat(n)
    } else {
        let head: String = v.chars().take(4).collect();
        let tail: String = v.chars().rev().take(4).collect::<String>().chars().rev().collect();
        format!("{head}...{tail}")
    }
}

pub fn handle_key(app: &mut App, k: KeyEvent) {
    let Mode::Keys(state) = &mut app.mode else { return };
    if state.flat.is_empty() && state.form.is_none() {
        state.refresh(app);
    }

    if let Some(form) = &mut state.form {
        handle_form_key(app, form, k);
        return;
    }
    if let Some((p, kid)) = state.confirm_delete.clone() {
        if k.code == KeyCode::Char('y') {
            match delete_key(app, &p, &kid) {
                Ok(()) => app.set_toast(Toast::success(format!("key `{p}/{kid}` removed"))),
                Err(e) => app.set_toast(Toast::error(format!("delete failed: {e}"))),
            }
            if let Mode::Keys(s) = &mut app.mode {
                s.confirm_delete = None;
                s.refresh(app);
            }
        } else if matches!(k.code, KeyCode::Char('n') | KeyCode::Esc) {
            if let Mode::Keys(s) = &mut app.mode {
                s.confirm_delete = None;
            }
        }
        return;
    }

    match k.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = Mode::Profiles(crate::tui::views::profiles::State::new(&app.index));
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if !state.flat.is_empty() {
                let i = state.list.selected().unwrap_or(0);
                state.list.select(Some((i + 1).min(state.flat.len() - 1)));
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if !state.flat.is_empty() {
                let i = state.list.selected().unwrap_or(0);
                state.list.select(Some(i.saturating_sub(1)));
            }
        }
        KeyCode::Char('a') | KeyCode::Char('+') => {
            state.form = Some(KeyForm::new_add());
        }
        KeyCode::Char('e') => {
            if let Some(i) = state.list.selected() {
                if let Some(row) = state.flat.get(i).cloned() {
                    if let Some(key) = app
                        .credentials
                        .by_provider
                        .get(&row.provider)
                        .and_then(|m| m.get(&row.key_id))
                    {
                        state.form = Some(KeyForm::from_existing(&row.provider, &row.key_id, key));
                    }
                }
            }
        }
        KeyCode::Char('x') => {
            if let Some(i) = state.list.selected() {
                if let Some(row) = state.flat.get(i) {
                    state.confirm_delete = Some((row.provider.clone(), row.key_id.clone()));
                }
            }
        }
        _ => {}
    }
}

fn handle_form_key(app: &mut App, form: &mut KeyForm, k: KeyEvent) {
    match k.code {
        KeyCode::Esc => {
            if let Mode::Keys(s) = &mut app.mode {
                s.form = None;
            }
        }
        KeyCode::Tab => {
            form.focus = (form.focus + 1) % 4;
        }
        KeyCode::BackTab => {
            form.focus = (form.focus + 3) % 4;
        }
        KeyCode::Backspace => {
            form.focused().pop();
        }
        KeyCode::Enter => {
            if form.focus < 3 {
                form.focus += 1;
            } else {
                match commit_key_form(app, form) {
                    Ok((p, kid)) => {
                        app.set_toast(Toast::success(format!("key `{p}/{kid}` saved")));
                        if let Mode::Keys(s) = &mut app.mode {
                            s.form = None;
                            s.refresh(app);
                        }
                    }
                    Err(e) => app.set_toast(Toast::error(format!("save failed: {e}"))),
                }
            }
        }
        KeyCode::Char(c) => {
            form.focused().push(c);
        }
        _ => {}
    }
}

/// 提交 key 表单（add 或 edit）；驱动 credentials 落盘 + 索引重命名 + 全量 rotate。
fn commit_key_form(app: &mut App, form: &KeyForm) -> Result<(String, String), Error> {
    let provider = form.provider.value().trim().to_string();
    let value = form.value.value().to_string();
    let note = form.note.value().to_string();
    let id_override = form.id_override.value().trim().to_string();

    if provider.is_empty() {
        return Err(Error::InvalidKeyId {
            id: String::new(),
            reason: "provider 不能为空".into(),
        });
    }
    if value.is_empty() {
        return Err(Error::InvalidKeyId {
            id: String::new(),
            reason: "value 不能为空".into(),
        });
    }

    // 计算新 id
    let new_id = if !id_override.is_empty() {
        credentials::validate_id(&id_override)?;
        id_override
    } else {
        // auto；如果撞车，扩展（在该 provider 下）
        let existing: Vec<String> = app
            .credentials
            .by_provider
            .get(&provider)
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default();
        // 排除 edit 的旧 id（避免与自己撞）
        let existing: Vec<String> = match &form.editing {
            Some((p, old)) if p == &provider => existing.into_iter().filter(|x| x != old).collect(),
            _ => existing,
        };
        credentials::unique_id(&value, &existing)?
    };

    let key = Key { value: value.clone(), note };

    // 落盘 credentials
    let map = app.credentials.by_provider.entry(provider.clone()).or_default();
    if let Some((old_p, old_id)) = &form.editing {
        if old_p == &provider {
            map.remove(old_id);
        }
    }
    map.insert(new_id.clone(), key);
    credentials::save(&app.paths.credentials(), &app.credentials)?;

    // 同步索引 + 重渲染
    if let Some((old_p, old_id)) = &form.editing {
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

pub fn draw(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let Mode::Keys(state) = &app.mode else { return };
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

    if let Some(form) = &state.form {
        draw_form(frame, chunks[1], form);
    } else if let Some((p, kid)) = &state.confirm_delete {
        crate::tui::widgets::draw_confirm(
            frame,
            chunks[1],
            "Delete key",
            &format!("将删除 `{p}/{kid}`。引用此 key 的 profile 启动后会鉴权失败。继续？"),
        );
    } else {
        draw_list(frame, chunks[1], state);
    }
    frame.render_widget(Paragraph::new("[Esc] back"), chunks[2]);
}

fn draw_list(frame: &mut Frame<'_>, area: Rect, state: &State) {
    let items: Vec<ListItem<'_>> = state
        .flat
        .iter()
        .map(|r| {
            ListItem::new(format!(
                "{}  {}  {}  ({})",
                r.provider, r.key_id, r.value_redacted, r.note
            ))
        })
        .collect();
    let mut ls = state.list.clone();
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Keys"))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(list, area, &mut ls);
}

fn draw_form(frame: &mut Frame<'_>, area: Rect, form: &KeyForm) {
    let lines = vec![
        form.provider.render(form.focus == 0).into(),
        form.value.render(form.focus == 1).into(),
        form.id_override.render(form.focus == 2).into(),
        form.note.render(form.focus == 3).into(),
    ];
    let title = if form.editing.is_some() {
        "Edit key (changing value re-renders all referencing profiles)"
    } else {
        "Add key"
    };
    let p = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: false });
    frame.render_widget(p, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn flatten_orders_by_provider_then_id() {
        let mut store = credentials::Store::default();
        let mut ds = BTreeMap::new();
        ds.insert("kb".into(), Key { value: "1234567890123".into(), note: "".into() });
        ds.insert("ka".into(), Key { value: "abcdefghij1234".into(), note: "".into() });
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
```

- [ ] **Step 2: 跑测试**

Run: `cargo test --lib tui::views::keys`
Expected: 3 tests pass。

- [ ] **Step 3: 三件套自检**

Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -10`
Expected: 全绿。

- [ ] **Step 4: Commit**

```bash
git add src/tui/views/keys.rs
git commit -m "feat(tui): keys panel — add/edit/delete + value-edit triggers rotate_key"
```

---

### Task 11: `tui/views/wizard.rs` —— 5 步状态机骨架（step 1/4/5；不含 HTTP / add-子流程）

**Files:**
- Modify: `src/tui/views/wizard.rs`

本 task 落地 wizard 的整体状态机 + step 1（选 provider，从已有列表选）+ step 4（起名）+ step 5（preview/commit）。step 2 假定 provider 下已有 key 可选；step 3 模型暂用自由输入（Task 12 加 HTTP）；add-provider / add-key 子流程留 Task 13。

- [ ] **Step 1: 重写 `src/tui/views/wizard.rs`**

```rust
//! 创建/编辑 profile 向导（spec §7）。
//!
//! 5 步：选 provider → 选 key → 选 model → 起名 → preview & commit。
//! 内联子流程：add-provider（Task 13）/ add-key（Task 13）。
//! 模型拉取（Task 12）：当前 step 3 简化为自由输入。

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use crate::catalog::Provider;
use crate::error::Error;
use crate::profile::{self, CreateInput};
use crate::settings::Settings;
use crate::tui::app::{App, Mode};
use crate::tui::widgets::{InputField, Toast};

#[derive(Debug)]
pub struct State {
    pub step: Step,
    pub locked_name: Option<String>, // 编辑模式下锁住 name
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
        Self {
            step: Step::Provider,
            locked_name: None,
            picked_provider: None,
            picked_key_id: None,
            picked_key_value: None,
            picked_model: None,
            provider_list: {
                let mut l = ListState::default();
                l.select(Some(0));
                l
            },
            key_list: {
                let mut l = ListState::default();
                l.select(Some(0));
                l
            },
            model_input: InputField::new("model").with_max_len(128),
            name_input: InputField::new("name").with_max_len(64),
        }
    }
}

impl State {
    pub fn is_in_input_mode(&self) -> bool {
        matches!(self.step, Step::Model | Step::Name)
    }

    pub fn for_edit(name: &str, app: &App) -> Self {
        let mut s = Self::default();
        s.locked_name = Some(name.into());
        if let Some(entry) = app.index.entries.get(name) {
            // 选中匹配的 provider / key / model
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
            // 关键：从 credentials 把 key value 也带上，否则 commit 时写出空 key
            if let Some(map) = app.credentials.by_provider.get(&entry.provider) {
                if let Some(k) = map.get(&entry.key_id) {
                    s.picked_key_value = Some(k.value.clone());
                }
            }
            s.picked_model = Some(entry.model.clone());
            s.model_input = InputField::new("model")
                .with_max_len(128)
                .with_initial(&entry.model);
            s.name_input = InputField::new("name")
                .with_max_len(64)
                .with_initial(name);
        }
        s
    }
}

pub fn handle_key(app: &mut App, k: KeyEvent) {
    let Mode::Wizard(state) = &mut app.mode else { return };
    match state.step {
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
    let Mode::Wizard(state) = &mut app.mode else { return };
    let n = app.providers.len();
    match k.code {
        KeyCode::Esc => back_to_profiles(app),
        KeyCode::Down | KeyCode::Char('j') => {
            let i = state.provider_list.selected().unwrap_or(0);
            state.provider_list.select(Some((i + 1).min(n.saturating_sub(1))));
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let i = state.provider_list.selected().unwrap_or(0);
            state.provider_list.select(Some(i.saturating_sub(1)));
        }
        KeyCode::Enter => {
            let i = state.provider_list.selected().unwrap_or(0);
            if i < n {
                state.picked_provider = Some(app.providers[i].clone());
                state.step = Step::Key;
                state.key_list.select(Some(0));
            }
            // 末尾"+ 添加新 provider..."由 Task 13 接入；当前简化为不响应
        }
        _ => {}
    }
}

fn handle_step_key(app: &mut App, k: KeyEvent) {
    let Mode::Wizard(state) = &mut app.mode else { return };
    let provider = match &state.picked_provider {
        Some(p) => p.clone(),
        None => {
            state.step = Step::Provider;
            return;
        }
    };
    let keys: Vec<(String, String)> = app
        .credentials
        .by_provider
        .get(&provider.id)
        .map(|m| m.iter().map(|(id, k)| (id.clone(), k.value.clone())).collect())
        .unwrap_or_default();
    let n = keys.len();
    match k.code {
        KeyCode::Esc => state.step = Step::Provider,
        KeyCode::Down | KeyCode::Char('j') => {
            let i = state.key_list.selected().unwrap_or(0);
            state.key_list.select(Some((i + 1).min(n.saturating_sub(1))));
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let i = state.key_list.selected().unwrap_or(0);
            state.key_list.select(Some(i.saturating_sub(1)));
        }
        KeyCode::Enter => {
            let i = state.key_list.selected().unwrap_or(0);
            if i < n {
                let (kid, val) = keys[i].clone();
                state.picked_key_id = Some(kid);
                state.picked_key_value = Some(val);
                state.step = Step::Model;
                if state.model_input.value().is_empty() {
                    state.model_input.clear();
                }
            } else {
                // 末尾"+ 添加新 key..."由 Task 13 接入；当前提示
                app.set_toast(Toast::info("add-key 子流程将在 Task 13 接入"));
            }
        }
        _ => {}
    }
}

fn handle_step_model(app: &mut App, k: KeyEvent) {
    let Mode::Wizard(state) = &mut app.mode else { return };
    match k.code {
        KeyCode::Esc => state.step = Step::Key,
        KeyCode::Backspace => state.model_input.pop(),
        KeyCode::Enter => {
            let m = state.model_input.value().trim().to_string();
            if !m.is_empty() {
                state.picked_model = Some(m);
                state.step = Step::Name;
                if state.name_input.value().is_empty() {
                    if let (Some(p), Some(model)) =
                        (state.picked_provider.as_ref(), state.picked_model.as_ref())
                    {
                        let suggested = profile::suggested_name(&p.id, model);
                        state.name_input = InputField::new("name")
                            .with_max_len(64)
                            .with_initial(&suggested);
                    }
                }
            } else {
                app.set_toast(Toast::error("model 不能为空"));
            }
        }
        KeyCode::Char(c) => state.model_input.push(c),
        _ => {}
    }
}

fn handle_step_name(app: &mut App, k: KeyEvent) {
    let Mode::Wizard(state) = &mut app.mode else { return };
    if state.locked_name.is_some() {
        // 编辑模式：name 不可改
        match k.code {
            KeyCode::Esc => state.step = Step::Model,
            KeyCode::Enter => state.step = Step::Preview,
            _ => {}
        }
        return;
    }
    match k.code {
        KeyCode::Esc => state.step = Step::Model,
        KeyCode::Backspace => state.name_input.pop(),
        KeyCode::Enter => {
            let name = state.name_input.value().trim().to_string();
            match profile::validate_name(&name) {
                Ok(()) => {
                    if app.index.entries.contains_key(&name) {
                        // 重名 → 用 suggested_name_with_key 自动换建议
                        if let (Some(p), Some(m), Some(kid)) = (
                            state.picked_provider.as_ref(),
                            state.picked_model.as_ref(),
                            state.picked_key_id.as_ref(),
                        ) {
                            let s = profile::suggested_name_with_key(&p.id, m, kid);
                            state.name_input = InputField::new("name")
                                .with_max_len(64)
                                .with_initial(&s);
                            app.set_toast(Toast::info("name 重复，已自动换建议"));
                        }
                    } else {
                        state.step = Step::Preview;
                    }
                }
                Err(e) => app.set_toast(Toast::error(e.to_string())),
            }
        }
        KeyCode::Char(c) => state.name_input.push(c),
        _ => {}
    }
}

fn handle_step_preview(app: &mut App, k: KeyEvent) {
    match k.code {
        KeyCode::Esc => {
            if let Mode::Wizard(s) = &mut app.mode {
                s.step = Step::Name;
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

fn commit_wizard(app: &mut App) -> Result<String, Error> {
    let Mode::Wizard(state) = &mut app.mode else {
        return Err(Error::ProfileNotFound { name: "<wizard>".into() });
    };
    let provider = state.picked_provider.clone().ok_or(Error::ProviderNotFound {
        id: "<unset>".into(),
    })?;
    let url = provider
        .anthropic_base_url
        .clone()
        .ok_or(Error::ProviderMissingAnthropicUrl {
            id: provider.id.clone(),
        })?;
    let key_id = state.picked_key_id.clone().unwrap_or_default();
    let key_value = state.picked_key_value.clone().unwrap_or_default();
    let model = state.picked_model.clone().unwrap_or_default();
    let name = state
        .locked_name
        .clone()
        .unwrap_or_else(|| state.name_input.value().to_string());
    profile::create(
        &app.paths,
        CreateInput {
            name: &name,
            provider_id: &provider.id,
            key_id: &key_id,
            model: &model,
            anthropic_base_url: &url,
            api_key_value: &key_value,
        },
    )?;
    app.reload_index()?;
    Ok(name)
}

pub fn draw(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let Mode::Wizard(state) = &app.mode else { return };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);
    frame.render_widget(Paragraph::new(format!("New/Edit profile — step: {:?}", state.step)), chunks[0]);
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
        .block(Block::default().borders(Borders::ALL).title("Step 1 / 5  Provider"))
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
    let p = Paragraph::new(state.model_input.render(true).to_string())
        .block(Block::default().borders(Borders::ALL).title("Step 3 / 5  Model（自由输入；Task 12 增强为自动拉取）"));
    frame.render_widget(p, area);
}

fn draw_step_name(frame: &mut Frame<'_>, area: Rect, state: &State) {
    let title = if state.locked_name.is_some() {
        "Step 4 / 5  Name（编辑模式 — 已锁定）"
    } else {
        "Step 4 / 5  Name"
    };
    let p = Paragraph::new(state.name_input.render(true).to_string())
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
        model
    );
    let p = Paragraph::new(body)
        .block(Block::default().borders(Borders::ALL).title("Step 5 / 5  Preview"))
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
        let mut s = State::default();
        s.picked_provider = Some(p);
        s.step = Step::Key;
        assert_eq!(s.step, Step::Key);
        assert!(s.picked_provider.is_some());
    }

    #[test]
    fn for_edit_locks_name_and_prefills() {
        // 构造 app + index
        use chrono::Utc;
        use crate::profile::IndexEntry;
        let p = crate::paths::Paths::with_root(std::env::temp_dir().join("ais-edit-pf"));
        let mut app = App {
            paths: p,
            providers: vec![catalog::find("deepseek").unwrap()],
            credentials: Default::default(),
            index: Default::default(),
            mode: Mode::Wizard(State::default()),
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
```

- [ ] **Step 2: 跑测试 + 三件套**

Run: `cargo test --lib tui::views::wizard`
Expected: 3 tests pass。

Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings`
Expected: 全绿。

- [ ] **Step 3: Commit**

```bash
git add src/tui/views/wizard.rs
git commit -m "feat(tui): wizard skeleton — provider/key picker, model+name input, preview commit"
```

---

### Task 12: wizard step 3 — HTTP 拉模型 + 失败回退 + 自定义条目

**Files:**
- Modify: `src/tui/views/wizard.rs`

- [ ] **Step 1: 在 `wizard.rs` 顶部 use 区追加**

```rust
use ratatui::widgets::ListState;
use crate::http;
```

(若已存在的 use 已覆盖，跳过。)

- [ ] **Step 2: 扩展 `State` 增加 model 列表与 fetch 状态**

把 `State` struct 增加字段：

```rust
pub model_list: ListState,
pub model_choices: Vec<String>,    // 拉到的列表；空表示 fetch 失败/未拉过
pub model_fetch_attempted: bool,   // 进入 step 3 时已尝试过拉一次
pub model_use_custom: bool,        // 用户选了"+ 自定义..."
```

`Default::default()` 中初始化：
```rust
model_list: { let mut l = ListState::default(); l.select(Some(0)); l },
model_choices: Vec::new(),
model_fetch_attempted: false,
model_use_custom: false,
```

- [ ] **Step 3: 改 `handle_step_key` 在确认 key 后预拉模型**

把 `handle_step_key` 中 `state.step = Step::Model;` 之后追加：

```rust
state.model_choices.clear();
state.model_fetch_attempted = false;
state.model_use_custom = false;
state.model_list.select(Some(0));
// fetch 同步阻塞 5 秒；失败/超时 → use_custom = true 走自由输入
let provider = state.picked_provider.clone().unwrap();
let bearer = state.picked_key_value.clone();
match http::fetch_models(
    provider.openai_base_url.as_deref().unwrap_or(""),
    &provider.models_endpoint_path,
    bearer.as_deref(),
) {
    Ok(list) if !list.is_empty() => {
        state.model_choices = list;
    }
    _ => {
        state.model_use_custom = true;
    }
}
state.model_fetch_attempted = true;
```

- [ ] **Step 4: 重写 `handle_step_model` 同时支持列表与自由输入**

```rust
fn handle_step_model(app: &mut App, k: KeyEvent) {
    let Mode::Wizard(state) = &mut app.mode else { return };

    if state.model_use_custom {
        // 自由输入模式
        match k.code {
            KeyCode::Esc => state.step = Step::Key,
            KeyCode::Backspace => state.model_input.pop(),
            KeyCode::Enter => {
                let m = state.model_input.value().trim().to_string();
                if !m.is_empty() {
                    state.picked_model = Some(m);
                    advance_to_name(state);
                } else {
                    app.set_toast(Toast::error("model 不能为空"));
                }
            }
            KeyCode::Char(c) => state.model_input.push(c),
            _ => {}
        }
        return;
    }

    // 列表模式：last item = "+ 自定义..."
    let n = state.model_choices.len();
    match k.code {
        KeyCode::Esc => state.step = Step::Key,
        KeyCode::Down | KeyCode::Char('j') => {
            let i = state.model_list.selected().unwrap_or(0);
            state.model_list.select(Some((i + 1).min(n)));  // n 而非 n-1：包含自定义
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
}

fn advance_to_name(state: &mut State) {
    state.step = Step::Name;
    if state.name_input.value().is_empty() {
        if let (Some(p), Some(model)) =
            (state.picked_provider.as_ref(), state.picked_model.as_ref())
        {
            let suggested = profile::suggested_name(&p.id, model);
            state.name_input = InputField::new("name")
                .with_max_len(64)
                .with_initial(&suggested);
        }
    }
}
```

- [ ] **Step 5: 重写 `draw_step_model` 同时支持两种模式**

```rust
fn draw_step_model(frame: &mut Frame<'_>, area: Rect, state: &State) {
    if state.model_use_custom {
        let para = Paragraph::new(format!(
            "{}\n\n（fetch 失败 / openai_base_url 缺失，转自由输入）",
            state.model_input.render(true)
        ))
        .block(Block::default().borders(Borders::ALL).title("Step 3 / 5  Model（自由输入）"))
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
        .block(Block::default().borders(Borders::ALL).title("Step 3 / 5  Model"))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(list, area, &mut ls);
}
```

- [ ] **Step 6: 跑测试 + 三件套**

Run: `cargo test --lib`
Expected: 全绿（http + wizard + 全部已有）。

Run: `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -10`
Expected: 全绿。

- [ ] **Step 7: Commit**

```bash
git add src/tui/views/wizard.rs
git commit -m "feat(tui): wizard step 3 fetches /v1/models with 5s timeout + custom fallback"
```

---

### Task 13: wizard 内联 add-provider / add-key 子流程

**Files:**
- Modify: `src/tui/views/wizard.rs`
- Modify: `src/tui/views/providers.rs`（pub `ProviderForm` / `write_user_provider` / `draw_form`）
- Modify: `src/tui/views/keys.rs`（pub `KeyForm` 字段与方法 / `draw_form`）

spec §7 Step 1 / Step 2 末尾的"+ 添加新 provider..." / "+ 添加新 key..." 进入子表单，完成后回到原 step 并自动选中新建项。

> **执行顺序提示**：本 task 的 Step 5（把 providers/keys 的 form 类型与渲染函数 pub 出去）实际上是 Step 1-4 编译通过的前提。**先执行 Step 5**，再回头做 Step 1-4。本文档按"语义先后"排列，但实施时优先解决可见性即可。

- [ ] **Step 1: `State` 追加 subflow 字段**

```rust
pub provider_form: Option<crate::tui::views::providers::ProviderForm>,
pub key_form: Option<crate::tui::views::keys::KeyForm>,
```

`Default::default()` 中：
```rust
provider_form: None,
key_form: None,
```

- [ ] **Step 2: 在 step 1 / step 2 的 Enter 末项分支替换"占位 toast"为打开子表单**

修改 `handle_step_provider` 中 `Enter` 分支：

```rust
KeyCode::Enter => {
    let i = state.provider_list.selected().unwrap_or(0);
    if i < n {
        state.picked_provider = Some(app.providers[i].clone());
        state.step = Step::Key;
        state.key_list.select(Some(0));
    } else {
        // "+ 添加新 provider..."
        state.provider_form = Some(crate::tui::views::providers::ProviderForm::new_add());
    }
}
```

`handle_step_key` 中末项分支：

```rust
} else {
    // "+ 添加新 key..." → 子表单
    let provider_id = provider.id.clone();
    let mut form = crate::tui::views::keys::KeyForm::new_add();
    form.provider = InputField::new("provider").with_initial(&provider_id);
    state.key_form = Some(form);
}
```

- [ ] **Step 3: 在 `handle_key` 顶层拦截子表单事件**

`handle_key` 第一行替换为：

```rust
pub fn handle_key(app: &mut App, k: KeyEvent) {
    {
        let Mode::Wizard(state) = &mut app.mode else { return };
        if state.provider_form.is_some() {
            handle_provider_subform_key(app, k);
            return;
        }
        if state.key_form.is_some() {
            handle_key_subform_key(app, k);
            return;
        }
    }
    let Mode::Wizard(state) = &mut app.mode else { return };
    match state.step { /* ... 与原版一致 ... */ }
}
```

- [ ] **Step 4: 实现两个 subform 处理函数**

在文件末尾追加：

```rust
fn handle_provider_subform_key(app: &mut App, k: KeyEvent) {
    let Mode::Wizard(state) = &mut app.mode else { return };
    let form = state.provider_form.as_mut().unwrap();
    match k.code {
        KeyCode::Esc => {
            state.provider_form = None;
        }
        KeyCode::Tab => form.focus = (form.focus + 1) % form.fields.len(),
        KeyCode::BackTab => form.focus = (form.focus + form.fields.len() - 1) % form.fields.len(),
        KeyCode::Backspace => form.fields[form.focus].pop(),
        KeyCode::Enter => {
            if form.focus + 1 < form.fields.len() {
                form.focus += 1;
            } else {
                let snapshot = form.to_toml_section();
                drop(form);
                match snapshot {
                    Ok((id, body)) => {
                        if let Err(e) = crate::tui::views::providers::write_user_provider(
                            &app.paths, &id, &body,
                        ) {
                            app.set_toast(Toast::error(format!("save failed: {e}")));
                            return;
                        }
                        if let Err(e) = app.reload_providers() {
                            app.set_toast(Toast::error(format!("reload failed: {e}")));
                            return;
                        }
                        // 选中新建的 provider
                        if let Mode::Wizard(s) = &mut app.mode {
                            if let Some(idx) =
                                app.providers.iter().position(|p| p.id == id)
                            {
                                s.provider_list.select(Some(idx));
                                s.picked_provider = Some(app.providers[idx].clone());
                            }
                            s.provider_form = None;
                            s.step = Step::Key;
                            s.key_list.select(Some(0));
                        }
                        app.set_toast(Toast::success(format!("provider `{id}` created")));
                    }
                    Err(e) => app.set_toast(Toast::error(format!("invalid: {e}"))),
                }
            }
        }
        KeyCode::Char(c) => form.fields[form.focus].push(c),
        _ => {}
    }
}

fn handle_key_subform_key(app: &mut App, k: KeyEvent) {
    let Mode::Wizard(state) = &mut app.mode else { return };
    let form = state.key_form.as_mut().unwrap();
    match k.code {
        KeyCode::Esc => state.key_form = None,
        KeyCode::Tab => form.focus = (form.focus + 1) % 4,
        KeyCode::BackTab => form.focus = (form.focus + 3) % 4,
        KeyCode::Backspace => form.focused().pop(),
        KeyCode::Enter => {
            if form.focus < 3 {
                form.focus += 1;
            } else {
                let provider_id = form.provider.value().trim().to_string();
                let value = form.value.value().to_string();
                let id_override = form.id_override.value().trim().to_string();
                let note = form.note.value().to_string();

                // 计算新 id
                let existing: Vec<String> = app
                    .credentials
                    .by_provider
                    .get(&provider_id)
                    .map(|m| m.keys().cloned().collect())
                    .unwrap_or_default();
                let new_id = if !id_override.is_empty() {
                    if let Err(e) = crate::credentials::validate_id(&id_override) {
                        app.set_toast(Toast::error(e.to_string()));
                        return;
                    }
                    id_override
                } else {
                    match crate::credentials::unique_id(&value, &existing) {
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
                    crate::credentials::Key { value: value.clone(), note },
                );
                if let Err(e) =
                    crate::credentials::save(&app.paths.credentials(), &app.credentials)
                {
                    app.set_toast(Toast::error(format!("save failed: {e}")));
                    return;
                }
                if let Mode::Wizard(s) = &mut app.mode {
                    s.picked_key_id = Some(new_id.clone());
                    s.picked_key_value = Some(value);
                    s.key_form = None;
                    s.step = Step::Model;
                }
                app.set_toast(Toast::success(format!("key `{provider_id}/{new_id}` created")));
            }
        }
        KeyCode::Char(c) => form.focused().push(c),
        _ => {}
    }
}
```

- [ ] **Step 5: 让 `providers::write_user_provider` 与 `keys::KeyForm` 公开（pub）**

修改 `src/tui/views/providers.rs` 的 `write_user_provider` 函数签名，前缀加 `pub`。

修改 `src/tui/views/keys.rs` 的 `KeyForm` struct 及其 `new_add` 方法、`fields_mut`、`focused`、所有字段全部 `pub`。

- [ ] **Step 6: 在 `draw` 里渲染子表单**

修改 `wizard.rs` 的 `draw`：

```rust
pub fn draw(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let Mode::Wizard(state) = &app.mode else { return };
    if let Some(form) = &state.provider_form {
        crate::tui::views::providers::draw_form_external(frame, area, form);
        return;
    }
    if let Some(form) = &state.key_form {
        crate::tui::views::keys::draw_form_external(frame, area, form);
        return;
    }
    // 原 5 步分派 ...
}
```

并在 `providers.rs` / `keys.rs` 末尾分别 `pub fn draw_form_external(frame, area, form: &ProviderForm)` / 同名 KeyForm 版本，逻辑就是把内部 `draw_form` 提出来 pub 出去（或直接把 `draw_form` 改 `pub`，二者等价；这里推荐直接改 `pub fn draw_form` 重命名为 `pub fn draw_form_external`，避免歧义）。

- [ ] **Step 7: 跑全部测试 + 三件套**

Run: `cargo test --lib`
Expected: 全绿。

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: 全绿。

- [ ] **Step 8: Commit**

```bash
git add src/tui/views/
git commit -m "feat(tui): wizard inline add-provider / add-key subflows per spec §7"
```

---

### Task 14: profiles 视图接 `n` `e` `r` `x` `Enter`

**Files:**
- Modify: `src/tui/views/profiles.rs`
- Modify: `src/tui/app.rs`

> **执行顺序提示**：先做 Step 2（给 `State` 加 `rename_input` / `renaming_from` / `confirm_delete` 字段），再做 Step 1（往 `handle_key` 里加引用这些字段的分支），否则中间编译会红。文档按"键绑定语义"排序便于阅读。

- [ ] **Step 1: 在 `profiles.rs::handle_key` 的 `_ => {}` 之前补这些分支**

```rust
KeyCode::Char('n') => {
    app.mode = Mode::Wizard(crate::tui::views::wizard::State::default());
}
KeyCode::Char('e') => {
    if let Some(name) = state.selected_name().map(str::to_string) {
        app.mode = Mode::Wizard(crate::tui::views::wizard::State::for_edit(&name, app));
    }
}
KeyCode::Char('r') => {
    if let Some(name) = state.selected_name().map(str::to_string) {
        // 简化版重命名：弹底栏输入框
        state.rename_input = Some(crate::tui::widgets::InputField::new("rename").with_initial(&name));
        state.renaming_from = Some(name);
    }
}
KeyCode::Char('x') => {
    if let Some(name) = state.selected_name().map(str::to_string) {
        state.confirm_delete = Some(name);
    }
}
KeyCode::Enter => {
    if let Some(name) = state.selected_name().map(str::to_string) {
        app.launch_target = Some(name);
        app.running = false;
    }
}
```

- [ ] **Step 2: `State` 增加 rename_input / renaming_from / confirm_delete**

```rust
#[derive(Debug, Default)]
pub struct State {
    pub list: ListState,
    pub names: Vec<String>,
    pub rename_input: Option<crate::tui::widgets::InputField>,
    pub renaming_from: Option<String>,
    pub confirm_delete: Option<String>,
}
```

`is_in_input_mode`：
```rust
pub fn is_in_input_mode(&self) -> bool {
    self.rename_input.is_some()
}
```

- [ ] **Step 3: rename_input / confirm_delete 事件处理**

把 `handle_key` 头部加：

```rust
pub fn handle_key(app: &mut App, k: KeyEvent) {
    let Mode::Profiles(state) = &mut app.mode else { return };
    if let Some(field) = state.rename_input.as_mut() {
        match k.code {
            KeyCode::Esc => {
                state.rename_input = None;
                state.renaming_from = None;
            }
            KeyCode::Backspace => field.pop(),
            KeyCode::Enter => {
                let new_name = field.value().trim().to_string();
                let from = state.renaming_from.clone().unwrap_or_default();
                state.rename_input = None;
                state.renaming_from = None;
                match rename_profile(app, &from, &new_name) {
                    Ok(()) => app.set_toast(crate::tui::widgets::Toast::success(format!(
                        "renamed `{from}` → `{new_name}`"
                    ))),
                    Err(e) => app.set_toast(crate::tui::widgets::Toast::error(format!(
                        "rename failed: {e}"
                    ))),
                }
                if let Mode::Profiles(s) = &mut app.mode {
                    *s = State::new(&app.index);
                }
                return;
            }
            KeyCode::Char(c) => field.push(c),
            _ => {}
        }
        return;
    }
    if let Some(name) = state.confirm_delete.clone() {
        if k.code == KeyCode::Char('y') {
            match crate::profile::delete(&app.paths, &name) {
                Ok(()) => app.set_toast(crate::tui::widgets::Toast::success(format!(
                    "deleted `{name}`"
                ))),
                Err(e) => app.set_toast(crate::tui::widgets::Toast::error(format!(
                    "delete failed: {e}"
                ))),
            }
            let _ = app.reload_index();
            if let Mode::Profiles(s) = &mut app.mode {
                *s = State::new(&app.index);
            }
        } else if matches!(k.code, KeyCode::Char('n') | KeyCode::Esc) {
            if let Mode::Profiles(s) = &mut app.mode {
                s.confirm_delete = None;
            }
        }
        return;
    }
    // 原本的 match k.code { ... }
}
```

- [ ] **Step 4: 实现 rename_profile**

在 `profiles.rs` 末尾新增：

```rust
fn rename_profile(app: &mut App, from: &str, to: &str) -> Result<(), crate::Error> {
    crate::profile::validate_name(to)?;
    if from == to {
        return Ok(());
    }
    if app.index.entries.contains_key(to) {
        return Err(crate::Error::InvalidProfileName {
            name: to.into(),
            reason: "已存在同名 profile".into(),
        });
    }
    let from_path = app.paths.settings_for(from);
    let to_path = app.paths.settings_for(to);
    std::fs::rename(&from_path, &to_path).map_err(|e| crate::Error::Io {
        path: from_path.clone(),
        source: e,
    })?;
    let mut idx = crate::profile::Index::load(&app.paths.claude_index())?;
    if let Some(entry) = idx.entries.remove(from) {
        idx.entries.insert(to.into(), entry);
    }
    idx.save(&app.paths.claude_index())?;
    app.reload_index()?;
    Ok(())
}
```

- [ ] **Step 5: `draw` 中渲染 rename_input / confirm_delete overlay**

在 `draw` 末尾、`draw_bottom` 之后追加：

```rust
if let Some(field) = &state.rename_input {
    let popup = crate::tui::widgets::centered_rect(60, 20, area);
    let p = Paragraph::new(field.render(true).to_string())
        .block(ratatui::widgets::Block::default()
            .borders(ratatui::widgets::Borders::ALL)
            .title("Rename profile"));
    frame.render_widget(ratatui::widgets::Clear, popup);
    frame.render_widget(p, popup);
}
if let Some(name) = &state.confirm_delete {
    crate::tui::widgets::draw_confirm(
        frame,
        area,
        "Delete profile",
        &format!("将删除 settings_{name}.json 与索引项（key 不动）。继续？"),
    );
}
```

- [ ] **Step 6: 跑全量测试 + 三件套**

Run: `cargo test --lib && cargo clippy --workspace --all-targets -- -D warnings`
Expected: 全绿。

- [ ] **Step 7: Commit**

```bash
git add src/tui/views/profiles.rs src/tui/app.rs
git commit -m "feat(tui): profiles n/e/r/x/Enter — wizard, edit, rename, delete, launch"
```

---

### Task 15: `tui/views/doctor.rs` —— Doctor 6 项检查

**Files:**
- Modify: `src/tui/views/doctor.rs`

**对 spec 的引用：** §9.4。仅检测+报告，不自动修复。

- [ ] **Step 1: 重写 `src/tui/views/doctor.rs`**

```rust
//! Doctor 面板（spec §9.4）：6 项检查；只读取，不修复。

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::claude;
use crate::providers;
use crate::tui::app::{App, Mode};

#[derive(Debug, Default)]
pub struct State {
    pub report: Vec<Item>,
    pub computed: bool,
}

#[derive(Debug, Clone)]
pub struct Item {
    pub label: String,
    pub status: Status,
    pub detail: String,
}

#[derive(Debug, Clone, Copy)]
pub enum Status {
    Pass,
    Fail,
}

impl State {
    pub fn is_in_input_mode(&self) -> bool {
        false
    }
}

pub fn handle_key(app: &mut App, k: KeyEvent) {
    let Mode::Doctor(state) = &mut app.mode else { return };
    if !state.computed {
        state.report = compute(app);
        state.computed = true;
        return; // 第一次进 doctor 模式时，先算完再处理键
    }
    if matches!(k.code, KeyCode::Esc | KeyCode::Char('q')) {
        app.mode = Mode::Profiles(crate::tui::views::profiles::State::new(&app.index));
    }
}

fn compute(app: &App) -> Vec<Item> {
    let mut out = Vec::new();
    out.push(check_claude_in_path());
    out.push(check_claude_version());
    out.push(check_root_writable(&app.paths.root));
    out.push(check_credentials_perm(&app.paths.credentials()));
    out.push(check_index_consistency(app));
    out.push(check_providers_loadable(&app.paths.providers()));
    out
}

fn check_claude_in_path() -> Item {
    match claude::probe_path() {
        Ok(p) => Item {
            label: "claude in PATH".into(),
            status: Status::Pass,
            detail: p.display().to_string(),
        },
        Err(e) => Item {
            label: "claude in PATH".into(),
            status: Status::Fail,
            detail: e.to_string(),
        },
    }
}

fn check_claude_version() -> Item {
    match claude::probe_path().and_then(|p| claude::probe_version(&p)) {
        Ok(v) => Item {
            label: "claude --version".into(),
            status: Status::Pass,
            detail: v,
        },
        Err(e) => Item {
            label: "claude --version".into(),
            status: Status::Fail,
            detail: e.to_string(),
        },
    }
}

fn check_root_writable(root: &std::path::Path) -> Item {
    use std::fs;
    let probe = root.join(".doctor-probe");
    let res = (|| -> std::io::Result<()> {
        fs::create_dir_all(root)?;
        fs::write(&probe, b"ok")?;
        fs::remove_file(&probe)?;
        Ok(())
    })();
    match res {
        Ok(()) => Item {
            label: "~/.ai-switch/ writable".into(),
            status: Status::Pass,
            detail: root.display().to_string(),
        },
        Err(e) => Item {
            label: "~/.ai-switch/ writable".into(),
            status: Status::Fail,
            detail: e.to_string(),
        },
    }
}

fn check_credentials_perm(path: &std::path::Path) -> Item {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if !path.exists() {
            return Item {
                label: "credentials.toml mode".into(),
                status: Status::Pass,
                detail: "(absent — will be created at first add)".into(),
            };
        }
        match std::fs::metadata(path) {
            Ok(m) => {
                let mode = m.permissions().mode() & 0o777;
                if mode & 0o077 == 0 {
                    Item {
                        label: "credentials.toml mode".into(),
                        status: Status::Pass,
                        detail: format!("0{mode:o}"),
                    }
                } else {
                    Item {
                        label: "credentials.toml mode".into(),
                        status: Status::Fail,
                        detail: format!("0{mode:o} (expected 0600)"),
                    }
                }
            }
            Err(e) => Item {
                label: "credentials.toml mode".into(),
                status: Status::Fail,
                detail: e.to_string(),
            },
        }
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Item {
            label: "credentials.toml mode".into(),
            status: Status::Pass,
            detail: "(skipped on this platform)".into(),
        }
    }
}

fn check_index_consistency(app: &App) -> Item {
    let dir = app.paths.claude_dir();
    let mut on_disk: Vec<String> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for ent in rd.flatten() {
            let name = ent.file_name().to_string_lossy().to_string();
            if let Some(stripped) = name.strip_prefix("settings_").and_then(|s| s.strip_suffix(".json")) {
                on_disk.push(stripped.to_string());
            }
        }
    }
    let in_index: std::collections::BTreeSet<&str> =
        app.index.entries.keys().map(String::as_str).collect();
    let on_disk_set: std::collections::BTreeSet<&str> =
        on_disk.iter().map(String::as_str).collect();

    let orphans: Vec<&&str> = on_disk_set.difference(&in_index).collect();
    let dangling: Vec<&&str> = in_index.difference(&on_disk_set).collect();

    if orphans.is_empty() && dangling.is_empty() {
        Item {
            label: ".ais-index.toml consistency".into(),
            status: Status::Pass,
            detail: format!("{} entries, {} files; matched", in_index.len(), on_disk.len()),
        }
    } else {
        let mut detail = String::new();
        if !orphans.is_empty() {
            detail.push_str(&format!("orphans (on disk, not in index): {orphans:?}; "));
        }
        if !dangling.is_empty() {
            detail.push_str(&format!("dangling (in index, no file): {dangling:?}"));
        }
        Item {
            label: ".ais-index.toml consistency".into(),
            status: Status::Fail,
            detail,
        }
    }
}

fn check_providers_loadable(path: &std::path::Path) -> Item {
    match providers::load_all(path) {
        Ok(list) => Item {
            label: "providers.toml loadable".into(),
            status: Status::Pass,
            detail: format!("{} providers (builtins + user)", list.len()),
        },
        Err(e) => Item {
            label: "providers.toml loadable".into(),
            status: Status::Fail,
            detail: e.to_string(),
        },
    }
}

pub fn draw(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let Mode::Doctor(state) = &app.mode else { return };
    let body = if state.computed {
        let mut out = String::new();
        for item in &state.report {
            let mark = match item.status {
                Status::Pass => "[OK]",
                Status::Fail => "[FAIL]",
            };
            out.push_str(&format!("{mark} {} — {}\n", item.label, item.detail));
        }
        out
    } else {
        "Computing...".to_string()
    };
    let para = Paragraph::new(body)
        .block(Block::default().borders(Borders::ALL).title("Doctor (d) — [Esc] back"))
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: false });
    frame.render_widget(para, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_writable_passes_for_temp_dir() {
        let p = std::env::temp_dir().join(format!("ais-doctor-{}", std::process::id()));
        std::fs::create_dir_all(&p).unwrap();
        let item = check_root_writable(&p);
        assert!(matches!(item.status, Status::Pass));
        std::fs::remove_dir_all(&p).ok();
    }

    #[test]
    fn root_writable_fails_for_nonexistent_parent() {
        let p = std::path::Path::new("/this/path/should/not/exist/ais-test-xxx");
        let item = check_root_writable(p);
        assert!(matches!(item.status, Status::Fail));
    }

    #[test]
    fn providers_loadable_passes_with_no_user_file() {
        let p = std::env::temp_dir().join(format!(
            "ais-doctor-prov-{}",
            std::process::id()
        ));
        let item = check_providers_loadable(&p.join("providers.toml"));
        assert!(matches!(item.status, Status::Pass));
    }
}
```

- [ ] **Step 2: 跑测试 + 三件套**

Run: `cargo test --lib tui::views::doctor && cargo clippy --workspace --all-targets -- -D warnings`
Expected: 全绿。

- [ ] **Step 3: Commit**

```bash
git add src/tui/views/doctor.rs
git commit -m "feat(tui): doctor panel — 6 checks per spec §9.4"
```

---

### Task 16: 空状态 + 人工验收清单 + push + PR

**Files:** 无代码改动；本任务把累计的功能跑一遍人工验收。

- [ ] **Step 1: 三件套全量自检**

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Expected: 全绿；测试数应当显著高于 Plan A（Plan A 36，Plan B 累积约 50+，具体取决于实际新增）。

- [ ] **Step 2: 空状态人工验证**

```bash
AIS_HOME=$(mktemp -d) cargo run -q
```
Expected: 进入空 Profiles 视图，详情区提示"(no profiles — press [n] to create one)"。按 `n` 进入向导。

- [ ] **Step 3: 端到端创建一个 profile**

延续 Step 2 的 `AIS_HOME`：在向导中选 `deepseek` → `+ 添加新 key...`（输入虚假 key 例如 `sk-aaaaaaaaafswv`，note 留空）→ Step 3 出现网络拉取失败/超时 → 切自由输入"deepseek-chat"→ Step 4 接受默认建议名 → Step 5 回车提交。

Expected: toast 提示 saved；Profiles 主视图列表新出现一项；按 `q` 退出，确认 `${AIS_HOME}/claude/settings_deepseek_deepseek-chat.json` 存在且内容正常。

- [ ] **Step 4: rotate_key 端到端**

```bash
AIS_HOME=<上面 mktemp 的目录> cargo run -q
```
进 `K`（keys 面板）→ 选刚才那个 key → `e` → 改 value 为新的 15+ 字符 → 回车提交。
Expected: toast `key ... saved`；按 `Esc` 回主视图，再用 `Settings::load` 读取上面 settings.json，确认 `ANTHROPIC_API_KEY` 已是新值。

- [ ] **Step 5: launch 路径**

确保 `claude` 在 PATH。在主视图上按 `Enter` → TUI 退出 → 进入 Claude Code（execvp 不返回）。

> 没装 claude 时 toast 报"claude not found in PATH"——这个分支 doctor 也能看到。

- [ ] **Step 6: q 退出 termios 干净**

Expected: 退出后 shell 行为正常（无奇异回显、无 raw mode 残留）。

- [ ] **Step 7: push + 开 PR**

```bash
git push -u origin feature/plan-b-tui
gh pr create --base main --head feature/plan-b-tui \
  --title "ai-switch V1 Plan B — TUI (M3 + M4 + M5)" \
  --body "$(cat <<'EOF'
## 概要
Plan B 落地 TUI：profiles 主视图 / 5 步创建向导（含内联 add-provider/key 子流程 + /v1/models 5s 拉取 + 失败回退）/ keys 面板（含 rotate）/ providers 面板 / doctor 6 项检查 / help overlay。Plan A 留下的"TUI not yet available"占位被替换。

## 行为契约
按 `docs/superpowers/specs/2026-04-29-ai-switch-design.md` §7-§11、§9.4 实现。所有数据层 API（profile / credentials / providers / settings / claude）由 TUI 调用，不绕过；profile::create 增加事务回滚（spec §7 Step 5）。

## 测试
- 单元：~50（Plan A 36 + Plan B http/profile 回滚/widgets/app/profiles/providers/keys/wizard/doctor 共 ~14）
- 集成：launch_smoke.rs 沿用 Plan A，TUI 自身依赖人工验收
- CI：三平台矩阵继承 Plan A
EOF
)"
```

- [ ] **Step 8: CI 三平台 watch**

```bash
gh pr checks --watch
```
Expected: ubuntu / macos / windows 三个 job 全绿。

如有失败，按 Plan A 同样的方法（读 log → 定位平台差异 → 修 → 重 push）。

---

## Plan B 自检 checklist（实施前过一眼）

- ✅ Spec §3-§6（数据契约）/ §7（向导 5 步 + 内联子流程）/ §8（启动）/ §9（主视图与 5 个面板）/ §9.4（doctor 6 项）/ §10（不变量）/ §11（错误展示） 全部映射到 Task 6 / 9 / 10 / 11 / 12 / 13 / 14 / 15。
- ✅ Spec §13 依赖清单 = Plan A 已有 + ratatui/crossterm/ureq（Task 1）；不引入 tokio/keyring/tempfile。
- ✅ Spec §14.1 单元测试：http parser / profile rollback / widgets / app dispatch / profiles render helpers / providers form / keys flatten / wizard state / doctor checks 全部新增。
- ⚠️ Spec §14.2 集成测试：TUI 自身的端到端没自动覆盖（依赖真实 TTY）；通过 Task 16 人工验收 + launch_smoke.rs 错误分支补足。
- ⚠️ Spec §16 待确认项（窄终端布局、profile 列表 >30 个滚动）继续延后，与 spec 一致。
- ⚠️ Spec §5.2 内置 provider 表 URL 校准在发布前 sweep；Plan B 不动该数据。
- ⚠️ Plan C（M6 发布）入口：Plan B 完成后只剩 README 写作 + GitHub Release + cargo install 验证；本仓库可直接 tag v0.1.0-beta。

---

## 执行选择

按 Plan A 同样的节奏：本地按 Task 1-15 顺序逐 commit，本地三件套全绿后 Task 16 push + 开 PR。每个 Task 是 atomic commit，任何一个 task 失败可单独回退或修补，不影响下一个。
