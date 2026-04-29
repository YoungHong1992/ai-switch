# ai-switch V1 设计文档

> 状态：定稿（待 spec 终审 → 进入 writing-plans）
> 日期：2026-04-29
> 命令名：`ais`
> 仓库根目录：`D:\Tools\DevMux`（待重命名为 `ai-switch`）

---

## 1. 概述

ai-switch（命令名 `ais`）是一个 **Claude Code 配置切换工具**。其核心特征有两点：

- **profile 单元 = 标准 Claude Code `settings.json` 文件**：每个 profile 就是一份独立、自包含、可单独使用的 settings.json；ais 通过 `claude --settings <path>` 显式启动而**不**改写用户的全局 `~/.claude/settings.json`
- **零 lock-in**：用户随时可以拿任意生成的 `settings_xxx.json` 脱离 ais 直接 `claude --settings xxx.json` 跑

**V1 范围**：只服务 Claude Code CLI。**架构预留多工具扩展**——磁盘布局、provider/credentials 抽象都按"未来 codex/cursor/aider 一起管理"的方向设计，但 V1 不实现非 Claude 工具的实际接入。

**核心隐喻**：`ais claude <name>` ≡ `claude --settings <path-to-settings>.json [...透传 args]`。ais 只负责 (a) 管理这些 settings.json、(b) 替你按 (provider × key × model) 三元组生成它们、(c) 替你启动 Claude Code 时透传 `--settings`。仅此而已。

---

## 2. 目标用户与场景

**目标用户**：单人开发者，在多个 Anthropic 账号 / 中转服务 / 不同模型供应商（官方 / DeepSeek / OpenRouter / Kimi / GLM / 私有中转）之间切换 Claude Code 的工作模式。

**典型一天**：
- 早上接公司项目：`ais claude work` → 用公司 Anthropic 官方账号
- 下午做开源副业：`ais claude deepseek-chat` → 用 DeepSeek 走 Anthropic 兼容端点
- 晚上调试模型：`ais claude deepseek-coder-fast` → 同 DeepSeek 账号但换模型
- 偶尔切自架中转：`ais claude my-relay`

**非目标**：
- 团队共享 profile（同一份 profile 多人协作）
- 多设备自动同步（用户可手动用 git）
- 多 AI Coding Agent 工具（V1 只 Claude，V2 才考虑 codex/cursor/aider）

---

## 3. 概念模型

```
┌─ provider (deepseek / openrouter / anthropic-official / 用户自定义...) ──┐
│  写死: anthropic_base_url + openai_base_url + 鉴权方式                    │
│      │                                                                   │
│      └── keys[]   ais 在 credentials.toml 里持有 N 个 key                │
│            │      (id 为脱敏字符串：<前4>...<后4>; 必要时用户手输)        │
│            │                                                              │
│            └── profile = settings_<name>.json                            │
│                  内容: env { ANTHROPIC_BASE_URL, _API_KEY, _MODEL }       │
│                  生成自 (provider, key, model) 三元组                    │
│                  生成后即明文固化，可脱离 ais 直接 claude --settings 跑  │
└──────────────────────────────────────────────────────────────────────────┘
```

**核心契约**：profile 是结果（快照），不是动态来源。settings.json 一旦生成，其内容是当下三元组的快照——后续 (provider, key, model) 任一变更不会自动追上去，需要 ais 主动重渲染（key 轮换批量重写所有引用方）。

---

## 4. 磁盘布局

```
~/.ai-switch/
├── credentials.toml        # 跨工具共享的 key 库（0600 权限）
├── providers.toml          # 用户自定义 provider；内置 catalog 打在二进制里
└── claude/
    ├── settings_<name>.json
    ├── settings_<name>.json
    └── .ais-index.toml     # ais 私有索引（每个 settings 来自哪个 provider/key/model）
```

**位置**：`~/.ai-switch/` 由 `directories::BaseDirs::home_dir()` 推导，跨平台一致。

**权限**：
- Linux/macOS：`credentials.toml` 创建时设为 `0600`；其他文件不强制
- Windows：依赖默认 ACL，不做特殊封装

**不写到 settings.json 的 ais 私有信息**：
- profile 与 (provider, key, model) 的对应关系存在 `~/.ai-switch/claude/.ais-index.toml`
- settings.json 严格保持纯净 Claude Code 格式，不夹任何 ais-only 字段
- 索引丢失/损坏：profile 仍能正常启动（settings.json 自包含），但批量轮换 key 会失效，TUI doctor 面板提示用户重建索引

---

## 5. Provider Catalog

### 5.1 数据结构

```rust
enum AuthScheme {
    Bearer,   // Authorization: Bearer <key>
    XApiKey,  // x-api-key: <key>
}

struct Provider {
    id: String,                          // "deepseek"
    display_name: String,                // "DeepSeek"
    anthropic_base_url: Option<String>,  // → settings.json 的 ANTHROPIC_BASE_URL
    openai_base_url: Option<String>,     // → ais 拉 /v1/models 用
    auth: AuthScheme,
    models_endpoint_path: String,        // 默认 "/v1/models"
}
```

**两类端点的分工**：
- `anthropic_base_url`：写进 settings.json 的 `ANTHROPIC_BASE_URL`，让 Claude Code 跑得起来。`None` 表示该 provider 不支持 Claude profile（catalog 中不应出现）。
- `openai_base_url`：ais 内部调 `/v1/models` 拉模型列表用（很多 provider 的 `/v1/models` 在 OpenAI 端点上而不在 Anthropic 端点上）。`None` 时向导跳过动态拉取，强制走自由输入。

### 5.2 内置 Catalog（编译时常量）

| provider id | display | anthropic_base_url | openai_base_url | auth |
|---|---|---|---|---|
| `anthropic-official` | Anthropic Official | `https://api.anthropic.com` | `None` | XApiKey |
| `deepseek` | DeepSeek | `https://api.deepseek.com/anthropic` | `https://api.deepseek.com/v1` | Bearer |
| `openrouter` | OpenRouter | （Anthropic 兼容端点） | `https://openrouter.ai/api/v1` | Bearer |
| `kimi` | Kimi (Moonshot) | `https://api.moonshot.cn/anthropic` | `https://api.moonshot.cn/v1` | Bearer |
| `glm` | GLM (智谱) | `https://open.bigmodel.cn/api/anthropic` | `https://open.bigmodel.cn/api/paas/v4` | Bearer |

> 实际 URL 与端点路径**待发布前再 sweep 一遍各家官方文档校准**。这张表说明字段范围与组装方式。

### 5.3 用户自定义 Provider

`~/.ai-switch/providers.toml`：

```toml
[my-relay]
display_name = "My Private Relay"
anthropic_base_url = "https://my-relay.example.com/anthropic"
openai_base_url = "https://my-relay.example.com/v1"
auth = "Bearer"
models_endpoint_path = "/v1/models"
```

**合并规则**：
- 启动时 ais 先读内置 catalog，再读 `providers.toml`
- 同名 provider：用户定义**覆盖**内置（应对中转/私有部署需要重写 base_url 的场景）
- 用户 provider 必须至少声明 `anthropic_base_url`，否则不可用作 Claude profile（TUI 校验时拒绝）

---

## 6. Credentials 模型

### 6.1 文件格式

```toml
[deepseek."sk-a...fswv"]
value = "sk-aaaaaaaaafswv"
note  = "personal"

[deepseek."sk-b...gwzh"]
value = "sk-bbbbbbbbbgwzh"
note  = "company"

[openrouter."sk-o...zzzz"]
value = "sk-or-real-key-zzzz"
note  = ""
```

### 6.2 Key id 生成规则

1. **`len(key) ≥ 12`**：默认 id = `<前4字符>...<后4字符>`，例如 `sk-a...fswv`。用户可在 TUI 添加向导里手动覆写为自定义字符串。
2. **`len(key) < 12`**：不自动生成，**强制**用户手动输入 id。
3. **唯一性**：同 provider 下若 id 撞车（罕见），扩成 `<前5>...<后5>`，再撞继续扩直到唯一。
4. **id 字符集**：`[a-zA-Z0-9._-]` 与省略号字符 `.`（自动 id 自然合法；用户自定义 id 受此校验，TUI 拒绝非法字符）。

### 6.3 渲染

- TUI 与 toml section 名一致使用 `<前4>...<后4>` 形式（带引号合法）
- 索引文件 `.ais-index.toml` 引用 key 也用同一个 id
- 不提供 `xxx` 替代版（一致性优先）

### 6.4 Key 编辑/轮换 UX

TUI `k` 面板里"编辑某条 key 的 value"是单一操作。改了 value 之后 ais 自动：

1. 重新计算脱敏 id（可能变也可能不变）
2. 重命名 `credentials.toml` 的 section（旧 id → 新 id）
3. 扫 `.ais-index.toml`，把所有引用旧 id 的项改成新 id
4. 重新渲染所有依赖该 key 的 settings.json（仅替换 `env.ANTHROPIC_API_KEY`，其他字段不动）
5. 弹回执：`已影响 N 个 profile，已同步更新`

---

## 7. Profile 创建向导（TUI 内）

主视图按 `n` 进入向导，5 步：

### Step 1 — 选 provider

```
┌─ 选 provider ───────────────────┐
│ ▶ deepseek                      │
│   anthropic-official            │
│   openrouter                    │
│   kimi                          │
│   glm                           │
│   my-relay (用户自定义)         │
│ ─────────────────────────────── │
│   + 添加新 provider…            │
└─────────────────────────────────┘
```

"添加新 provider" → 进入 provider 创建子流程（写 `providers.toml`），完成后回到 Step 1。

### Step 2 — 选/添加 key

```
┌─ 选 key（deepseek 下面）────────┐
│ ▶ sk-a...fswv  (personal)       │
│   sk-b...gwzh  (company)        │
│ ─────────────────────────────── │
│   + 添加新 key…                 │
└─────────────────────────────────┘
```

"添加新 key" → 弹输入框：

```
┌─ 添加新 key ────────────────────┐
│ Value: ************             │  输入屏蔽
│ ID:    sk-c...xyz9 (自动)       │  ≥12 时自动回填，可覆写
│                                 │  <12 时此格强制用户输入
│ Note:  ___________              │  可选
└─────────────────────────────────┘
```

写入 `credentials.toml`，回到 Step 2 选中新 key。

### Step 3 — 选 model

若 `provider.openai_base_url.is_some()`：

```
┌─ 正在拉取 https://api.deepseek.com/v1/models … ┐
└────────────────────────────────────────────────┘
```

成功 → 菜单展示模型列表 + 末位"自定义…"
失败/超时（默认 5 秒） → fallback 自由输入

若 `openai_base_url.is_none()`：直接走自由输入。

### Step 4 — 起名

```
┌─ Profile 名 ─────────────────────────────────┐
│ 建议: deepseek_deepseek-chat                 │  默认 <provider>_<model>
│ Name: ____________________                   │  回车接受建议；可改
│                                              │
│ 重名时 ais 主动追加 _<key-id-cleaned> 后缀  │
└──────────────────────────────────────────────┘
```

**校验**：`[a-zA-Z0-9_-]`，长度 1-64。

**默认建议名生成顺序**：
1. 首选 `<provider>_<model>`（例如 `deepseek_deepseek-chat`）
2. 若已存在 → ais 自动改建议为 `<provider>_<model>_<key-id-cleaned>`，其中 `<key-id-cleaned>` 是 key id 把 `...` 替换为 `_`（例如 `deepseek_deepseek-chat_sk-a___fswv`）
3. 若仍重名 → 输入框留空，用户必须自起

输入框始终显示当前建议值；用户可回车直接接受，也可改写后回车。

### Step 5 — 预览 & 确认

```
┌─ 即将生成 ──────────────────────────────────────────────┐
│ ~/.ai-switch/claude/settings_deepseek_deepseek-chat.json │
│                                                          │
│ {                                                        │
│   "env": {                                               │
│     "ANTHROPIC_BASE_URL": "https://api.deepseek.com/anthropic", │
│     "ANTHROPIC_API_KEY":  "sk-aaaaaaaaafswv",            │
│     "ANTHROPIC_MODEL":    "deepseek-chat"                │
│   }                                                      │
│ }                                                        │
│                                                          │
│ [Enter] 写盘   [Esc] 返回上一步                          │
└──────────────────────────────────────────────────────────┘
```

**写盘的副作用**（事务）：
1. 创建/覆盖 `~/.ai-switch/claude/settings_<name>.json`
2. 在 `.ais-index.toml` 追加：
   ```toml
   [<name>]
   provider = "deepseek"
   key_id = "sk-a...fswv"
   model = "deepseek-chat"
   created_at = "2026-04-29T14:32:00Z"
   ```
3. 若新 key 是首次添加，已在 Step 2 写入 `credentials.toml`

事务失败：回滚已创建的文件，TUI 弹错误条。

---

## 8. 启动流程（`ais claude <name>`）

```
ais claude deepseek_deepseek-chat [...透传 args]
       │
       ├─ 解析 <name>，定位 ~/.ai-switch/claude/settings_<name>.json
       ├─ 文件不存在 → stderr 输出错误 + exit 1
       ├─ which("claude")
       ├─ claude 不在 PATH → stderr 提示"未找到 claude，请安装" + exit 1
       │
       ├─ Unix:    execvp("claude", ["claude", "--settings", <abs-path>, ...args])
       │           （ais 进程被替换，零额外 PID；终端控制权完全移交）
       │
       └─ Windows: spawn + wait + 透传 exit code
                   （Windows 没有 execvp 等价；需要保留父进程）
```

**契约**：
- ais 不在 stdout/stderr 写任何东西（避免污染 claude 输出 / 给 IDE 集成带来惊喜）
- 错误（profile 不存在 / claude 不在 PATH）走 stderr + 非 0 退出码
- 退出码完全透传 claude 的

**逃生口（设计核心）**：用户随时可以
```bash
$ claude --settings ~/.ai-switch/claude/settings_<name>.json [...args]
```
完全脱离 ais 直接跑。这是产品的根本契约。

---

## 9. TUI 主视图与交互

### 9.1 主页（Profiles 视图）

```
┌─ ai-switch v0.1.0 ── claude 2.1.116 ── ~/.ai-switch/claude/ ─────────────┐
│                                                                          │
│ Profiles                          │ Details: deepseek_deepseek-chat      │
│                                   │                                      │
│ ▶ deepseek_deepseek-chat          │ Path:  ~/.ai-switch/claude/          │
│   provider=deepseek               │        settings_deepseek_deepseek    │
│   key=sk-a...fswv (personal)      │        -chat.json                    │
│   model=deepseek-chat             │                                      │
│                                   │ env:                                 │
│   work-anthropic                  │   ANTHROPIC_BASE_URL                 │
│   provider=anthropic-official     │     = https://api.deepseek.com/      │
│   key=sk-w...xxxx (work)          │       anthropic                      │
│   model=claude-sonnet-4-6         │   ANTHROPIC_API_KEY                  │
│                                   │     = sk-aaaa...fswv  (脱敏显示)     │
│                                   │   ANTHROPIC_MODEL                    │
│                                   │     = deepseek-chat                  │
│                                   │                                      │
│                                   │ Created: 2026-04-29 14:32            │
├──────────────────────────────────────────────────────────────────────────┤
│ [↑↓] move  [Enter] launch  [n] new  [e] edit  [r] rename  [x] delete    │
│ [p] providers  [k] keys  [d] doctor  [?] help  [q] quit                 │
└──────────────────────────────────────────────────────────────────────────┘
```

**空状态**（首跑无 profile）：左面板留白，中央提示"还没任何 profile。按 [n] 创建第一个、[p] 管理 provider、[?] 查看帮助"。底部状态栏照常。

### 9.2 二级面板

| 键 | 面板 | 功能 |
|---|---|---|
| `p` | Providers | 内置 provider 只读展示；用户自定义可增/删/改 base_url、auth、models endpoint |
| `k` | Keys (Credentials) | 跨 provider 管理所有 key：增加、改 value（触发批量重渲染）、改 note、删除 |
| `d` | Doctor | claude 是否在 PATH、版本、配置目录权限、credentials.toml 是否可读、孤儿 settings.json 检测 |
| `?` | Help | 全部快捷键速查 |

### 9.3 关键操作语义

- `Enter` 启动 = TUI 整洁退出 → exec claude（终端控制权完全交给 claude，ais 进程不再阻塞）
- `n` 新建 → §7 向导
- `e` 编辑 → 重走 §7 向导但锁定 profile 名（仅修改 (provider, key, model)；改后旧 settings.json 重写）
- `r` 重命名 → 弹输入框；改文件名 + 改索引项 key（不影响 settings.json 内容）
- `x` 删除 → 弹确认；**只删 settings.json + 索引项**，不删该 key（key 跨 profile 共享）
- 改 `k` 面板里某 key 的 value → 弹"将影响 N 个 profile"确认 → 全部重渲染
- `q` / `Esc` 主页 → 退出 TUI

### 9.4 Doctor 面板内容

| 检查项 | 通过条件 |
|---|---|
| `claude` 在 PATH | `which claude` 返回路径 |
| Claude Code 版本 | `claude --version` 解析成功 |
| `~/.ai-switch/` 可读写 | 创建临时文件 + 删除成功 |
| `credentials.toml` 权限 | Linux/macOS 上 `mode & 0o077 == 0`；Windows 跳过 |
| `.ais-index.toml` 一致性 | 索引项数 == 实际 settings 文件数；列出孤儿（有 json 无索引）/ 死链（有索引无 json） |
| Provider 表加载 | `providers.toml` 解析无错；同名覆盖关系列出 |

V1 Doctor 仅检测+报告，不自动修复。

---

## 10. 数据契约（关键不变量）

1. **settings.json 严格保持 Claude Code 标准格式**——不夹任何 ais 私有字段。任何 ais metadata 写到 `.ais-index.toml`。
2. **settings.json 里 API key 始终明文**——保"escape hatch"承诺；`credentials.toml` 也是明文（但 0600）。两处明文是已知权衡。
3. **profile 是快照**——生成后不动态跟随上游 (provider, key) 变化，除非显式触发重渲染。
4. **key 跨 profile 共享**——同一个 deepseek key 可被多个 profile 引用；删 profile 不删 key；删 key 必须先确认所有引用方失效。
5. **provider 同名覆盖**——用户定义 provider > 内置 provider。

---

## 11. 错误处理

`thiserror` 定义的核心错误 variant：

```rust
enum Error {
    ProfileNotFound { name: String },
    ClaudeNotInPath,
    ClaudeVersionParse(String),
    SettingsParse { path: PathBuf, source: serde_json::Error },
    CredentialsCorrupted { path: PathBuf, source: toml::de::Error },
    ProvidersCorrupted { path: PathBuf, source: toml::de::Error },
    IndexCorrupted { path: PathBuf, source: toml::de::Error },
    KeyIdConflict { provider: String, id: String },
    InvalidProfileName { name: String, reason: String },
    InvalidKeyId { id: String, reason: String },
    HttpFetch { url: String, source: ureq::Error },
    Io { path: PathBuf, source: std::io::Error },
    PermissionTooOpen { path: PathBuf, mode: u32 },  // 仅 Unix
}
```

**展示原则**：
- CLI 路径（`ais claude <name>` / `ais --version` / `-h`）：错误打 stderr + 非 0 exit
- TUI 路径：错误弹底部红色 toast，3 秒自动消失，操作维持状态
- 损坏文件（toml 解析失败）：TUI 拒绝启动，引导用户去 doctor 面板看明细 + 提供"备份原文件 + 创建空白文件"操作

---

## 12. 模块/代码结构

**Greenfield 重启**：旧 M1 代码（`crates/ais-core` / `ais-cli` / `ais-tui` 下的源码与测试、Cargo.toml、`.cargo/config.toml`、CI workflow）全部清掉，从头开始。

**单 binary crate**（V1 不做 lib/bin 拆分；V2 多工具时再 promote 出 `ais-core`）：

```
ai-switch/
├── Cargo.toml
├── README.md
├── LICENSE                # MIT OR Apache-2.0
├── .github/workflows/
│   └── ci.yml             # ubuntu/macos/windows 三平台 cargo test
└── src/
    ├── main.rs            # CLI dispatch: `ais [...]` / `ais claude <name>`
    ├── tui/
    │   ├── mod.rs         # 入口、事件循环
    │   ├── app.rs         # 应用状态机
    │   ├── views/
    │   │   ├── profiles.rs
    │   │   ├── providers.rs
    │   │   ├── keys.rs
    │   │   ├── doctor.rs
    │   │   └── wizard.rs  # §7 5 步向导
    │   └── widgets.rs
    ├── catalog.rs         # 内置 provider 表（编译时常量）
    ├── providers.rs       # ~/.ai-switch/providers.toml 加载与合并
    ├── credentials.rs     # ~/.ai-switch/credentials.toml 读写、id 算法、key 轮换
    ├── profile.rs         # settings_*.json + .ais-index.toml 读写、扫描、重渲染
    ├── settings.rs        # Claude Code settings.json 渲染（仅 env 块）
    ├── claude.rs          # detect、launch（Unix execvp / Windows spawn）
    ├── http.rs            # /v1/models 拉取（ureq 同步）
    ├── paths.rs           # ~/.ai-switch/ 路径常量
    └── error.rs
```

---

## 13. 依赖清单

### 运行时

```toml
[dependencies]
serde       = { version = "1", features = ["derive"] }
serde_json  = "1"
toml        = "0.8"
thiserror   = "1"
anyhow      = "1"
ratatui     = "0.29"
crossterm   = "0.28"
clap        = { version = "4", features = ["derive"] }
which       = "7"
ureq        = { version = "2", features = ["json"] }
directories = "5"
chrono      = { version = "0.4", features = ["serde"] }   # created_at
```

### 测试

仅 std + 项目内手写 `TestDir` 辅助（不引入 `tempfile`，避开旧 M1 那个 Win32 import lib 链接问题）。

### 不引入

- `keyring`（OS 安全存储）→ V2 再评估
- `tempfile` → 业务代码不需要，测试用手写 helper
- `reqwest` / `tokio` → 单线程 + 同步 HTTP 即可，`ureq` 足够

---

## 14. 验收门槛与测试边界

### 14.1 单元测试

- key id 脱敏算法（≥12 默认、<12 强制手输、扩展规则）
- credentials.toml round-trip（写后读 == 写入对象）
- 内置 provider catalog 加载 + 用户自定义合并优先级
- settings.json 渲染（输入 (provider, key, model) → 输出标准 Claude Code 格式）
- profile 名校验（合法字符、长度边界）

### 14.2 集成测试

- 创建 profile 全流程（mock 用户输入 → 生成 settings.json + 索引 + credentials）
- 改 key value 触发批量重渲染（验证 N 个 settings.json 全部更新且其他字段不动）
- 启动流程（用 `--print-settings-and-exit` 之类的 mock claude 二进制，验证 args 透传与 exit code 透传）
- 索引损坏后的降级行为（profile 仍能启动，doctor 面板报告）

### 14.3 跨平台 CI

GitHub Actions 矩阵 = `ubuntu-latest` / `macos-latest` / `windows-latest`，每平台跑 `cargo test --release`。

### 14.4 人工验收门槛（V1 上线必过）

1. 三平台 `cargo build --release` 通过
2. `ais` 进 TUI、`q` 退出，无残留 termios 状态
3. 走 §7 向导创建一个 deepseek profile，生成的 `settings_xxx.json` 用裸 `claude --settings <path>` 能跑通
4. `ais claude <name>` 启动效果与裸跑等价（同一会话、相同 env、相同模型）
5. 改 key value 后，老 settings.json 里的 ANTHROPIC_API_KEY 字段同步更新；其他字段（如未来用户手改的 permissions）保持不动
6. Doctor 面板 6 项检查全部通过

---

## 15. V1 非范围

| 项 | 推迟原因 |
|---|---|
| 多工具实际接入（`ais codex` / `ais cursor` / `ais aider`） | 磁盘布局已留位，V1 不实现 |
| shell-init / 全局 alias / wrapper | 用户已明确不要 CLI，不需要也不该提供 |
| OS keyring 集成 | credentials.toml 明文 + 0600 权限够用；keyring 引入跨平台链接复杂度 |
| 多设备同步 | 用户可手动把 `~/.ai-switch/` 加到 git/dropbox |
| 从 `~/.claude/settings.json` 导入 | 用户首次手起一个 profile 即可 |
| Doctor 自动修复 | V1 仅检测 + 报告 |
| `brew` / `scoop` / `winget` 分发 | V1 仅 GitHub Release 静态二进制 + `cargo install` |
| 自动更新（`ais self-update`） | 不做 |
| 遥测 | 不做 |
| settings.json 高级字段（permissions / hooks / statusLine 等） | 用户可手改文件；ais 不主动管理 |
| 多模型字段（ANTHROPIC_SMALL_FAST_MODEL 等） | 用户可手改 settings.json |

---

## 16. 风险与开放问题

| 风险 | 缓解 |
|---|---|
| Claude Code 未来改 `--settings` 行为或废弃该 flag | doctor 面板内化 Claude Code 版本检测；订阅其 changelog |
| 内置 provider 的 base_url / models endpoint 上游变更 | 内置表跟版本发布；用户自定义 provider 永远可绕开 |
| `/v1/models` 拉取慢/超时影响向导体验 | 默认 5 秒超时 + fallback 自由输入；网络异常下用户体验不卡 |
| credentials.toml 明文 = 安全负担 | 文档明示；0600 权限；V2 评估 keyring |
| 用户手改 settings.json 后再 `ais e` 编辑 → ais 重渲染会覆盖手改字段 | 当前规则：`e` 编辑只覆盖 env 块，其他字段保留；但若用户改了 env 块本身则会丢失。文档明示 |
| Windows 上 `execvp` 不可用，spawn + wait 路径与 Unix 不一致 | 启动语义文档明示；CI 三平台覆盖 |
| 索引（.ais-index.toml）与 settings 实际文件漂移 | doctor 面板检测；提供"重建索引"操作（按 base_url 反向匹配 provider） |
| 用户在 TUI 之外手动加/删 settings_xxx.json 文件 | 启动时扫描目录，与索引对账，孤儿文件按"未知来源 profile"展示，仍可启动但不能轮换 key |

### 待确认（不阻塞 spec 终审，留给 plan 阶段或实现期解决）

- 内置 provider 表的实际 URL 与端点路径（发布前 sweep 校准）
- `models_endpoint_path` 的 fallback 默认值是否够用（OpenRouter 等是否有特殊路径）
- TUI 在窄终端（< 80 列）下的降级布局
- profile 列表很多时的滚动 / 搜索（V1 简单线性即可，>30 个再优化）

---

## 17. 时间线（参考）

| 里程碑 | 范围 | 估算 |
|---|---|---|
| **M0** Greenfield 清场 | 删旧代码、新 Cargo.toml、CI 模板、`paths.rs` + `error.rs` 骨架 | 0.5 天 |
| **M1** 数据层 | catalog/providers/credentials/profile/settings 五个模块 + 单元测试 | 2 天 |
| **M2** 启动 CLI | `main.rs` dispatch + `claude.rs` + 集成测试 | 1 天 |
| **M3** TUI 骨架 | 主视图 + 5 个二级面板的导航与只读展示 | 2 天 |
| **M4** TUI 写操作 | 创建向导、key 轮换、provider 增删、删除/重命名 | 3 天 |
| **M5** Doctor + 打磨 | doctor 面板、错误体验、空状态、人工验收清单 | 1 天 |
| **M6** 发布 | README、安装脚本、GitHub Release、cargo install 验证 | 1 天 |

合计约 10.5 天，按需推进。

---
