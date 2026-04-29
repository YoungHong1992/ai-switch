# ai-switch V1 Plan A — Foundations + Launch CLI

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 交付可运行的 `ais claude <name>` —— 它读取 `~/.ai-switch/claude/settings_<name>.json` 并 `exec claude --settings <path> [...透传]`。同时落地 catalog / providers / credentials / settings / profile 五个数据层模块、跨平台 CI、错误类型与路径常量，作为 Plan B（TUI）与 Plan C（Doctor + Release）的基石。

**Architecture:** 单 binary + 配套 lib（`name = "ais"`）。所有持久化状态在 `~/.ai-switch/`：`credentials.toml` + `providers.toml`（顶层共享）+ `claude/{settings_*.json, .ais-index.toml}`（Claude 工具子目录）。内置 provider catalog 是编译期常量；用户 `providers.toml` 同名覆盖内置。Settings 严格保持 Claude Code 标准 JSON，不夹任何 ais 私有字段；ais 的 (provider, key, model) 来源信息存到独立的 `.ais-index.toml`。

**Tech Stack:** Rust 2024 edition；`serde` / `serde_json` / `toml` / `thiserror` / `anyhow` / `clap`（CLI dispatch）/ `which`（PATH 探测）/ `directories`（home dir）/ `chrono`（created_at）。Plan A **不引入** `ratatui` / `crossterm` / `ureq` / `keyring` / `tempfile`（这些归 Plan B/C）。

**对 spec 的引用规则：** 模块字段、契约、错误语义以 `docs/superpowers/specs/2026-04-29-ai-switch-design.md` 为准；本 plan 只把 spec 拆成可执行步骤。

---

## 文件结构（Plan A 完成后）

```
ai-switch/
├── Cargo.toml
├── LICENSE
├── README.md
├── .gitignore
├── .github/workflows/ci.yml
├── docs/
│   └── superpowers/
│       ├── specs/2026-04-29-ai-switch-design.md
│       └── plans/2026-04-29-ai-switch-v1-plan-a-foundations.md   ← 本文件
├── src/
│   ├── main.rs        # CLI dispatch（clap）
│   ├── lib.rs         # 模块根
│   ├── paths.rs       # ~/.ai-switch/ 路径常量
│   ├── error.rs       # thiserror Error enum
│   ├── catalog.rs     # 内置 provider 表
│   ├── providers.rs   # 用户 providers.toml 加载与覆盖
│   ├── credentials.rs # credentials.toml 读写、key id 脱敏算法
│   ├── settings.rs    # Claude Code settings.json 渲染/读写
│   ├── profile.rs     # settings_*.json + .ais-index.toml 管理
│   └── claude.rs      # `claude` 二进制探测 + launch（execvp/spawn）
└── tests/
    └── launch_smoke.rs  # 启动路径错误分支集成测试
```

---

### Task 1: Greenfield Cargo manifest + skeleton

**Files:**
- Create: `Cargo.toml`
- Create: `.gitignore`
- Create: `LICENSE`
- Create: `src/main.rs`
- Create: `src/lib.rs`
- Modify: `README.md`

> **关于 Windows 工作机的 stale 代码**：本 plan 在 Linux 仓库（`/home/yanghong/Tools/ai-switch`）上执行；Windows 端 `D:\Tools\DevMux` 还有旧 M1 代码，待 Plan A 推送到 `origin/main` 后用户自行 `git pull --rebase` + `git clean -fdx` 同步。本 plan 不处理 Windows 端清理。

- [ ] **Step 1: 验证清白基线**

Run: `ls -la && find . -name '*.rs' -not -path './.git/*' 2>/dev/null`
Expected: 仅 `README.md`、`docs/`、`.git/`、`.claude/` 存在，无任何 `.rs` 文件。

- [ ] **Step 2: 创建 `Cargo.toml`**

```toml
[package]
name = "ai-switch"
version = "0.1.0"
edition = "2024"
license = "MIT OR Apache-2.0"
description = "Claude Code 配置切换工具：profile = 标准 settings.json 文件，零 lock-in"
repository = "https://github.com/YoungHong1992/ai-switch"

[[bin]]
name = "ais"
path = "src/main.rs"

[lib]
name = "ais"
path = "src/lib.rs"

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

[profile.release]
lto = true
codegen-units = 1
strip = true
```

- [ ] **Step 3: 创建 `.gitignore`**

```
/target
Cargo.lock.bak
*.swp
.DS_Store
```

- [ ] **Step 4: 创建 `src/lib.rs`（模块根；引用尚未存在的子模块，编译会失败，由后续 Task 逐个补齐）**

```rust
pub mod catalog;
pub mod claude;
pub mod credentials;
pub mod error;
pub mod paths;
pub mod profile;
pub mod providers;
pub mod settings;

pub use error::Error;

pub type Result<T> = std::result::Result<T, Error>;
```

- [ ] **Step 5: 创建 `src/main.rs`（占位实现，Task 10 替换）**

```rust
fn main() {
    eprintln!("ais: not implemented yet (foundations not landed)");
    std::process::exit(2);
}
```

- [ ] **Step 6: 替换 `README.md`**

```markdown
# ai-switch

Claude Code 配置切换工具：profile 单元 = 标准 Claude Code `settings.json` 文件，零 lock-in。

详见 [docs/superpowers/specs/2026-04-29-ai-switch-design.md](docs/superpowers/specs/2026-04-29-ai-switch-design.md)。
```

- [ ] **Step 7: 创建 `LICENSE`（MIT 单许可，与 Cargo.toml 的 `MIT OR Apache-2.0` 一致；Apache-2 文本可在 Plan C 补全或保持 MIT 单许可统一）**

```
MIT License

Copyright (c) 2026 yanghong

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

- [ ] **Step 8: 验证占位编译报错符合预期**

Run: `cargo check 2>&1 | head -10`
Expected: 报告 `unresolved module crate::catalog`（以及 claude/credentials/error/paths/profile/providers/settings）—— 后续 Task 会逐个补齐。

- [ ] **Step 9: Commit**

```bash
git add Cargo.toml .gitignore src/ README.md LICENSE
git commit -m "chore: greenfield restart, Cargo manifest + skeleton

Plan A Task 1. Single binary+lib crate, Rust 2024 edition. Deps
limited to Plan A scope; ratatui/crossterm/ureq deferred to Plan B."
```

---

### Task 2: `paths.rs` + 桩 `error.rs`

**Files:**
- Create: `src/paths.rs`
- Create: `src/error.rs` (stub; full enum 在 Task 3)

- [ ] **Step 1: 写 `src/error.rs` 桩**

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("could not locate user home directory")]
    HomeDirNotFound,
}
```

- [ ] **Step 2: 写 `src/paths.rs`（含单元测试）**

```rust
use std::path::PathBuf;

use crate::error::Error;

#[derive(Debug, Clone)]
pub struct Paths {
    pub root: PathBuf,
}

impl Paths {
    pub fn from_home() -> Result<Self, Error> {
        let dirs = directories::BaseDirs::new().ok_or(Error::HomeDirNotFound)?;
        Ok(Self::with_root(dirs.home_dir().join(".ai-switch")))
    }

    pub fn with_root(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn credentials(&self) -> PathBuf {
        self.root.join("credentials.toml")
    }

    pub fn providers(&self) -> PathBuf {
        self.root.join("providers.toml")
    }

    pub fn claude_dir(&self) -> PathBuf {
        self.root.join("claude")
    }

    pub fn claude_index(&self) -> PathBuf {
        self.claude_dir().join(".ais-index.toml")
    }

    pub fn settings_for(&self, profile_name: &str) -> PathBuf {
        self.claude_dir()
            .join(format!("settings_{profile_name}.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_resolve_relative_to_root() {
        let p = Paths::with_root(PathBuf::from("/tmp/test-ais"));
        assert_eq!(p.credentials(), PathBuf::from("/tmp/test-ais/credentials.toml"));
        assert_eq!(p.providers(), PathBuf::from("/tmp/test-ais/providers.toml"));
        assert_eq!(p.claude_dir(), PathBuf::from("/tmp/test-ais/claude"));
        assert_eq!(
            p.claude_index(),
            PathBuf::from("/tmp/test-ais/claude/.ais-index.toml")
        );
        assert_eq!(
            p.settings_for("work"),
            PathBuf::from("/tmp/test-ais/claude/settings_work.json")
        );
    }
}
```

- [ ] **Step 3: 跑测试**

Run: `cargo test --lib paths`
Expected: 1 test passes.

- [ ] **Step 4: Commit**

```bash
git add src/paths.rs src/error.rs
git commit -m "feat(paths): path constants + stub error type"
```

---

### Task 3: `error.rs` 完整 enum

**Files:**
- Modify: `src/error.rs`

- [ ] **Step 1: 重写 `src/error.rs` 为完整 variant**

```rust
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("could not locate user home directory")]
    HomeDirNotFound,

    #[error("profile not found: {name}")]
    ProfileNotFound { name: String },

    #[error("`claude` not found in PATH; install Claude Code first")]
    ClaudeNotInPath,

    #[error("could not parse `claude --version` output: {0}")]
    ClaudeVersionParse(String),

    #[error("could not parse settings.json at {path}: {source}")]
    SettingsParse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("credentials.toml at {path} is corrupted: {source}")]
    CredentialsCorrupted {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("providers.toml at {path} is corrupted: {source}")]
    ProvidersCorrupted {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error(".ais-index.toml at {path} is corrupted: {source}")]
    IndexCorrupted {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("key id `{id}` already exists for provider `{provider}`")]
    KeyIdConflict { provider: String, id: String },

    #[error("invalid profile name `{name}`: {reason}")]
    InvalidProfileName { name: String, reason: String },

    #[error("invalid key id `{id}`: {reason}")]
    InvalidKeyId { id: String, reason: String },

    #[error("provider `{id}` is not registered")]
    ProviderNotFound { id: String },

    #[error("provider `{id}` has no anthropic_base_url; cannot use as Claude profile")]
    ProviderMissingAnthropicUrl { id: String },

    #[error("key value too short to derive id automatically (need >=12 chars, got {len})")]
    KeyValueTooShortForAutoId { len: usize },

    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[cfg(unix)]
    #[error("file {path} permission too open (mode {mode:o}); should be 0600")]
    PermissionTooOpen { path: PathBuf, mode: u32 },
}
```

- [ ] **Step 2: 验证编译**

Run: `cargo check`
Expected: 还会因为 catalog/providers/etc 模块缺失而失败，但 paths.rs 与 error.rs 无错。可以用 `cargo check --lib 2>&1 | grep -E '(error|warning)' | head -20` 查具体哪些是 error.rs 引发的（应当为 0 条）。

- [ ] **Step 3: Commit**

```bash
git add src/error.rs
git commit -m "feat(error): complete Error enum per spec §11"
```

---

### Task 4: `catalog.rs` 内置 provider 表

**Files:**
- Create: `src/catalog.rs`

- [ ] **Step 1: 写 `src/catalog.rs`**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum AuthScheme {
    Bearer,
    XApiKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provider {
    pub id: String,
    pub display_name: String,
    pub anthropic_base_url: Option<String>,
    pub openai_base_url: Option<String>,
    pub auth: AuthScheme,
    pub models_endpoint_path: String,
}

/// Built-in provider catalog (compile-time constant via fn).
///
/// 实际 URL 与端点路径在 v0.1.0 发布前会再 sweep 一次官方文档校准；
/// 当前值用于跑通整套数据流。
pub fn builtins() -> Vec<Provider> {
    vec![
        Provider {
            id: "anthropic-official".into(),
            display_name: "Anthropic Official".into(),
            anthropic_base_url: Some("https://api.anthropic.com".into()),
            openai_base_url: None,
            auth: AuthScheme::XApiKey,
            models_endpoint_path: "/v1/models".into(),
        },
        Provider {
            id: "deepseek".into(),
            display_name: "DeepSeek".into(),
            anthropic_base_url: Some("https://api.deepseek.com/anthropic".into()),
            openai_base_url: Some("https://api.deepseek.com/v1".into()),
            auth: AuthScheme::Bearer,
            models_endpoint_path: "/v1/models".into(),
        },
        Provider {
            id: "openrouter".into(),
            display_name: "OpenRouter".into(),
            anthropic_base_url: Some("https://openrouter.ai/api/v1".into()),
            openai_base_url: Some("https://openrouter.ai/api/v1".into()),
            auth: AuthScheme::Bearer,
            models_endpoint_path: "/v1/models".into(),
        },
        Provider {
            id: "kimi".into(),
            display_name: "Kimi (Moonshot)".into(),
            anthropic_base_url: Some("https://api.moonshot.cn/anthropic".into()),
            openai_base_url: Some("https://api.moonshot.cn/v1".into()),
            auth: AuthScheme::Bearer,
            models_endpoint_path: "/v1/models".into(),
        },
        Provider {
            id: "glm".into(),
            display_name: "GLM (智谱)".into(),
            anthropic_base_url: Some("https://open.bigmodel.cn/api/anthropic".into()),
            openai_base_url: Some("https://open.bigmodel.cn/api/paas/v4".into()),
            auth: AuthScheme::Bearer,
            models_endpoint_path: "/v1/models".into(),
        },
    ]
}

pub fn find(id: &str) -> Option<Provider> {
    builtins().into_iter().find(|p| p.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn builtins_includes_anthropic_official() {
        let p = find("anthropic-official").unwrap();
        assert_eq!(p.auth, AuthScheme::XApiKey);
        assert_eq!(p.openai_base_url, None);
        assert!(p.anthropic_base_url.is_some());
    }

    #[test]
    fn builtins_deepseek_has_both_endpoints() {
        let p = find("deepseek").unwrap();
        assert_eq!(p.auth, AuthScheme::Bearer);
        assert!(p.anthropic_base_url.is_some());
        assert!(p.openai_base_url.is_some());
    }

    #[test]
    fn unknown_provider_returns_none() {
        assert!(find("nonexistent").is_none());
    }

    #[test]
    fn all_builtins_have_unique_ids() {
        let bs = builtins();
        let ids: BTreeSet<_> = bs.iter().map(|p| p.id.clone()).collect();
        assert_eq!(ids.len(), bs.len());
    }
}
```

- [ ] **Step 2: 跑测试**

Run: `cargo test --lib catalog`
Expected: 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/catalog.rs
git commit -m "feat(catalog): built-in provider table per spec §5.2"
```

---

### Task 5: `providers.rs` 用户 TOML 加载与覆盖

**Files:**
- Create: `src/providers.rs`

- [ ] **Step 1: 写 `src/providers.rs`**

```rust
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::catalog::{self, AuthScheme, Provider};
use crate::error::Error;

#[derive(Debug, Deserialize, Serialize)]
struct UserProvider {
    display_name: Option<String>,
    anthropic_base_url: Option<String>,
    openai_base_url: Option<String>,
    auth: AuthScheme,
    #[serde(default = "default_models_endpoint")]
    models_endpoint_path: String,
}

fn default_models_endpoint() -> String {
    "/v1/models".into()
}

fn parse_user_file(path: &Path) -> Result<BTreeMap<String, UserProvider>, Error> {
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let s = fs::read_to_string(path).map_err(|e| Error::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    toml::from_str(&s).map_err(|e| Error::ProvidersCorrupted {
        path: path.to_path_buf(),
        source: e,
    })
}

/// Load full provider table = built-ins overlaid by user-defined providers.
///
/// 同名 provider：用户定义覆盖内置（spec §5.3）。
pub fn load_all(user_providers_path: &Path) -> Result<Vec<Provider>, Error> {
    let user = parse_user_file(user_providers_path)?;

    let mut by_id: BTreeMap<String, Provider> = catalog::builtins()
        .into_iter()
        .map(|p| (p.id.clone(), p))
        .collect();

    for (id, up) in user {
        let provider = Provider {
            id: id.clone(),
            display_name: up.display_name.unwrap_or_else(|| id.clone()),
            anthropic_base_url: up.anthropic_base_url,
            openai_base_url: up.openai_base_url,
            auth: up.auth,
            models_endpoint_path: up.models_endpoint_path,
        };
        by_id.insert(id, provider);
    }

    Ok(by_id.into_values().collect())
}

/// Convenience: find a provider by id, considering user overrides.
pub fn find(user_providers_path: &Path, id: &str) -> Result<Option<Provider>, Error> {
    Ok(load_all(user_providers_path)?.into_iter().find(|p| p.id == id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_root() -> PathBuf {
        let id = format!(
            "ais-providers-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let p = std::env::temp_dir().join(id);
        fs::create_dir_all(&p).unwrap();
        p
    }

    struct TempRoot(PathBuf);
    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn no_user_file_returns_only_builtins() {
        let root = TempRoot(temp_root());
        let path = root.0.join("providers.toml");
        let all = load_all(&path).unwrap();
        assert_eq!(all.len(), catalog::builtins().len());
    }

    #[test]
    fn user_can_add_new_provider() {
        let root = TempRoot(temp_root());
        let path = root.0.join("providers.toml");
        fs::write(
            &path,
            r#"
[my-relay]
display_name = "My Private Relay"
anthropic_base_url = "https://my-relay.example.com/anthropic"
openai_base_url = "https://my-relay.example.com/v1"
auth = "Bearer"
"#,
        )
        .unwrap();
        let all = load_all(&path).unwrap();
        let mine = all.iter().find(|p| p.id == "my-relay").unwrap();
        assert_eq!(
            mine.anthropic_base_url.as_deref(),
            Some("https://my-relay.example.com/anthropic")
        );
        assert_eq!(mine.auth, AuthScheme::Bearer);
        assert_eq!(mine.models_endpoint_path, "/v1/models"); // default
    }

    #[test]
    fn user_override_wins_against_builtin() {
        let root = TempRoot(temp_root());
        let path = root.0.join("providers.toml");
        fs::write(
            &path,
            r#"
[deepseek]
anthropic_base_url = "https://my-deepseek-mirror.example.com/anthropic"
openai_base_url = "https://my-deepseek-mirror.example.com/v1"
auth = "Bearer"
"#,
        )
        .unwrap();
        let all = load_all(&path).unwrap();
        let ds = all.iter().find(|p| p.id == "deepseek").unwrap();
        assert_eq!(
            ds.anthropic_base_url.as_deref(),
            Some("https://my-deepseek-mirror.example.com/anthropic")
        );
    }

    #[test]
    fn corrupted_file_returns_error() {
        let root = TempRoot(temp_root());
        let path = root.0.join("providers.toml");
        fs::write(&path, "this is not toml === [[[").unwrap();
        let err = load_all(&path).unwrap_err();
        assert!(matches!(err, Error::ProvidersCorrupted { .. }));
    }
}
```

- [ ] **Step 2: 跑测试**

Run: `cargo test --lib providers`
Expected: 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/providers.rs
git commit -m "feat(providers): user TOML loader + builtin override per spec §5.3"
```

---

### Task 6: `credentials.rs` —— id 算法 + 文件 io

**Files:**
- Create: `src/credentials.rs`

模块最密；分两步落代码。

- [ ] **Step 1: 写 id 算法 + Store 类型 + 单元测试**

`src/credentials.rs`：

```rust
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Key {
    pub value: String,
    #[serde(default)]
    pub note: String,
}

/// On-disk shape: { provider_id: { key_id: Key } }
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Store {
    pub by_provider: BTreeMap<String, BTreeMap<String, Key>>,
}

/// Compute the default redacted id for a key value (spec §6.2 rule 1).
///
/// - len >= 12 → `<前4>...<后4>`
/// - len < 12  → `KeyValueTooShortForAutoId`（调用方应弹"请用户手动输入 id"）
pub fn auto_id(value: &str) -> Result<String, Error> {
    let len = value.chars().count();
    if len < 12 {
        return Err(Error::KeyValueTooShortForAutoId { len });
    }
    let head: String = value.chars().take(4).collect();
    let chars: Vec<char> = value.chars().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    Ok(format!("{head}...{tail}"))
}

/// Widen a redacted id by 1 char on each side (spec §6.2 rule 3).
///
/// Returns the wider candidate, or `None` if widening would overlap.
fn widen(value: &str, head_n: usize, tail_n: usize) -> Option<String> {
    let chars: Vec<char> = value.chars().collect();
    let new_head = head_n + 1;
    let new_tail = tail_n + 1;
    if new_head + new_tail >= chars.len() {
        return None;
    }
    let head: String = chars[..new_head].iter().collect();
    let tail: String = chars[chars.len() - new_tail..].iter().collect();
    Some(format!("{head}...{tail}"))
}

/// Generate a unique id for `value` against `existing` ids in the same provider.
/// Starts at 4+4, widens on collision until unique. Returns Err if value < 12 chars
/// (caller should ask user to type one) or exhausts widening room.
pub fn unique_id(value: &str, existing: &[String]) -> Result<String, Error> {
    let mut head_n = 4usize;
    let mut tail_n = 4usize;
    let mut candidate = auto_id(value)?;
    while existing.iter().any(|e| e == &candidate) {
        match widen(value, head_n, tail_n) {
            Some(c) => {
                candidate = c;
                head_n += 1;
                tail_n += 1;
            }
            None => {
                return Err(Error::KeyIdConflict {
                    provider: String::new(),
                    id: candidate,
                });
            }
        }
    }
    Ok(candidate)
}

/// Validate a user-supplied custom key id (spec §6.2 rule 4).
pub fn validate_id(id: &str) -> Result<(), Error> {
    if id.is_empty() {
        return Err(Error::InvalidKeyId {
            id: id.into(),
            reason: "must not be empty".into(),
        });
    }
    let ok = id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.');
    if !ok {
        return Err(Error::InvalidKeyId {
            id: id.into(),
            reason: "allowed chars: [a-zA-Z0-9._-]".into(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod algo_tests {
    use super::*;

    #[test]
    fn auto_id_for_normal_key() {
        assert_eq!(auto_id("sk-aaaaaaaaafswv").unwrap(), "sk-a...fswv");
    }

    #[test]
    fn auto_id_at_exact_12_boundary() {
        assert_eq!(auto_id("123456789012").unwrap(), "1234...9012");
    }

    #[test]
    fn auto_id_too_short_errors() {
        let err = auto_id("12345678901").unwrap_err();
        assert!(matches!(err, Error::KeyValueTooShortForAutoId { len: 11 }));
    }

    #[test]
    fn unique_id_no_collision() {
        let id = unique_id("sk-aaaaaaaaafswv", &[]).unwrap();
        assert_eq!(id, "sk-a...fswv");
    }

    #[test]
    fn unique_id_widens_on_collision() {
        let existing = vec!["sk-a...fswv".to_string()];
        // Pick a 15-char value that shares first 4 + last 4 with the existing id.
        let id = unique_id("sk-aaXaaaXfswv1", &existing).unwrap();
        // First widen attempt: 5+5 over the 15-char string.
        // chars = ['s','k','-','a','a','X','a','a','X','f','s','w','v','1', + one more if 15]
        // Verify the widened form differs from the colliding "sk-a...fswv".
        assert_ne!(id, "sk-a...fswv");
        assert!(id.contains("..."));
    }

    #[test]
    fn validate_id_accepts_legal() {
        assert!(validate_id("sk-a...fswv").is_ok());
        assert!(validate_id("personal").is_ok());
        assert!(validate_id("k1").is_ok());
        assert!(validate_id("a.b").is_ok());
    }

    #[test]
    fn validate_id_rejects_illegal() {
        assert!(validate_id("").is_err());
        assert!(validate_id("has space").is_err());
        assert!(validate_id("with/slash").is_err());
        assert!(validate_id("中文").is_err());
    }
}
```

- [ ] **Step 2: 跑算法测试**

Run: `cargo test --lib credentials::algo_tests`
Expected: 7 tests pass.

- [ ] **Step 3: 追加 io（load/save + 0600 权限）**

在 `src/credentials.rs` 末尾追加：

```rust
pub fn load(path: &Path) -> Result<Store, Error> {
    if !path.exists() {
        return Ok(Store::default());
    }
    let s = fs::read_to_string(path).map_err(|e| Error::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    toml::from_str(&s).map_err(|e| Error::CredentialsCorrupted {
        path: path.to_path_buf(),
        source: e,
    })
}

pub fn save(path: &Path, store: &Store) -> Result<(), Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| Error::Io {
            path: parent.to_path_buf(),
            source: e,
        })?;
    }
    let serialized = toml::to_string_pretty(store).map_err(|e| Error::Io {
        path: path.to_path_buf(),
        source: std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
    })?;
    fs::write(path, serialized).map_err(|e| Error::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    set_permissions_0600(path)?;
    Ok(())
}

#[cfg(unix)]
fn set_permissions_0600(path: &Path) -> Result<(), Error> {
    use std::os::unix::fs::PermissionsExt;
    let perm = fs::Permissions::from_mode(0o600);
    fs::set_permissions(path, perm).map_err(|e| Error::Io {
        path: path.to_path_buf(),
        source: e,
    })
}

#[cfg(not(unix))]
fn set_permissions_0600(_path: &Path) -> Result<(), Error> {
    Ok(())
}

#[cfg(test)]
mod io_tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_root() -> PathBuf {
        let id = format!(
            "ais-cred-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let p = std::env::temp_dir().join(id);
        fs::create_dir_all(&p).unwrap();
        p
    }

    struct TempRoot(PathBuf);
    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn missing_file_loads_empty_store() {
        let root = TempRoot(temp_root());
        let path = root.0.join("credentials.toml");
        let store = load(&path).unwrap();
        assert!(store.by_provider.is_empty());
    }

    #[test]
    fn round_trip_preserves_keys() {
        let root = TempRoot(temp_root());
        let path = root.0.join("credentials.toml");

        let mut store = Store::default();
        let mut deepseek = BTreeMap::new();
        deepseek.insert(
            "sk-a...fswv".into(),
            Key {
                value: "sk-aaaaaaaaafswv".into(),
                note: "personal".into(),
            },
        );
        deepseek.insert(
            "sk-b...gwzh".into(),
            Key {
                value: "sk-bbbbbbbbbgwzh".into(),
                note: "company".into(),
            },
        );
        store.by_provider.insert("deepseek".into(), deepseek);

        save(&path, &store).unwrap();
        let loaded = load(&path).unwrap();

        let ds = &loaded.by_provider["deepseek"];
        assert_eq!(ds["sk-a...fswv"].value, "sk-aaaaaaaaafswv");
        assert_eq!(ds["sk-a...fswv"].note, "personal");
        assert_eq!(ds["sk-b...gwzh"].value, "sk-bbbbbbbbbgwzh");
    }

    #[test]
    fn corrupted_file_returns_error() {
        let root = TempRoot(temp_root());
        let path = root.0.join("credentials.toml");
        fs::write(&path, "this = is = not = toml = ====").unwrap();
        let err = load(&path).unwrap_err();
        assert!(matches!(err, Error::CredentialsCorrupted { .. }));
    }

    #[cfg(unix)]
    #[test]
    fn save_sets_0600_on_unix() {
        use std::os::unix::fs::PermissionsExt;
        let root = TempRoot(temp_root());
        let path = root.0.join("credentials.toml");
        save(&path, &Store::default()).unwrap();
        let perm = fs::metadata(&path).unwrap().permissions();
        assert_eq!(perm.mode() & 0o777, 0o600);
    }
}
```

- [ ] **Step 4: 跑全部 credentials 测试**

Run: `cargo test --lib credentials`
Expected: 11 tests pass（7 algo + 4 io）。

- [ ] **Step 5: Commit**

```bash
git add src/credentials.rs
git commit -m "feat(credentials): key store, id algorithm, 0600 on unix"
```

---

### Task 7: `settings.rs` —— Claude Code settings.json 渲染

**Files:**
- Create: `src/settings.rs`

- [ ] **Step 1: 写 `src/settings.rs`**

```rust
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Error;

/// Minimal Claude Code settings.json shape we manage.
///
/// 我们仅主动写 `env` 块（spec §10 数据契约 1）；用户在文件里手加的其他
/// 字段（permissions / hooks / statusLine / ...）都收集进 `extras`，
/// 在 round-trip 时原样保留。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,

    #[serde(flatten)]
    pub extras: BTreeMap<String, serde_json::Value>,
}

impl Settings {
    pub fn render(anthropic_base_url: &str, api_key: &str, model: &str) -> Self {
        let mut env = BTreeMap::new();
        env.insert("ANTHROPIC_BASE_URL".into(), anthropic_base_url.into());
        env.insert("ANTHROPIC_API_KEY".into(), api_key.into());
        env.insert("ANTHROPIC_MODEL".into(), model.into());
        Self {
            env,
            extras: BTreeMap::new(),
        }
    }

    pub fn load(path: &Path) -> Result<Self, Error> {
        let s = fs::read_to_string(path).map_err(|e| Error::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        serde_json::from_str(&s).map_err(|e| Error::SettingsParse {
            path: path.to_path_buf(),
            source: e,
        })
    }

    pub fn save(&self, path: &Path) -> Result<(), Error> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| Error::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| Error::SettingsParse {
            path: path.to_path_buf(),
            source: e,
        })?;
        fs::write(path, json + "\n").map_err(|e| Error::Io {
            path: path.to_path_buf(),
            source: e,
        })
    }

    /// Replace only `env.ANTHROPIC_API_KEY`; everything else (其他 env vars / extras) untouched.
    pub fn replace_api_key(&mut self, new_key: &str) {
        self.env.insert("ANTHROPIC_API_KEY".into(), new_key.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_path(name: &str) -> PathBuf {
        let id = format!(
            "ais-settings-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            name
        );
        std::env::temp_dir().join(id)
    }

    #[test]
    fn render_emits_three_env_keys() {
        let s = Settings::render(
            "https://api.deepseek.com/anthropic",
            "sk-aaa",
            "deepseek-chat",
        );
        assert_eq!(s.env["ANTHROPIC_BASE_URL"], "https://api.deepseek.com/anthropic");
        assert_eq!(s.env["ANTHROPIC_API_KEY"], "sk-aaa");
        assert_eq!(s.env["ANTHROPIC_MODEL"], "deepseek-chat");
        assert!(s.extras.is_empty());
    }

    #[test]
    fn round_trip_preserves_extras() {
        let path = temp_path("extras");
        let mut s = Settings::render("u", "k", "m");
        s.extras.insert(
            "permissions".into(),
            serde_json::json!({ "allow": ["bash:*"] }),
        );
        s.save(&path).unwrap();

        let loaded = Settings::load(&path).unwrap();
        assert_eq!(loaded.env["ANTHROPIC_API_KEY"], "k");
        assert_eq!(
            loaded.extras["permissions"],
            serde_json::json!({ "allow": ["bash:*"] })
        );

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn replace_api_key_does_not_touch_other_env() {
        let mut s = Settings::render("u", "old-key", "m");
        s.env.insert("CUSTOM_VAR".into(), "should-stay".into());
        s.replace_api_key("new-key");
        assert_eq!(s.env["ANTHROPIC_API_KEY"], "new-key");
        assert_eq!(s.env["CUSTOM_VAR"], "should-stay");
        assert_eq!(s.env["ANTHROPIC_BASE_URL"], "u");
    }

    #[test]
    fn parse_corrupted_returns_error() {
        let path = temp_path("corrupt");
        fs::write(&path, "{{{ not json").unwrap();
        let err = Settings::load(&path).unwrap_err();
        assert!(matches!(err, Error::SettingsParse { .. }));
        let _ = fs::remove_file(&path);
    }
}
```

- [ ] **Step 2: 跑测试**

Run: `cargo test --lib settings`
Expected: 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/settings.rs
git commit -m "feat(settings): Claude Code settings.json render + extras round-trip"
```

---

### Task 8: `profile.rs` —— Index + 名字校验/建议 + create/delete/rotate

**Files:**
- Create: `src/profile.rs`

- [ ] **Step 1: 写 `src/profile.rs`**

```rust
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::paths::Paths;
use crate::settings::Settings;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexEntry {
    pub provider: String,
    pub key_id: String,
    pub model: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Index {
    pub entries: BTreeMap<String, IndexEntry>,
}

impl Index {
    pub fn load(path: &Path) -> Result<Self, Error> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let s = fs::read_to_string(path).map_err(|e| Error::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        toml::from_str(&s).map_err(|e| Error::IndexCorrupted {
            path: path.to_path_buf(),
            source: e,
        })
    }

    pub fn save(&self, path: &Path) -> Result<(), Error> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| Error::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }
        let s = toml::to_string_pretty(self).map_err(|e| Error::Io {
            path: path.to_path_buf(),
            source: std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
        })?;
        fs::write(path, s).map_err(|e| Error::Io {
            path: path.to_path_buf(),
            source: e,
        })
    }
}

/// Validate a profile name per spec §7 Step 4.
pub fn validate_name(name: &str) -> Result<(), Error> {
    if name.is_empty() || name.len() > 64 {
        return Err(Error::InvalidProfileName {
            name: name.into(),
            reason: format!("length must be 1-64, got {}", name.len()),
        });
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(Error::InvalidProfileName {
            name: name.into(),
            reason: "allowed chars: [a-zA-Z0-9_-]".into(),
        });
    }
    Ok(())
}

/// Default suggested profile name (spec §7 Step 4 rule 1).
/// Replaces any model chars not in [a-zA-Z0-9_-] with `-`.
pub fn suggested_name(provider_id: &str, model: &str) -> String {
    let model_clean: String = model
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    format!("{provider_id}_{model_clean}")
}

/// Fallback suggested name when rule-1 collides (spec §7 Step 4 rule 2).
pub fn suggested_name_with_key(provider_id: &str, model: &str, key_id: &str) -> String {
    let cleaned: String = key_id.replace("...", "_");
    format!("{}_{}", suggested_name(provider_id, model), cleaned)
}

pub struct CreateInput<'a> {
    pub name: &'a str,
    pub provider_id: &'a str,
    pub key_id: &'a str,
    pub model: &'a str,
    pub anthropic_base_url: &'a str,
    pub api_key_value: &'a str,
}

/// Create a new profile: write settings_<name>.json + add index entry.
/// Caller is responsible for ensuring (provider, key) already exist in credentials store.
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
    settings.save(&paths.settings_for(input.name))?;

    let mut index = Index::load(&paths.claude_index())?;
    index.entries.insert(
        input.name.into(),
        IndexEntry {
            provider: input.provider_id.into(),
            key_id: input.key_id.into(),
            model: input.model.into(),
            created_at: Utc::now(),
        },
    );
    index.save(&paths.claude_index())?;

    Ok(())
}

/// Delete a profile: remove settings_<name>.json + index entry.
/// Key in credentials.toml is untouched (跨 profile 共享).
pub fn delete(paths: &Paths, name: &str) -> Result<(), Error> {
    let settings_path = paths.settings_for(name);
    if settings_path.exists() {
        fs::remove_file(&settings_path).map_err(|e| Error::Io {
            path: settings_path.clone(),
            source: e,
        })?;
    }
    let mut index = Index::load(&paths.claude_index())?;
    index.entries.remove(name);
    index.save(&paths.claude_index())?;
    Ok(())
}

/// Re-render every settings.json that references (provider, key_id) with new key value.
/// Returns the list of profile names whose settings.json was rewritten (sorted).
pub fn rotate_key(
    paths: &Paths,
    provider_id: &str,
    key_id: &str,
    new_key_value: &str,
) -> Result<Vec<String>, Error> {
    let index = Index::load(&paths.claude_index())?;
    let mut affected = Vec::new();

    for (name, entry) in &index.entries {
        if entry.provider == provider_id && entry.key_id == key_id {
            let path = paths.settings_for(name);
            let mut s = Settings::load(&path)?;
            s.replace_api_key(new_key_value);
            s.save(&path)?;
            affected.push(name.clone());
        }
    }
    affected.sort();
    Ok(affected)
}

/// Rename key_id in index entries (used by Plan B's TUI when key value edit changes the redacted id).
pub fn rename_key_id_in_index(
    paths: &Paths,
    provider_id: &str,
    old_id: &str,
    new_id: &str,
) -> Result<(), Error> {
    let mut index = Index::load(&paths.claude_index())?;
    for entry in index.entries.values_mut() {
        if entry.provider == provider_id && entry.key_id == old_id {
            entry.key_id = new_id.into();
        }
    }
    index.save(&paths.claude_index())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_root() -> PathBuf {
        let id = format!(
            "ais-profile-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let p = std::env::temp_dir().join(id);
        fs::create_dir_all(&p).unwrap();
        p
    }

    struct TempRoot(PathBuf);
    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn validate_name_accepts_legal_chars() {
        assert!(validate_name("deepseek_fast").is_ok());
        assert!(validate_name("work-anthropic").is_ok());
        assert!(validate_name("a").is_ok());
        assert!(validate_name(&"x".repeat(64)).is_ok());
    }

    #[test]
    fn validate_name_rejects_illegal() {
        assert!(validate_name("").is_err());
        assert!(validate_name(&"x".repeat(65)).is_err());
        assert!(validate_name("has spaces").is_err());
        assert!(validate_name("with/slash").is_err());
        assert!(validate_name("dot.notation").is_err());
    }

    #[test]
    fn suggested_name_basic() {
        assert_eq!(
            suggested_name("deepseek", "deepseek-chat"),
            "deepseek_deepseek-chat"
        );
    }

    #[test]
    fn suggested_name_replaces_invalid_model_chars() {
        assert_eq!(
            suggested_name("openrouter", "anthropic/claude-3-opus:beta"),
            "openrouter_anthropic-claude-3-opus-beta"
        );
    }

    #[test]
    fn suggested_name_with_key_replaces_ellipsis() {
        assert_eq!(
            suggested_name_with_key("deepseek", "deepseek-chat", "sk-a...fswv"),
            "deepseek_deepseek-chat_sk-a_fswv"
        );
    }

    #[test]
    fn create_then_delete_round_trip() {
        let root = TempRoot(temp_root());
        let paths = Paths::with_root(root.0.join("ais"));

        create(
            &paths,
            CreateInput {
                name: "my-deepseek",
                provider_id: "deepseek",
                key_id: "sk-a...fswv",
                model: "deepseek-chat",
                anthropic_base_url: "https://api.deepseek.com/anthropic",
                api_key_value: "sk-aaaaaaaaafswv",
            },
        )
        .unwrap();

        let settings_path = paths.settings_for("my-deepseek");
        assert!(settings_path.exists());
        let s = Settings::load(&settings_path).unwrap();
        assert_eq!(s.env["ANTHROPIC_API_KEY"], "sk-aaaaaaaaafswv");

        let index = Index::load(&paths.claude_index()).unwrap();
        let entry = &index.entries["my-deepseek"];
        assert_eq!(entry.provider, "deepseek");
        assert_eq!(entry.key_id, "sk-a...fswv");
        assert_eq!(entry.model, "deepseek-chat");

        delete(&paths, "my-deepseek").unwrap();
        assert!(!settings_path.exists());
        let index = Index::load(&paths.claude_index()).unwrap();
        assert!(!index.entries.contains_key("my-deepseek"));
    }

    #[test]
    fn rotate_key_updates_only_matching_profiles() {
        let root = TempRoot(temp_root());
        let paths = Paths::with_root(root.0.join("ais"));

        create(&paths, CreateInput {
            name: "p1", provider_id: "deepseek", key_id: "sk-a...fswv",
            model: "deepseek-chat", anthropic_base_url: "u1", api_key_value: "old-key",
        }).unwrap();
        create(&paths, CreateInput {
            name: "p2", provider_id: "deepseek", key_id: "sk-a...fswv",
            model: "deepseek-coder", anthropic_base_url: "u1", api_key_value: "old-key",
        }).unwrap();
        create(&paths, CreateInput {
            name: "p3", provider_id: "deepseek", key_id: "sk-b...gwzh",
            model: "deepseek-chat", anthropic_base_url: "u1", api_key_value: "other-key",
        }).unwrap();

        let affected = rotate_key(&paths, "deepseek", "sk-a...fswv", "new-key").unwrap();
        assert_eq!(affected, vec!["p1".to_string(), "p2".to_string()]);

        assert_eq!(
            Settings::load(&paths.settings_for("p1")).unwrap().env["ANTHROPIC_API_KEY"],
            "new-key"
        );
        assert_eq!(
            Settings::load(&paths.settings_for("p2")).unwrap().env["ANTHROPIC_API_KEY"],
            "new-key"
        );
        assert_eq!(
            Settings::load(&paths.settings_for("p3")).unwrap().env["ANTHROPIC_API_KEY"],
            "other-key"
        );
    }

    #[test]
    fn rename_key_id_in_index_only_targets_provider() {
        let root = TempRoot(temp_root());
        let paths = Paths::with_root(root.0.join("ais"));

        create(&paths, CreateInput {
            name: "p1", provider_id: "deepseek", key_id: "sk-a...fswv",
            model: "deepseek-chat", anthropic_base_url: "u", api_key_value: "v",
        }).unwrap();
        create(&paths, CreateInput {
            name: "p2", provider_id: "openrouter", key_id: "sk-a...fswv",
            model: "any", anthropic_base_url: "u", api_key_value: "v",
        }).unwrap();

        rename_key_id_in_index(&paths, "deepseek", "sk-a...fswv", "sk-aa...fswvv").unwrap();
        let idx = Index::load(&paths.claude_index()).unwrap();
        assert_eq!(idx.entries["p1"].key_id, "sk-aa...fswvv");
        assert_eq!(idx.entries["p2"].key_id, "sk-a...fswv");
    }
}
```

- [ ] **Step 2: 跑测试**

Run: `cargo test --lib profile`
Expected: 8 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/profile.rs
git commit -m "feat(profile): index, name validation/suggestion, create/delete/rotate"
```

---

### Task 9: `claude.rs` —— 二进制探测 + launch

**Files:**
- Create: `src/claude.rs`

- [ ] **Step 1: 写 `src/claude.rs`**

```rust
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::Error;

/// Locate the `claude` binary on PATH.
pub fn probe_path() -> Result<PathBuf, Error> {
    which::which("claude").map_err(|_| Error::ClaudeNotInPath)
}

/// Run `claude --version` and parse the output (loose parse: any non-empty trimmed line).
pub fn probe_version(claude_path: &Path) -> Result<String, Error> {
    let output = Command::new(claude_path)
        .arg("--version")
        .output()
        .map_err(|e| Error::Io {
            path: claude_path.to_path_buf(),
            source: e,
        })?;
    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if s.is_empty() {
        return Err(Error::ClaudeVersionParse(
            "empty stdout from `claude --version`".into(),
        ));
    }
    Ok(s)
}

/// Build the argument vector that we will pass to `claude`.
/// Pure function — unit tested.
pub fn build_args(settings_path: &Path, passthrough: &[String]) -> Vec<String> {
    let mut args = vec![
        "--settings".to_string(),
        settings_path.to_string_lossy().into_owned(),
    ];
    args.extend_from_slice(passthrough);
    args
}

/// Launch `claude` with our settings. On Unix this `execvp`'s and never returns; on Windows
/// it spawns a child, waits, and returns the exit code (defaults to 1 if signaled).
pub fn launch(
    claude_path: &Path,
    settings_path: &Path,
    passthrough: &[String],
) -> Result<i32, Error> {
    let args = build_args(settings_path, passthrough);

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = Command::new(claude_path).args(&args).exec();
        // exec only returns on failure
        Err(Error::Io {
            path: claude_path.to_path_buf(),
            source: err,
        })
    }

    #[cfg(not(unix))]
    {
        let status = Command::new(claude_path)
            .args(&args)
            .status()
            .map_err(|e| Error::Io {
                path: claude_path.to_path_buf(),
                source: e,
            })?;
        Ok(status.code().unwrap_or(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn build_args_inserts_settings_first() {
        let args = build_args(
            &PathBuf::from("/x/settings.json"),
            &["--print".into(), "hello".into()],
        );
        assert_eq!(
            args,
            vec![
                "--settings".to_string(),
                "/x/settings.json".to_string(),
                "--print".to_string(),
                "hello".to_string(),
            ]
        );
    }

    #[test]
    fn build_args_with_no_passthrough() {
        let args = build_args(&PathBuf::from("/x/s.json"), &[]);
        assert_eq!(args, vec!["--settings".to_string(), "/x/s.json".to_string()]);
    }
}
```

- [ ] **Step 2: 跑单元测试**

Run: `cargo test --lib claude`
Expected: 2 tests pass.

(`probe_path` / `probe_version` / `launch` 不写单元测试——它们是对系统命令的薄包装；集成测试在 Task 11 用一次性场景覆盖 `probe_path` 的 not-found 分支。)

- [ ] **Step 3: 验证整个 lib 此时全绿**

Run: `cargo test --lib`
Expected: paths(1) + catalog(4) + providers(4) + credentials(11) + settings(4) + profile(8) + claude(2) = **34 tests pass**。

- [ ] **Step 4: Commit**

```bash
git add src/claude.rs
git commit -m "feat(claude): binary probe + launch (execvp on unix, spawn on windows)"
```

---

### Task 10: `main.rs` CLI dispatch

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: 重写 `src/main.rs`**

```rust
use std::process::ExitCode;

use ais::{claude, paths::Paths};
use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "ais",
    version,
    about = "Claude Code 配置切换工具",
    long_about = "ais — Claude Code 配置切换工具。\n\
                  裸跑 `ais` 进入 TUI（Plan B 实现）；\n\
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
        None => {
            // 裸跑 `ais`：Plan B 接管 TUI；Plan A 期间打印帮助并退出 0。
            eprintln!("ais: TUI not yet available (Plan A only). Use `ais claude <name>` to launch.");
            ExitCode::from(0)
        }
        Some(Cmd::Claude { name, passthrough }) => match run_claude(&name, &passthrough) {
            Ok(code) => ExitCode::from(code as u8),
            Err(e) => {
                eprintln!("ais: {e}");
                ExitCode::from(1)
            }
        },
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

- [ ] **Step 2: 验证编译 + `--help` / `--version`**

Run: `cargo build`
Expected: pass.

Run: `cargo run -q -- --help 2>&1 | head -20`
Expected: clap-generated help mentioning `ais claude <NAME>`.

Run: `cargo run -q -- --version`
Expected: `ai-switch 0.1.0`（来自 Cargo.toml）。

- [ ] **Step 3: 验证 profile 不存在时 stderr 报错 + exit 1**

Run: `cargo run -q -- claude nonexistent-profile-xxx 2>&1 | tail -3 ; echo "exit=$?"`
Expected: stderr 含 "profile not found: nonexistent-profile-xxx"，`exit=1`。

(注：因为 `~/.ai-switch/` 此时根本不存在，`Paths::from_home()` 仍能成功——它不创建目录，只算路径。`settings_for(name).exists()` 返回 false，触发 `ProfileNotFound`。)

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat(main): clap-based CLI dispatch with `ais claude <name>`"
```

---

### Task 11: 启动路径集成测试（错误分支）

**Files:**
- Create: `tests/launch_smoke.rs`

完整启动（真的把 claude 跑起来）依赖外部二进制；放到 V0.1 人工验收清单（Task 13）。本任务覆盖错误分支 + profile 创建后能正确读出 settings 路径。

- [ ] **Step 1: 创建 `tests/launch_smoke.rs`**

```rust
//! 端到端 smoke：覆盖 ais 二进制在不同输入下的可观察行为。
//!
//! 不依赖真实 `claude` 二进制存在；只覆盖：
//! - profile 不存在 → exit 1，stderr 含特定错误
//! - profile 存在但 PATH 上无 claude → exit 1，stderr 含 "claude not in PATH"

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use ais::paths::Paths;
use ais::profile::{self, CreateInput};

fn cargo_bin() -> PathBuf {
    // CARGO_BIN_EXE_<bin-name> is set by Cargo for integration tests of bin targets.
    // ref: https://doc.rust-lang.org/cargo/reference/environment-variables.html
    PathBuf::from(env!("CARGO_BIN_EXE_ais"))
}

fn temp_root(tag: &str) -> PathBuf {
    let id = format!(
        "ais-launch-smoke-{}-{}-{}",
        tag,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let p = std::env::temp_dir().join(id);
    fs::create_dir_all(&p).unwrap();
    p
}

struct TempRoot(PathBuf);
impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn nonexistent_profile_yields_exit_1_and_stderr_message() {
    let root = TempRoot(temp_root("missing"));
    let home = root.0.join("home");
    fs::create_dir_all(&home).unwrap();

    let output = Command::new(cargo_bin())
        .arg("claude")
        .arg("definitely-not-a-profile-xxxxxxx")
        .env("HOME", &home)
        .env("USERPROFILE", &home) // Windows
        .output()
        .expect("ais binary should be runnable");

    assert!(!output.status.success(), "expected non-zero exit");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("profile not found"),
        "stderr should mention profile-not-found; got: {stderr}"
    );
}

#[test]
fn existing_profile_but_no_claude_in_path_yields_exit_1() {
    let root = TempRoot(temp_root("noclaude"));
    let home = root.0.join("home");
    fs::create_dir_all(&home).unwrap();

    // Build a minimal profile under <home>/.ai-switch/claude/settings_p1.json
    let paths = Paths::with_root(home.join(".ai-switch"));
    profile::create(
        &paths,
        CreateInput {
            name: "p1",
            provider_id: "deepseek",
            key_id: "sk-a...fswv",
            model: "deepseek-chat",
            anthropic_base_url: "https://api.deepseek.com/anthropic",
            api_key_value: "sk-aaaaaaaaafswv",
        },
    )
    .unwrap();

    let output = Command::new(cargo_bin())
        .arg("claude")
        .arg("p1")
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .env("PATH", "") // 强制 which::which("claude") 失败
        .output()
        .expect("ais binary should be runnable");

    assert!(!output.status.success(), "expected non-zero exit");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found in PATH") || stderr.contains("claude"),
        "stderr should mention claude-not-found; got: {stderr}"
    );
}
```

- [ ] **Step 2: 跑集成测试**

Run: `cargo test --test launch_smoke`
Expected: 2 tests pass.

注意：在 macOS/Linux 上，即使 `PATH=""`，部分 shell 内置可能仍有效；但 `which::which` 严格按 PATH 搜索，应当返回 not-found。如果某平台失败，第二条测试可能需要换成在临时空目录里设 PATH。如果测试稳定通过则不调整。

- [ ] **Step 3: Commit**

```bash
git add tests/launch_smoke.rs
git commit -m "test(launch): smoke for missing-profile and missing-claude error paths"
```

---

### Task 12: GitHub Actions CI

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: 写 CI workflow**

```yaml
name: ci

on:
  push:
    branches: [main]
  pull_request:

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: "-D warnings"

jobs:
  test:
    name: test (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    steps:
      - uses: actions/checkout@v4
      - name: Install stable rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - name: Cache cargo
        uses: Swatinem/rust-cache@v2
      - name: cargo fmt --check
        run: cargo fmt --all -- --check
      - name: cargo clippy
        run: cargo clippy --workspace --all-targets -- -D warnings
      - name: cargo test (lib + integration)
        run: cargo test --workspace --all-targets
      - name: cargo build --release
        run: cargo build --release
```

- [ ] **Step 2: 本地预演 fmt + clippy + test 三件套**

Run: `cargo fmt --all -- --check`
Expected: pass。如果失败，跑 `cargo fmt --all` 再 commit。

Run: `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -20`
Expected: 0 warnings。如有，逐个修。

Run: `cargo test --workspace --all-targets`
Expected: 36 tests pass（lib 34 + integration 2）。

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: cross-platform fmt+clippy+test+release matrix"
```

---

### Task 13: V0.1.0-alpha 人工验收清单（无代码改动；交付前 checklist）

**Files:** 无修改。把以下清单跑一遍，结果写到 PR 描述/Release Notes。

- [ ] **Step 1: 三平台 release build**

```bash
cargo build --release
```

Expected: 在你工作的 OS 上产生 `target/release/ais`（或 `ais.exe`）。

- [ ] **Step 2: `--help` / `--version` 输出正常**

```bash
./target/release/ais --help
./target/release/ais --version
```

Expected: help 列出 `claude` 子命令；version 是 `ai-switch 0.1.0`。

- [ ] **Step 3: 手工创建一个 profile 文件，裸 `claude --settings` 跑通**

```bash
mkdir -p ~/.ai-switch/claude
cat > ~/.ai-switch/claude/settings_smoke.json <<'EOF'
{
  "env": {
    "ANTHROPIC_BASE_URL": "https://api.deepseek.com/anthropic",
    "ANTHROPIC_API_KEY": "sk-PUT-A-REAL-KEY-HERE",
    "ANTHROPIC_MODEL": "deepseek-chat"
  }
}
EOF

# 先脱离 ais 直接跑，验证 settings.json 自身有效
claude --settings ~/.ai-switch/claude/settings_smoke.json --version
```

Expected: `claude` 能识别 `--settings` flag 并打印版本（实际请求模型不在此步范围）。

- [ ] **Step 4: `ais claude smoke` 等价于裸跑**

```bash
./target/release/ais claude smoke --version
```

Expected: 与 Step 3 输出一致；exit code 一致。

- [ ] **Step 5: 透传参数生效**

```bash
./target/release/ais claude smoke --print "echo hello"
```

Expected: claude 收到 `--print "echo hello"` 与裸 `claude --settings <path> --print "echo hello"` 等价。

- [ ] **Step 6: profile 不存在时报错正确**

```bash
./target/release/ais claude this-profile-does-not-exist-xxxxxxx
echo "exit=$?"
```

Expected: stderr "ais: profile not found: ..."，`exit=1`。

- [ ] **Step 7: 清理临时 profile**

```bash
rm -f ~/.ai-switch/claude/settings_smoke.json
```

如果 `~/.ai-switch/claude/.ais-index.toml` 存在但已为空（空 profile 索引文件），可一并删除。本验收不要求自动清理。

- [ ] **Step 8: Tag + Release Notes（可选，由 Plan C 正式 release 时统一）**

V1 Plan A 完成可以打 `v0.1.0-alpha.1`：
```bash
git tag v0.1.0-alpha.1
git push origin v0.1.0-alpha.1
```

---

## Plan A 自检 checklist（实施前过一眼）

- ✅ 所有 spec §3-§6、§8、§10、§11 的数据契约都有 Task 实现：catalog (§5.2)、providers (§5.3)、credentials (§6)、settings (§10)、profile (§7 写盘部分)、launch (§8)、error (§11)
- ✅ Spec §7（创建向导 UI）→ 仅落数据层 API（`profile::create` / `validate_name` / `suggested_name*`），UI 推 Plan B
- ✅ Spec §9 (TUI 视图) → 全部推 Plan B
- ✅ Spec §13 依赖清单 = Plan A Cargo.toml 的子集（去掉 ratatui/crossterm/ureq）
- ✅ Spec §14.1-14.3 单元/集成/CI 测试范围全覆盖
- ✅ Spec §14.4 人工验收清单的 1、3、4、6 都在 Task 13；TUI 退出（2）、轮换 settings.json（5）、Doctor（6 第二部分）依赖 Plan B/C，本 plan 不交付
- ⚠️ 内置 provider 表的 base_url 在发布前需校准（spec §5.2 已明示），Task 4 注释里提醒
- ⚠️ Plan B 入口：`main.rs` 裸跑 `ais` 当前打印 "TUI not yet available"，Plan B 第一个 task 替换该分支为 ratatui 主循环
- ⚠️ Plan C 入口：`Cargo.toml` 的 `repository` 字段已写 `https://github.com/YoungHong1992/ai-switch`，发布时如有变更同步更新

---

## 执行选择

Plan A 完成后可单独发 `v0.1.0-alpha.1` 内测包；Plans B、C 在后续会话用同样流程拆出。
