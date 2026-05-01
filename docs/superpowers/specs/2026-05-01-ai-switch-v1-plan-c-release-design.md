# ai-switch V1 Plan C — Release (M6) 设计文档

> 状态：定稿（待 spec 终审 → 进入 writing-plans）
> 日期：2026-05-01
> 命令名：`ais`
> 上游 spec：[2026-04-29-ai-switch-design.md](2026-04-29-ai-switch-design.md)
> 关联 plan：[Plan A](../plans/2026-04-29-ai-switch-v1-plan-a-foundations.md)、[Plan B](../plans/2026-04-30-ai-switch-v1-plan-b-tui.md)

---

## 1. 概述

Plan C 是 ai-switch V1 的发布里程碑（设计 §17 的 M6）。Plan A（M0-M2）已完成 foundations + launch CLI，Plan B（M3-M5）已完成 TUI 全部视图与 Doctor。Plan C 把仓库从"代码功能完整"推到"对外可发布"。

**目标**：让 ai-switch v0.1.0 以 **Public Preview** 形态同时在 **GitHub Release**（4 个预编译二进制）与 **crates.io**（`cargo install ai-switch`）上线，且首次用户的开箱体验不被未校准的 provider catalog 折损。

**性质定位**：v0.1.0 = Public Preview。0.1.x 内允许 CLI / 磁盘配置 / provider catalog / Rust library API 的破坏性调整；不强承诺向后兼容。

**Plan C 触及代码的最小面**：原则上 `src/catalog.rs` 是唯一可能改动的业务代码文件；其余产物均为文档、CI、元数据。

---

## 2. 范围

### 2.1 在范围内（P0）

| 类别 | 交付物 | 影响文件 |
|---|---|---|
| 元数据 | Cargo.toml 补齐 crates.io 必备字段（不含 `rust-version`，理由见 §5）；版本号停在 `0.1.0`；双 LICENSE 落盘 | `Cargo.toml`、`LICENSE-MIT`（新建）、`LICENSE-APACHE`（新建）、删除旧 `LICENSE` |
| Catalog 校准 | 5 个内置 provider 的 4 字段按官方文档逐个 sweep；deepseek + anthropic-official 真账号过 wizard 亲测 | `src/catalog.rs`（按需调字面量；OpenRouter 视 sweep 结果决定是否移除） |
| 文档 | README 中等深度；CHANGELOG.md 起步（Keep a Changelog 1.1） | `README.md`、`CHANGELOG.md`（新建） |
| 发布管线 | `release.yml`：tag 触发 + preflight gates + 三段式 publish + 全平台 artifact smoke | `.github/workflows/release.yml`（新建） |
| 文案核对 | `--help` / `--version` 输出与设计对齐；Doctor 6 项文案与设计 §9.4 对照；快捷键文案与代码对齐（特别是 Keys 面板入口为大写 `K`） | 仅微调（如有出入） |
| 验收回执 | Linux 上 §14.4 1-6 全过；walkthrough 回执文件归档 + 写入 PR 描述 | `docs/superpowers/release-receipts/v0.1.0.md`（新建） |

### 2.2 不在范围（沿设计 §15 推迟）

- brew / scoop / winget 分发
- `curl | sh` 安装脚本（**与上游 §17 M6 偏离的有意收缩**：V1 仅 cargo install + GitHub Release 二进制；安装脚本评估推到 V0.2）
- 自动更新（`ais self-update`）
- 遥测
- 二进制签名（codesign / Authenticode）；macOS Gatekeeper 提示在 README FAQ 处理
- 多设备同步、shell-init / global alias
- README 截图、asciicast（留 V0.2）
- macOS / Windows 的人工 walkthrough（仅 Linux 亲测，但 release.yml 在每个平台对 archive 做 smoke——见 §4.4）
- Rust library API 的公开承诺面（`pub` 模块属内部接口，docs.rs 暴露但不受 SemVer 约束；README/CHANGELOG 明示）
- 其它未在 §2.1 列出的代码改动

---

## 3. 发布姿态决议

**版本号**：`0.1.0`

**渠道**：
- GitHub Release：tag `v0.1.0` 触发，发预编译二进制 + checksums；用 `--prerelease` 标记 Public Preview
- crates.io：同步首发 `ai-switch` 0.1.0；`cargo install ai-switch` 后命令名为 `ais`

**Release artifacts（4 targets）**：

| OS runner | target triple | artifact 名（从 tag 派生） | 备注 |
|---|---|---|---|
| `ubuntu-latest` | `x86_64-unknown-linux-musl` | `ais-${VER}-x86_64-unknown-linux-musl.tar.gz` | 真静态；`apt install musl-tools` |
| `macos-13` | `x86_64-apple-darwin` | `ais-${VER}-x86_64-apple-darwin.tar.gz` | macos-13 = Intel runner |
| `macos-14` | `aarch64-apple-darwin` | `ais-${VER}-aarch64-apple-darwin.tar.gz` | macos-14 = Apple Silicon |
| `windows-latest` | `x86_64-pc-windows-msvc` | `ais-${VER}-x86_64-pc-windows-msvc.zip` | msvc 默认链接 |

`${VER}` 从 `${GITHUB_REF_NAME#v}` 在 release.yml 内动态派生，**不在 workflow 里硬编码 `0.1.0`**——保未来 `0.1.1`、`0.2.0` 等 tag 不踩雷。

**Archive 内部结构**（防多包同时解压互相覆盖）：
```
ais-${VER}-<target>/
├── ais  (或 ais.exe)
├── LICENSE-MIT
├── LICENSE-APACHE
└── README.md
```

每 archive 同目录附 `<archive>.sha256`；publish job 汇总 `SHA256SUMS`。

**自动化语义**：tag 推送即触发全自动发布（含 `cargo publish`）；preflight 阶段保留 ~2 分钟人工 Cancel 窗口；进入 publish job 后 `cargo publish` 是不可逆点。

---

## 4. release.yml 工作流

### 4.1 触发与并发

```yaml
on:
  push:
    tags: ['v*.*.*']

concurrency:
  group: release-${{ github.ref }}
  cancel-in-progress: false
```

GitHub glob `v*.*.*` 不等价于 SemVer——`v0.1.0-rc1`、`v01.0.0`、`vfoo.bar.baz` 都会匹配。**严格 SemVer 校验在 preflight Gate 1 完成**（regex `^v[0-9]+\.[0-9]+\.[0-9]+$`），非 SemVer tag 即时 fail。

**仓库侧加固**（与 release.yml 配套）：
- 启用 GitHub Repository → Settings → Tags → tag protection rule，仅维护者能 push `v*` tag
- `publish` job 配 `environment: release-prod`（GH Environments 机制），首次发布时由维护者手动 approve；之后是否保留由后续版本调整

### 4.2 Job 拓扑

```
preflight ─► build (matrix×4) ─► publish ─► finalize
```

四 job 串行；前者失败后者跳过。

### 4.3 preflight gates（ubuntu-latest，~2 min）

| Gate | 检查 | 失败行为 |
|---|---|---|
| Gate 1 | tag 严格 SemVer 正则 + tag 名 `${GITHUB_REF_NAME#v}` 必须等于 `cargo metadata` 的 version + tag commit 必须是 `origin/main` 当前 HEAD 或其祖先 | 立即 fail；不进入 build |
| Gate 2 | `cargo fmt --check` + `cargo clippy --all-targets -- -D warnings` + `cargo test --locked` | 立即 fail |
| Gate 3 | `cargo publish --dry-run --locked` | 立即 fail；不暴露 `CRATES_TOKEN` |

Gate 2 与 ci.yml 的 PR 校验重复，作为最后一道防线（防止 main 上有未跑 CI 的 commit 被打 tag）。

### 4.4 build job（matrix 4 × build + archive smoke）

每平台：
1. `rustup target add <triple>`
2. Linux musl 额外 `apt install musl-tools`
3. `cargo build --release --locked --target <triple>`
4. 平台二进制断言：
   - Linux musl：`file target/.../ais` 含 "statically linked"；`ldd` 返回 "not a dynamic executable"（注意 `ldd` 在静态二进制上 exit code ≠ 0，脚本用 `|| true` 包裹避免 `set -e` 误杀）
   - macOS：`lipo -info` 验证架构与 target 匹配
   - Windows：用 `dumpbin /headers` 或 PowerShell 读 PE header（`file` 在 Windows runner 不可靠）；验证 PE32+ x86_64
5. **Archive smoke**（关键，弥补仅 Linux 人工 walkthrough 的不足）：
   - 把 `ais` / `ais.exe` + `LICENSE-MIT` + `LICENSE-APACHE` + `README.md` 打进 `ais-${VER}-<target>/`
   - 解压回临时目录，跑 `./ais --version` + `./ais --help`
   - 输出必须包含 `0.1.0`；exit 0
   - macOS 上额外 `chmod +x` 后跑（验证打包未丢可执行权限）
6. 计算 `<archive>.sha256`
7. `actions/upload-artifact@v4`（pin SHA 而非 tag）

### 4.5 publish job（ubuntu-latest，依赖 build matrix 全绿；environment: release-prod）

权限：`permissions: { contents: write }`（GH Release 创建用）；`CRATES_TOKEN` secret 仅此 job 可见。

**三段式发布**（顺序写死，比"先 cargo publish 后 GH Release"更稳）：

1. **Step A — GH Release draft 上传 artifacts**
   - 下载所有 build artifacts → 汇总 `SHA256SUMS`
   - `gh release create $TAG --draft --prerelease --notes-file release-notes.md`，上传 4 个 archive + `SHA256SUMS`
   - 失败：删 draft，整 workflow fail；什么都没公开
2. **Step B — `cargo publish`**
   - `cargo publish --locked --token $CRATES_TOKEN`
   - 失败处理：先 `cargo search ai-switch --limit 1` 检查 0.1.0 是否已上 crates.io（应对客户端超时但服务端成功的脏抖动）；已上则视为成功继续 Step C，否则 fail + 删 GH draft
3. **Step C — 把 GH Release 从 draft 改为 published**
   - `gh release edit $TAG --draft=false`
   - 失败：crate 已不可逆，但 draft 仍在；维护者手工 `gh release edit` 补救

**关键不变量**：
- Step A 不公开任何东西
- Step B 是不可逆点；只有 Step A 成功才进入 Step B
- Step C 失败时 crates.io 可见 / GH Release 不可见（最差状态），可手工补 Step C 而无需 cargo yank

### 4.6 finalize job（阻塞，但 cargo publish 已不可逆）

- 轮询 `cargo search ai-switch --limit 1` 直到 0.1.0 出现（最长 5 min）
- ephemeral container（`rust:1-slim`）跑 `cargo install ai-switch --version 0.1.0` 后 `ais --version`
- 失败：workflow exit non-zero（让维护者明确感知），但因 cargo publish 已不可逆，回滚途径仅 `cargo yank`（**`cargo yank` 不是 unpublish**，只阻止新版本解析，已下载的 crate 仍可用）

§12 验收门槛与此 job 联动：finalize fail 不允许 Plan C 视为完成，必须人工 `cargo install` smoke 通过 + 决定是否 yank。

---

## 5. Cargo.toml metadata

```toml
[package]
name          = "ai-switch"
version       = "0.1.0"
edition       = "2024"
authors       = ["YoungHong1992"]
license       = "MIT OR Apache-2.0"
description   = "Claude Code 配置切换工具：profile = 标准 settings.json 文件，零 lock-in"
repository    = "https://github.com/YoungHong1992/ai-switch"
homepage      = "https://github.com/YoungHong1992/ai-switch"
documentation = "https://github.com/YoungHong1992/ai-switch#readme"
readme        = "README.md"
keywords      = ["claude", "claude-code", "cli", "tui", "config"]
categories    = ["command-line-utilities", "development-tools"]
include = [
    "src/**/*.rs",
    "Cargo.toml",
    "Cargo.lock",
    "README.md",
    "LICENSE-MIT",
    "LICENSE-APACHE",
    "CHANGELOG.md",
]
```

**`rust-version` 字段决议**：**不写**。理由：
- edition 2024 已隐含 Rust 1.85+
- 写 `rust-version = "1.85"` 即承诺 MSRV，需 CI 矩阵跑 1.85 build/test，会膨胀 ci.yml
- Public Preview 期 toolchain 承诺收紧未到时机，留给 v1.0.0

**`Cargo.lock` 进 tarball**：binary crate 的 reproducible install 需要它；`cargo install` 默认会读 lockfile（除非用户加 `--no-lockfile-update`）。

**LICENSE 调整**：
- 新增 `LICENSE-MIT`（标准 MIT 模板，holder = YoungHong1992，year = 2026）
- 新增 `LICENSE-APACHE`（apache.org/licenses/LICENSE-2.0 plain text，固定一次抓取）
- 删除旧 `LICENSE`（Rust 生态双授权惯例：仅保留 `LICENSE-MIT` + `LICENSE-APACHE`，root 不放总览文件；与 Codex 建议反向，本决议保留）

**`include` 白名单语义**：crates.io tarball 不含 `tests/`、`docs/`、`.github/`、`target/`；下游 `cargo install` 拉取最小集。

**authors**：仅 GitHub 用户名，不暴露邮箱（你的决议）。crates.io 上游联系走 GitHub issue。

**`cargo publish --dry-run` 期望**：无 warning、tarball < 1MB、文件列表与 `include` 一致。

**Crate name 占用核查（T0 阻塞门）**：Plan C 第 0 步 `cargo search ai-switch --limit 5` + 浏览 crates.io 网页确认未被占用。占用则停 Plan C 与作者协商 fallback 名（候选：`aiswitch` / `ais-cli` / `claude-switch`）。

---

## 6. Provider Catalog Sweep 方法论

**目标**：5 个内置 provider 的 4 字段（`anthropic_base_url` / `openai_base_url` / `auth` / `models_endpoint_path`）按官方文档校准；deepseek + anthropic-official 真账号亲测 wizard 全流程。

### 6.1 字段校准锚点

| provider id | 校准来源（按优先级） |
|---|---|
| `anthropic-official` | docs.anthropic.com → API Reference → Authentication（确认 `x-api-key`）+ base URL |
| `deepseek` | api-docs.deepseek.com → Anthropic API（兼容端点）+ Models endpoint（OpenAI 兼容） |
| `openrouter` | openrouter.ai/docs → 是否暴露 Anthropic 兼容端点；若否，从默认 catalog 移除 |
| `kimi` | platform.moonshot.cn → API 文档 → Anthropic 兼容路径 |
| `glm` | bigmodel.cn → 开放平台 API → Anthropic 兼容路径 |

### 6.2 处置规则

1. **官方文档明确支持 Anthropic 兼容端点** → 进默认 catalog，字段照填
2. **官方文档明确不支持** → 从默认 catalog 移除。**重要约束**：与上游 §5.3 一致，V1 自定义 provider 必须暴露 `anthropic_base_url` 才能用于 Claude profile；不向 README 提供"OpenAI-only provider 用 providers.toml 自己加"这条 escape hatch（会误导用户）。被移除的 provider 仅在 README FAQ 简短解释"为什么没有"
3. **官方文档不清晰 / URL 不稳定** → 移除 + 在 CHANGELOG Known limitations 标注 "deferred"

### 6.3 Sweep 产出（归档要求）

每 provider 的 sweep 结果**写进 release receipt 文件** `docs/superpowers/release-receipts/v0.1.0.md`（不是仅 PR 描述）：
- 文档 URL
- 文档查阅日期
- 4 字段最终值
- 进 catalog / 移除 / 标 deferred 的处置 + 理由

CHANGELOG `[0.1.0]` 的 Known limitations 也注 "Provider URLs verified against official docs as of 2026-05-XX"。

### 6.4 亲测 walkthrough（deepseek + anthropic-official）

**重要**：Keys 面板入口快捷键是大写 `K`（小写 `k` 是 vim 上移）。

deepseek 完整版：
```
ais → n → 选 deepseek → 加真 key → 选 model → 接受默认名 → 写盘
ais claude <name> --version → exit 0、stdout 含 claude 版本号
ais → e 编辑 → 改 model → 重渲染
ais → K → 改 key value → 弹"已影响 N 个 profile" → 全部重渲染
ais → d → Doctor 6 项全绿
ais → x 删 profile + 删 key
```

anthropic-official 简化版：仅到 wizard 完成 + `ais claude <name> --version` 启动。

**为什么用 `--version` 而非 `-p "echo hi"`**：后者会触发真 LLM 推理输出，结果非 deterministic（模型可能加标点、空行、随机 token），diff 判据不稳。`--version` 是 deterministic CLI flag，纯校验"启动语义"。

### 6.5 安全约束

- 真账号 key 仅在 `AIS_HOME=/tmp/ais-walkthrough-2026-05-01/` 隔离目录使用，结束后整目录 `rm -rf`
- PR 描述与 receipts 文件中的 key 必须脱敏成 `sk-a...fswv` 形式
- 不附带任何未脱敏的截图或日志

---

## 7. 文档：README + CHANGELOG

### 7.1 README 章节骨架

```
# ai-switch
> 一行 tagline + Public Preview 警示横幅
## 是什么 / 不是什么 / 前置依赖（claude CLI 必须已安装并在 PATH）
## 安装
  方式 1：cargo install ai-switch（推荐；命令名 `ais`）
  方式 2：GitHub Release 预编译二进制（4 targets + SHA256SUMS 校验步骤）
## 快速上手（5 步文字 walkthrough）
## 关键设计
  profile = settings.json 快照、escape hatch、索引文件、credentials 0600
## 配置目录树
## Doctor 6 项检查清单
## FAQ
## 卸载（cargo uninstall + rm -rf ~/.ai-switch/）
## License: MIT OR Apache-2.0
## Rust library API 状态：内部接口，0.x 不受 SemVer 约束
```

**README 不放**：截图、asciicast、详尽 troubleshooting。

**README 内的链接策略**：所有链 `docs/superpowers/specs/*` 的位置都用 GitHub absolute URL（`https://github.com/YoungHong1992/ai-switch/blob/main/docs/...`），因为 crates.io tarball 通过 `include` 白名单已排除 `docs/`，相对链接在 crates.io 详情页会变坏链。

**目标长度**：≤ 300 行 markdown。

### 7.2 README FAQ 必备题

| 题 | 答案要点 |
|---|---|
| crate 叫 ai-switch 但命令名是 ais？ | 是设计如此；Rust 允许；记一句"`cargo install ai-switch` 后用 `ais`" |
| ais 会改我的 ~/.claude/settings.json 吗？ | **不会**；ais 通过 `claude --settings <path>` 启动而不动全局 settings |
| API key 存几份？都是明文吗？ | 两份：`credentials.toml`（0600 on Unix，跨 provider 库）与生成的 `settings_<name>.json`（明文，是 escape hatch 的代价）。V2 评估 keyring |
| wizard 拉模型列表会把 key 发给 provider 吗？ | 会——`/v1/models` 是认证端点；这是模型动态枚举的代价；网络失败 fallback 自由输入 |
| 是否有遥测 / 联网？ | 无遥测；联网仅在 wizard 拉 `/v1/models` 与 Doctor 解析 `claude --version`；其它操作纯本地 |
| OpenRouter / Kimi / GLM 怎么没有 / 有？ | 视 §6 sweep 结果；不可用的 provider 不进默认 catalog；自定义 provider 必须暴露 Anthropic-compatible endpoint |
| 自定义 provider 能用 OpenAI-only endpoint 吗？ | **不能**；ais 的本质是 `claude --settings`，需要 Anthropic-compatible endpoint |
| Windows 上启动语义有差异吗？ | Unix 用 `execvp` 替换进程；Windows 用 spawn + wait + 退出码透传；用户感知一致 |
| `ais e` / key 轮换会保留我手改的 permissions / hooks 字段吗？ | 是；ais 仅重写 `env` 块，其它字段不动（设计 §10） |
| 如何验证 SHA256SUMS？ | `sha256sum -c SHA256SUMS`（Linux/macOS）或 PowerShell `Get-FileHash` 对比 |
| macOS 提示"未签名/被 Gatekeeper 拦截"怎么办？ | V1 不签名；用 `xattr -d com.apple.quarantine ais` 解除隔离；或用 `cargo install` 路径 |
| Windows 把二进制放哪？ | 解压到 PATH 路径或加入 PATH；推荐与 `claude.exe` 同目录 |
| `AIS_HOME` 环境变量？ | 高级/测试逃生口，覆盖 `~/.ai-switch/` 默认根；不建议常驻 shell rc |
| 删除 `~/.ai-switch/` 的后果？ | 所有 profile / key 丢失（settings.json 仍可独立使用，但 ais 看不到）；建议先备份目录 |

### 7.3 CHANGELOG.md 起步

格式：[Keep a Changelog 1.1](https://keepachangelog.com/en/1.1.0/) + SemVer 2.0；常驻 `[Unreleased]` 章节；diff 链接同步维护。

`[0.1.0] - 2026-05-XX` 章节内容覆盖：Public Preview 声明、TUI 五个视图、5 步 wizard、跨 provider key 库 + 批量 re-render、Doctor 6 项、`ais claude <name>` 启动语义。

Known limitations 段落明示：
- credentials.toml 明文（0600 on Unix）
- 无自动更新 / 遥测 / brew/scoop/winget
- Provider URLs verified as of 2026-05-XX
- Rust library API 内部、0.x 不受 SemVer

### 7.4 CHANGELOG 维护契约 + Release notes 策略

- `[Unreleased]` 段落常驻，PR merge 进 main 时同步追加条目
- tag 发布时把 `[Unreleased]` 内容平移到新版本号下、加日期
- release.yml **不**自动改 CHANGELOG.md（避免 workflow 改写历史）

**Release notes 策略**（修订自上一版"接受 --generate-notes 自动列表"）：
- initial release（v0.1.0）的 release-notes.md **手写功能总览**为主，不依赖 `--generate-notes` 自动 PR 列表（首发可能受 Plan A/Plan B 之外的 commit 干扰，且首发是用户的第一印象点）
- 内容：
  - Public Preview 提示（一句）
  - 功能总览（5 行：TUI、wizard、key 库、Doctor、launch 语义）
  - 安装速查（cargo install + 4 个 archive 链接 + SHA256SUMS 链接 + 校验命令）
  - 已知限制（链 CHANGELOG）
  - 完整变更链 GitHub commits/PRs

后续 patch 版本（0.1.x）可改用 `--generate-notes`，因为有上一个 tag 作 baseline。

---

## 8. 验收 walkthrough（§14.4 1-6）

### 8.1 范围与执行者

仅 Linux 由 Plan C 作者亲测；macOS / Windows 仅靠 ci.yml（PR 阶段 cargo test）+ release.yml build job 的 archive smoke（每平台 `ais --version` / `ais --help`）把关。Linux 亲测 + 跨平台 archive smoke 一起构成 V1 的最低验收基线。

### 8.2 执行顺序

```
项 1（build）→ 项 2（TUI 进出）→ 项 6（preflight Doctor）
→ 项 3（wizard + 裸 claude）→ 项 4（ais claude 等价）
→ 项 5（key 轮换批量同步）→ 项 6（最终 Doctor）
```

Doctor 跑两次：preflight 暴露环境问题；最终是验收主证据。

### 8.3 准备

- 测试主机：当前 Linux 工作机
- 隔离目录：`AIS_HOME=/tmp/ais-walkthrough-2026-05-01/`，结束 `rm -rf`
- 真账号：1 个 deepseek key（仅本地、不入 commit / 截图）
- 记录 `claude --version` 到回执

### 8.4 通过判据（逐项）

| 项 | 通过判据 |
|---|---|
| 1 | 本地 `cargo build --release` ok + ci.yml 三平台绿 |
| 2 | `ais` → `q` 后 `stty -a` 与启动前 diff 为空、无 raw mode 残留 |
| 3 | 生成的 settings 路径裸跑 `claude --settings <path> --version` exit=0、stdout 含 claude 版本号 |
| 4 | `ais claude <name> --version` 与项 3 输出 exit code 等价；ais 自身**不**写任何 stdout/stderr（设计 §8 契约："ais 不在 stdout/stderr 写任何东西"） |
| 5 | TUI `K`（大写）改 key value → settings.json `jq` 差异**仅** `env.ANTHROPIC_API_KEY` 行变化 |
| 6 | Doctor 6 项均显示 PASS |

判据用 `--version` 而非 `-p "echo hi"` 的理由见 §6.4。

任一项失败 → walkthrough 终止；定位 + 修复进同一 PR；修完重跑全部 6 项（不允许 partial 通过）。

### 8.5 回执归档

回执双份：

1. **PR 描述**（GitHub UI 可见，方便 review）
2. **`docs/superpowers/release-receipts/v0.1.0.md`**（仓库归档；后续版本同目录累加）

回执模板关键字段：Host / **tag `v0.1.0`** / claude version / AIS_HOME / 日期；六项每项一段含命令 + 判据 + 输出快照（脱敏）；末尾 `rm -rf` cleanup 确认。

**不写 commit sha**：squash merge 后 main 的 sha 与 PR 分支不同；写 tag 名（tag 是确定的、不可变的）替代。receipt 在 PR 内提交时是"草稿状态"（tag 字段留 `v0.1.0 (pending)`），merge 后由 release.yml 触发的脚本或人工补一个修订 commit 把 `(pending)` 去掉——或者更简单：merge 后再单独提一个 commit 完成 receipt，不挡 release。**采用后者**：PR 不强制 receipt 完整，receipt 文件在 merge 后单独 commit；release.yml 不依赖 receipt 文件。

### 8.6 Sweep 记录归档

§6.3 的 5 个 provider sweep 记录也写进同一 receipt 文件（章节 "Provider Catalog Sweep"），与 walkthrough 同源归档。

---

## 9. 执行编排：方案 A（单 PR + preflight gates）

### 9.1 分支与 PR

- 单分支 `feature/plan-c-release`，单 PR 合入 main
- PR 包含：catalog sweep（src/catalog.rs）、Cargo.toml metadata、双 LICENSE 文件、README、CHANGELOG、release.yml、release-notes.md 模板、（可选）`--help` / Doctor / 快捷键文案微调
- PR 描述粘 sweep 记录 + walkthrough 回执（脱敏）；正式 receipt 文件可在 merge 后单独 commit
- merge 后在 main 上手动打 tag `v0.1.0`

### 9.2 内部任务顺序（plan 阶段拆分参考）

```
T0  cargo search ai-switch 占用核查（阻塞门）
T1  Cargo.toml metadata + 双 LICENSE + 删除旧 LICENSE
T2  catalog sweep + 必要的 src/catalog.rs 调整 + 单元测试同步
T3  README 撰写（含 FAQ 14 题）
T4  CHANGELOG.md 起步
T5  release.yml 编写 + preflight gates + 三段式 publish + archive smoke
T6  --help / Doctor / 快捷键文案对照（核对 K vs k 等已有偏差）
T7  Linux walkthrough 执行 + receipt 文件草稿
T8  PR 提交 + review + merge
T9  receipt 文件正式 commit（去 `(pending)`）
T10 打 tag → release.yml 全自动三段式发布
T11 finalize cargo install 实测；如失败启动手工补救（含 yank 评估）
```

T0 失败立即停 Plan C 与作者协商；其余 T 按顺序执行。

### 9.3 安全网

- preflight 3 个 gates（§4.3）+ build matrix 4 个 archive smoke（§4.4）
- publish job 三段式（§4.5）：Step A 仅 draft 不公开；Step B 是不可逆点；Step C 公开
- walkthrough 在 release 之前；任何项 fail 修复后重跑
- crates.io publish 不可回滚；事后唯一手段是 `cargo yank`（**仅阻新解析，不撤销已下载**）+ 立刻发 `0.1.1` 补丁
- protected tag 规则 + `release-prod` GitHub Environment 手动 approve（仅首次发布）

---

## 10. 风险登记

| # | 风险 | 缓解 |
|---|---|---|
| C1 | crate name `ai-switch` 已被占用 | T0 阻塞门；占用则与作者协商 fallback 名 |
| C2 | `cargo publish` 后发现严重 bug | preflight + walkthrough + archive smoke + release-prod environment approve 是发布前最后防线；事后 `cargo yank --version 0.1.0` + 立发 `0.1.1`；明示 yank ≠ unpublish |
| C3 | tag 误推触发误发 | preflight Gate 1 严格 SemVer regex + tag commit 必须 main 祖先；protected tag 规则；release-prod environment 手动 approve；preflight ~2 min Cancel 窗口 |
| C4 | provider 文档抓取过时 | CHANGELOG / README FAQ 注明 verification 日期；Doctor 不联网校准 |
| C5 | musl artifact 实测非真静态 | build job 的 `ldd` + `file` 断言（用 `\|\| true` 避开 set -e 误杀）；失败则 README "static" 措辞改成 "prebuilt" 或切回 gnu |
| C6 | macOS 两个 runner 跨架构错位 | 显式 `--target` + `lipo -info` 断言匹配；archive smoke 在产生平台上跑 |
| C7 | macOS Gatekeeper 阻挡未签名二进制 | README FAQ 写 `xattr -d com.apple.quarantine` 解法或转用 cargo install |
| C8 | Windows zip 解压后 PATH 错 | README 安装节提示"解压到 PATH 路径或加入 PATH"；archive smoke 用 PowerShell 跑 `ais.exe --version` |
| C9 | crates.io ok / GH Release 失败的不一致 | 三段式 publish：Step A draft 上传 → Step B cargo publish → Step C draft 改 published；最差状态 crate 可见 / Release draft 可补救 |
| C10 | 真账号 key 在 walkthrough 泄漏 | 全程不截图；回执 `sk-a...fswv` 脱敏；`AIS_HOME` 隔离 + 跑完即删；PR review 自审 |
| C11 | `cargo install ai-switch` 命令名歧义 | README 安装节首段明确 "crate `ai-switch`，命令 `ais`"；FAQ 单独一题 |
| C12 | `cargo publish` 网络抖动重发变 duplicate version | Step B 失败先 `cargo search ai-switch` 检查是否已上 crates.io；若已上视为成功；token scope 仅 publish；首发后评估降权/轮换 |
| C13 | release.yml artifact 命名硬编码导致下版本错误 | workflow 全部从 `${GITHUB_REF_NAME#v}` 派生 `${VER}`；不在 yaml 里写字面量 `0.1.0` |
| C14 | README 链 docs/ 在 crates.io 详情页变坏链 | 全部用 GitHub absolute URL |
| C15 | walkthrough 输出 diff 因 LLM 不确定性误失败 | 项 3/4 用 `--version` 而非 `-p`；项 4 改判 exit code 等价 + ais 不写 stdout/stderr |
| C16 | 回执 commit sha 与 tag sha 不一致（squash merge） | receipt 写 tag `v0.1.0`，不写 sha；merge 后单独 commit 完成 receipt |
| C17 | `Cargo.lock` 不在 tarball 导致 cargo install 不可复现 | `include` 白名单显式加 `Cargo.lock` |
| C18 | third-party action 升级被劫持 | release.yml 中所有 third-party action `uses:` pin commit SHA，不 pin tag；尤其 `actions/upload-artifact`、`softprops/action-gh-release` 等 |
| C19 | docs.rs 暴露 lib API 引发"以为 public"误用 | README + CHANGELOG 明示 Rust library API 内部、不受 SemVer；考虑 `[lib] doc = false` 关 docs.rs（待 plan 阶段评估副作用） |

---

## 11. 决议汇总

| # | 决议 |
|---|---|
| D1 | 版本号 `0.1.0`，定位 Public Preview |
| D2 | GitHub Release + crates.io 同时上线 |
| D3 | 4 targets：linux musl x86_64、macos x86_64、macos aarch64、windows msvc x86_64 |
| D4 | 5 个 provider 全部 sweep；deepseek + anthropic-official 真账号亲测；不可信项移除而非标 experimental |
| D5 | §14.4 1-6 仅 Linux 亲测；macOS / Windows 由 release.yml 的 archive smoke 把关 |
| D6 | tag 推送即三段式自动发布；preflight 3 gates；release-prod environment 手动 approve（仅首次发布） |
| D7 | README 中等深度（无截图 / asciicast）；CHANGELOG.md Keep a Changelog 1.1 |
| D8 | 执行编排：方案 A，单 PR + 单 tag |
| D9 | Cargo.toml authors 仅 GitHub 用户名，不暴露邮箱 |
| D10 | LICENSE 双授权（MIT + Apache-2.0），新增 LICENSE-* 两份；删除旧 root LICENSE（Rust 惯例） |
| D11 | Release notes：initial 手写功能总览；后续 patch 改用 `--generate-notes` |
| D12 | walkthrough 回执双份：PR 描述 + `docs/superpowers/release-receipts/v0.1.0.md` 仓库归档；不写 commit sha，写 tag 名 |
| D13 | crates.io tarball 不含 `tests/` / `docs/` / `.github/` / `target/`；含 `Cargo.lock` |
| D14 | 不写 `rust-version` 字段；MSRV 由 edition 2024 隐含 |
| D15 | walkthrough item 3/4 用 `--version` 替代 `-p "echo hi"`，避免 LLM 输出非 deterministic |
| D16 | Rust library API 在 0.x 期间为内部接口、不受 SemVer 约束；README/CHANGELOG 明示 |
| D17 | release.yml 内全部 version 字面量从 `${GITHUB_REF_NAME#v}` 派生 |
| D18 | release.yml 三段式 publish（draft → cargo publish → publish release）；不采用"先 cargo 后 GH" |
| D19 | Sweep 结果归档到 receipt 文件，不仅 PR 描述 |
| D20 | release.yml 内 third-party action `uses:` pin commit SHA |

---

## 12. 验收门槛（Plan C 完成定义）

Plan C 完成 = 全部满足：

1. main 上有 tag `v0.1.0`，对应 commit 包含 §2.1 列出的全部产物（receipt 可在 tag 之后单独 commit）
2. GitHub Release `v0.1.0` 已 publish（`--prerelease` 标记），4 个 archive + `SHA256SUMS` 可下载；每 archive 含 `ais` + `LICENSE-MIT` + `LICENSE-APACHE` + `README.md`
3. crates.io 上 `ai-switch` 0.1.0 可见；release.yml 的 finalize job 在 ephemeral 环境 `cargo install ai-switch --version 0.1.0` 后 `ais --version` 输出 `0.1.0`（finalize 失败则人工 `cargo install` smoke 必过 + 决定是否 yank）
4. Linux walkthrough §14.4 1-6 全部 PASS，回执已写入 `docs/superpowers/release-receipts/v0.1.0.md`
5. CHANGELOG.md `[0.1.0]` 条目齐备，`[Unreleased]` 段落保留
6. README 通过自审：安装两条路径可用、quickstart 可在 fresh 环境复现、FAQ 涵盖 §10 全部 C-编号风险条目、所有 docs/ 链接为 GitHub absolute URL

任一未满足 → Plan C 未完成；不允许"差不多就发"。

---

## 13. V1 之后的衔接（仅留位，不实现）

- v0.2：截图 / asciicast / 更详尽 FAQ；评估 `curl | sh` 安装脚本；可能 brew tap
- v0.3：keyring 评估、Doctor 自动修复、MSRV gate
- v1.0.0：CLI / 配置兼容性承诺；Rust library API 决定是否对外公开；多工具（ais codex / ais cursor）开始评估

Plan C 不做也不挡这些后续路径。

---
