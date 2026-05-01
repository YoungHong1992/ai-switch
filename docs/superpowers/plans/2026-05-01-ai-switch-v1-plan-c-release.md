# ai-switch V1 Plan C — Release (M6) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 ai-switch V1 从"代码功能完整"推到"v0.1.0 Public Preview 可发布"，同步上线 GitHub Release（4 个预编译二进制）与 crates.io。

**Architecture:** 单分支 `feature/plan-c-release` + 单 PR 落地全部产物（catalog sweep / Cargo.toml metadata / 双 LICENSE / README / CHANGELOG / release.yml / receipt 草稿 / 文案对照）；merge 进 main 后人工打 tag `v0.1.0`，由 release.yml 三段式自动发布（draft → cargo publish → publish release）。

**Tech Stack:** Rust 2024 edition · GitHub Actions · crates.io · `gh` CLI · `cargo` workflow · ratatui (existing)。

**Spec：** [docs/superpowers/specs/2026-05-01-ai-switch-v1-plan-c-release-design.md](../specs/2026-05-01-ai-switch-v1-plan-c-release-design.md)

---

## File Structure

| 操作 | 路径 | 责任 |
|---|---|---|
| 新建 | `LICENSE-MIT` | MIT 许可证文本（holder=YoungHong1992, 2026） |
| 新建 | `LICENSE-APACHE` | Apache-2.0 plain text（apache.org 官方版本） |
| 删除 | `LICENSE` | 旧单授权文件，被双授权两份替换 |
| 修改 | `Cargo.toml` | 补 crates.io 元数据（authors / keywords / categories / homepage / documentation / readme / `include` 白名单）；不写 `rust-version` |
| 修改 | `src/catalog.rs` | 5 个内置 provider 的 4 字段按 sweep 结果调字面量；可能移除 OpenRouter |
| 修改 | `src/catalog.rs` 单元测试 | 与 sweep 后的 catalog 对齐（如条目数 / OpenRouter 测试是否保留） |
| 重写 | `README.md` | Public Preview 说明 / 安装 / quickstart / 关键设计 / Doctor / FAQ / 卸载 / License / lib API 状态 |
| 新建 | `CHANGELOG.md` | Keep a Changelog 1.1 + SemVer 2.0；含 `[Unreleased]` 与 `[0.1.0]` |
| 新建 | `.github/workflows/release.yml` | tag 触发；preflight 3 gates + matrix×4 archive smoke + 三段式 publish + finalize |
| 新建 | `docs/superpowers/release-receipts/v0.1.0.md` | walkthrough 回执 + sweep 记录归档（先草稿，merge 后正式 commit） |
| 微调（如有出入） | `src/tui/views/profiles.rs` 等 | TUI 快捷键文案与代码对齐（已在 widgets.rs:176 验证 Help 写 `K`，footer 待核） |

---

## Task 0：crate name 占用核查（阻塞门）

**Files:** 无。

- [ ] **Step 1：在仓库根目录执行 cargo search 检查**

```bash
cargo search ai-switch --limit 5
```

预期：列出的 crates 中**不**包含 name 完全匹配 `ai-switch` 的条目。如包含→**立即停止 Plan C**，回到对话与作者协商 fallback 名（候选：`aiswitch` / `ais-cli` / `claude-switch`）。

- [ ] **Step 2：浏览 crates.io 网页二次确认**

打开 `https://crates.io/crates/ai-switch`。预期：`404 Not Found` 或 "Crate not found"。

- [ ] **Step 3：将核查日期 + 输出片段记录到本地草稿（暂存到任一临时位置）**

记录格式：

```
T0 crate name verification — 2026-05-01
$ cargo search ai-switch --limit 5
<output>
crates.io page: 404
=> name available, proceed Plan C
```

T7 写 receipt 文件时复用此条。

---

## Task 1：双 LICENSE 文件 + Cargo.toml metadata

**Files:**
- Create: `LICENSE-MIT`
- Create: `LICENSE-APACHE`
- Delete: `LICENSE`
- Modify: `Cargo.toml`

- [ ] **Step 1：创建分支 `feature/plan-c-release`**

```bash
git checkout -b feature/plan-c-release
```

- [ ] **Step 2：写 `LICENSE-MIT`（MIT 标准模板）**

文件内容：

```
MIT License

Copyright (c) 2026 YoungHong1992

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

- [ ] **Step 3：抓 `LICENSE-APACHE`（apache.org 官方 plain text）**

```bash
curl -fsSL https://www.apache.org/licenses/LICENSE-2.0.txt -o LICENSE-APACHE
```

校验：

```bash
head -3 LICENSE-APACHE
# 应包含 "Apache License" + "Version 2.0, January 2004"
wc -l LICENSE-APACHE
# 应在 200 行左右
```

- [ ] **Step 4：删除旧 `LICENSE`**

```bash
git rm LICENSE
```

- [ ] **Step 5：替换 `Cargo.toml` `[package]` 段**

当前 `Cargo.toml` 见仓库根；把 `[package]` 段替换为：

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

注意：保留 `Cargo.toml` 后面的 `[[bin]]` / `[lib]` / `[dependencies]` / `[profile.release]` 等段不变。

- [ ] **Step 6：跑 cargo metadata 校验**

```bash
cargo metadata --no-deps --format-version 1 | grep -oE '"description":"[^"]+"' | head -1
```

预期：输出 `"description":"Claude Code 配置切换工具：profile = 标准 settings.json 文件，零 lock-in"`。

- [ ] **Step 7：跑 `cargo build --release` 确认 Cargo.toml 调整不破坏构建**

```bash
cargo build --release --locked
```

预期：`Finished release [...]`，零 warning（除已有的）。

- [ ] **Step 8：轻量 metadata 校验**（不跑 publish dry-run，因 README/CHANGELOG 此时尚未落盘）

```bash
# 校验 Cargo.toml 语法 + license / categories 等字段被 cargo 接受
cargo metadata --no-deps --format-version 1 > /dev/null
echo "metadata parse ok"

# 校验当前会被打包的文件清单
cargo package --list --allow-dirty > /tmp/package-list.txt
echo "Files that WILL be packaged:"
cat /tmp/package-list.txt
```

预期：
- `cargo metadata` 退出码 0，无 warning
- `package-list.txt` **不含** `tests/`、`docs/`、`.github/`、`target/`、旧单文件 `LICENSE`
- `package-list.txt` **含** `Cargo.toml.orig`、`Cargo.toml`、`Cargo.lock`、`src/main.rs` 等；`README.md`、`CHANGELOG.md`、`LICENSE-MIT`、`LICENSE-APACHE` 此时尚未创建（`include` 配置已声明但磁盘上还没有），cargo 会报这几项缺失——是预期，**真正的 `cargo publish --dry-run` 阻塞校验在 T4 Step 2 完成后才跑**

- [ ] **Step 9：commit**

```bash
git add LICENSE-MIT LICENSE-APACHE Cargo.toml
# LICENSE 已在 Step 4 通过 git rm 删除并 stage，无需再 git rm
git commit -m "chore(metadata): 双 LICENSE 落盘 + Cargo.toml crates.io 元数据补齐

- 新增 LICENSE-MIT (MIT 标准模板，holder=YoungHong1992 2026)
- 新增 LICENSE-APACHE (apache.org 官方 plain text)
- 删除旧单授权 LICENSE，改双授权 Rust 惯例
- Cargo.toml 补 keywords / categories / homepage / documentation / readme
- include 白名单：仅 src/Cargo.toml/Cargo.lock/README/LICENSE-*/CHANGELOG
- 不写 rust-version 字段（edition 2024 已隐含 1.85+）"
```

---

## Task 2：Provider catalog sweep + 代码对齐

**Files:**
- Modify: `src/catalog.rs`（按 sweep 结果调字面量；可能移除 OpenRouter）
- Modify: `src/catalog.rs` 单元测试模块

### 2.A — 五家逐个 sweep（文档校准）

逐家执行下列查阅与判断；每家**记录到本地 sweep 草稿**（与 T0 同位置临时文件，T7 复用）：

```
provider id: <id>
official doc URL: <url>
checked at: 2026-05-01
anthropic_base_url: <value or "not exposed">
openai_base_url:    <value or "n/a">
auth:               Bearer | XApiKey
models_endpoint_path: <path>
verdict: keep | remove | deferred
reason: <一行>
```

- [ ] **Step 1：sweep `anthropic-official`**
  - 来源：`https://docs.anthropic.com/en/api/getting-started`
  - 校准：`anthropic_base_url` = `https://api.anthropic.com`；auth = `XApiKey`（`x-api-key` header）；`openai_base_url` = `None`（不暴露 OpenAI 兼容端点）
  - verdict：保留

- [ ] **Step 2：sweep `deepseek`**
  - 来源：`https://api-docs.deepseek.com/guides/anthropic_api`、`https://api-docs.deepseek.com/api/list-models`
  - 校准：`anthropic_base_url` 与 `openai_base_url` 是否仍是当前 catalog 写的值
  - verdict：保留

- [ ] **Step 3：sweep `openrouter`**
  - 来源：`https://openrouter.ai/docs`
  - 校准重点：是否暴露 Anthropic 兼容端点（`/anthropic/v1` 或类似）；当前 catalog 把 `anthropic_base_url` 写成 `https://openrouter.ai/api/v1`，与 OpenAI 兼容端点同一 URL，**这是上游设计 §5.2 的占位，不正确**
  - verdict：若官方明确不支持 → `remove`；若官方 docs 不清晰 → `deferred`

- [ ] **Step 4：sweep `kimi`**
  - 来源：`https://platform.moonshot.cn/docs/api/anthropic` 或类似路径
  - 校准 4 字段
  - verdict：保留 / remove / deferred

- [ ] **Step 5：sweep `glm`（智谱）**
  - 来源：`https://open.bigmodel.cn/dev/howuse` → Anthropic 兼容端点段
  - 校准 4 字段
  - verdict：保留 / remove / deferred

### 2.B — 修改 `src/catalog.rs`

- [ ] **Step 6：根据 sweep verdict 写测试（按 verdict 分支处理）**

测试形态由每家 provider 的 verdict 决定，**不**强制要求"先失败"。在 `src/catalog.rs` 的 `#[cfg(test)] mod tests` 块尾部追加适配测试：

**verdict = remove**（如 sweep 后决定移除某 provider）：

```rust
#[test]
fn <provider>_removed_until_anthropic_endpoint_confirmed() {
    assert!(find("<provider>").is_none(),
        "<provider> 在 v0.1.0 sweep 后未确认 Anthropic 兼容端点，应从默认 catalog 移除");
}
```

**verdict = keep + 字段需更新**（sweep 发现当前字面量与官方文档不一致，例如 OpenRouter 的 anthropic_base_url 占位错误需改为真实值）：

```rust
#[test]
fn <provider>_anthropic_endpoint_distinct_from_openai() {
    let p = find("<provider>").unwrap();
    let a = p.anthropic_base_url.unwrap();
    let o = p.openai_base_url.unwrap();
    assert_ne!(a, o, "anthropic 与 openai base_url 不应相同（占位错误）");
    assert_eq!(a, "<sweep 后的 anthropic_base_url 实际值>");
}
```

**verdict = keep + 字段无需更新**（sweep 后字面量本就正确）：跳过新增测试；现有测试足够覆盖。

**verdict = deferred**（文档不清晰）：等同 remove，但在 commit message 与 receipt 中标注 "deferred"。

- [ ] **Step 7：跑测试确认状态符合预期**

```bash
cargo test --quiet --lib catalog::tests
```

预期：
- 若 verdict = remove / 字段需更新且新测试已写：测试**失败**（Step 8 改实现后通过）
- 若 verdict = keep + 字段无需更新：测试**全部通过**（无需 Step 8）

记录每条 verdict 对应的实际预期到 sweep 草稿文件。

- [ ] **Step 8：按 sweep 结果改 `builtins()` 函数**

仅改字面量；如某 provider verdict = remove，删除其 `Provider { ... }` 块。例如假设 sweep 决定移除 OpenRouter：

```rust
// 删除 src/catalog.rs 中 OpenRouter 那段（约 47-54 行）
// Provider {
//     id: "openrouter".into(),
//     ...
// },
```

其它 provider 按 sweep 结果调 `anthropic_base_url`、`openai_base_url`、`auth`、`models_endpoint_path`。

- [ ] **Step 9：跑全部 catalog 测试 + 全仓测试**

```bash
cargo test --quiet --lib catalog::tests
cargo test --quiet
```

预期：全部 PASS。如 `all_builtins_have_unique_ids` 等老测试因移除条目而需调整 expected 数量，更新它们。

- [ ] **Step 10：更新 `src/catalog.rs` 顶部注释**

把当前的 `/// 实际 URL 与端点路径在 v0.1.0 发布前会再 sweep 一次官方文档校准；当前值用于跑通整套数据流。` 改为：

```rust
/// Built-in provider catalog (compile-time constant via fn).
///
/// URLs verified against official docs as of 2026-05-01. Upstream changes
/// are not auto-tracked; users override via providers.toml.
pub fn builtins() -> Vec<Provider> {
```

- [ ] **Step 11：commit**

```bash
git add src/catalog.rs
git commit -m "feat(catalog): v0.1.0 sweep — URLs verified against official docs

- anthropic-official: <verdict>
- deepseek:           <verdict>
- openrouter:         <verdict>
- kimi:               <verdict>
- glm:                <verdict>

Sweep 详细记录归档到 docs/superpowers/release-receipts/v0.1.0.md (T7)。"
```

替换 `<verdict>` 为 sweep 实际结论（保留 / 移除 / deferred）。

---

## Task 3：README.md（Public Preview 版）

**Files:**
- Rewrite: `README.md`

- [ ] **Step 1：备份当前 README.md**

```bash
cp README.md /tmp/README.md.bak
```

- [ ] **Step 2：用下列骨架写 README.md**

完整内容（替换 `<...>` 占位为实际内容）：

```markdown
# ai-switch

> Claude Code 配置切换工具：profile = 标准 settings.json，零 lock-in。

> ⚠ **Public Preview (v0.1.0)** — CLI、磁盘配置、provider catalog、Rust library API 在 0.1.x 期间均可能调整；不强承诺向后兼容。

## 是什么 / 不是什么

ai-switch（命令名 `ais`）做这五件事，仅此而已：

1. 管 `~/.ai-switch/claude/settings_<name>.json`：每个 profile 就是一份独立的 Claude Code `settings.json`
2. 按 (provider × key × model) 三元组生成这些 settings.json
3. 跨 provider 的 key 库：编辑一个 key 自动重渲染所有引用它的 profile
4. TUI 操作（profiles / providers / keys / doctor / wizard）
5. `ais claude <name>` 启动 Claude Code，等价于 `claude --settings <path-to-settings>`

**不做**：自动更新、遥测、brew/scoop/winget 分发、多设备同步、shell 全局 alias、settings.json 高级字段（permissions / hooks 等用户自行编辑）。详见 [设计文档](https://github.com/YoungHong1992/ai-switch/blob/main/docs/superpowers/specs/2026-04-29-ai-switch-design.md) §15。

**前置依赖**：`claude` CLI 必须已安装并在 PATH。安装见 [Anthropic 官方文档](https://docs.anthropic.com/en/docs/claude-code/quickstart)。

## 安装

### 方式 1：cargo install（推荐）

    cargo install ai-switch

> crate 名是 `ai-switch`，安装后命令名是 `ais`（这是有意为之）。

### 方式 2：GitHub Release 预编译二进制

到 [Releases](https://github.com/YoungHong1992/ai-switch/releases) 下载对应平台 archive：

- `ais-v0.1.0-x86_64-unknown-linux-musl.tar.gz`（Linux x86_64，static）
- `ais-v0.1.0-x86_64-apple-darwin.tar.gz`（macOS Intel）
- `ais-v0.1.0-aarch64-apple-darwin.tar.gz`（macOS Apple Silicon）
- `ais-v0.1.0-x86_64-pc-windows-msvc.zip`（Windows x86_64）

校验：

    # Linux / macOS
    sha256sum -c SHA256SUMS

    # Windows PowerShell
    Get-FileHash ais-v0.1.0-*.zip -Algorithm SHA256

解压后把 `ais` / `ais.exe` 放到 PATH 路径（推荐与 `claude` 同目录）。

## 快速上手

1. 跑 `ais` 进 TUI
2. 按 `n` 进 wizard：选 provider → 添加 key → 选 model → 起名 → 写盘
3. 退到主页按 `Enter` 启动 claude（透传 `--settings`）
4. 想脱离 ais 运行：`claude --settings ~/.ai-switch/claude/settings_<name>.json`
5. 想轮换 key：在主页按 `K`（大写）→ 选 key → 编辑 value → ais 自动批量同步所有 profile

## 关键设计

- **profile = settings.json 快照**：生成后是当下 (provider, key, model) 的固化结果；上游变化不会自动追，需主动重渲染（key 轮换是唯一自动批量场景）
- **escape hatch**：任何生成的 `settings_<name>.json` 都能裸跑 `claude --settings <path>`；ais 不锁住任何东西
- **私有索引**：`~/.ai-switch/claude/.ais-index.toml` 记录 profile ↔ (provider, key, model) 关系；丢失不影响启动，但会失去批量轮换能力（Doctor 会提示）
- **credentials.toml**：跨 provider key 库，0600（Unix）；明文存储是 V1 的已知权衡，V2 评估 keyring

## 配置目录

```
~/.ai-switch/
├── credentials.toml        # 跨 provider key 库（0600 on Unix）
├── providers.toml          # 用户自定义 provider；内置 catalog 打在二进制里
└── claude/
    ├── settings_<name>.json
    ├── settings_<name>.json
    └── .ais-index.toml     # ais 私有索引
```

可用 `AIS_HOME` 环境变量覆盖根目录（高级 / 测试逃生口；不建议常驻 shell rc）。

## Doctor

主页按 `d` 进 Doctor 面板，6 项检查：

| # | 检查 | 通过条件 |
|---|---|---|
| 1 | `claude` 在 PATH | `which claude` 返回路径 |
| 2 | Claude Code 版本 | `claude --version` 解析成功 |
| 3 | `~/.ai-switch/` 可读写 | 临时文件创建删除成功 |
| 4 | `credentials.toml` 权限 | Linux/macOS `mode & 0o077 == 0`；Windows 跳过 |
| 5 | `.ais-index.toml` 一致性 | 索引项数 == 实际 settings 文件数；列出孤儿/死链 |
| 6 | Provider 表加载 | `providers.toml` 解析无错；同名覆盖关系列出 |

V1 仅检测 + 报告，不自动修复。

## FAQ

**Q: crate 叫 `ai-switch` 但命令名是 `ais`？**
A: 是设计如此。Rust 的 crate name 与 binary name 可分离；用 `cargo install ai-switch` 安装后用 `ais` 调用。

**Q: ais 会改我的 `~/.claude/settings.json` 吗？**
A: **不会**。ais 通过 `claude --settings <path>` 显式启动，全程不动用户的全局 Claude Code 配置。

**Q: API key 存几份？都是明文吗？**
A: 两份。`~/.ai-switch/credentials.toml` 是跨 provider key 库（Unix 上 0600 权限）；生成的 `settings_<name>.json` 也含明文 key（这是 escape hatch 的代价）。V2 评估 keyring 集成。

**Q: wizard 拉模型列表时会把 key 发给 provider 吗？**
A: 会——`/v1/models` 是认证端点，必须带 key。这是动态枚举模型的代价。网络失败 5 秒超时后 fallback 自由输入。

**Q: 是否有遥测 / 后台联网？**
A: 无遥测。仅在 wizard 拉 `/v1/models` 与 Doctor 解析 `claude --version` 时联网；其它操作纯本地。

**Q: OpenRouter / Kimi / GLM 在内置 catalog 里吗？**
A: <按 T2 sweep 结果填写。例如："anthropic-official 与 deepseek 已校准；OpenRouter 因官方未明确暴露 Anthropic 兼容端点暂未进默认 catalog；Kimi、GLM 已校准。用户可通过 `~/.ai-switch/providers.toml` 添加自定义 provider。"> 注意：自定义 provider 必须暴露 Anthropic 兼容端点（设计 §5.3）；OpenAI-only provider **不**支持。

**Q: Windows 上启动语义有差异吗？**
A: Unix 用 `execvp` 替换进程；Windows 用 spawn + wait + 退出码透传。从用户角度感知一致——同会话、同 env、同模型、同 exit code。

**Q: `ais e` 编辑 / key 轮换会保留我手改的 `permissions` / `hooks` 字段吗？**
A: 会。ais 仅重写 `env` 块，其它字段原样保留。

**Q: 如何验证 SHA256SUMS？**
A: Linux/macOS：`sha256sum -c SHA256SUMS`；Windows PowerShell：`Get-FileHash <archive> -Algorithm SHA256` 后与 SHA256SUMS 对比。

**Q: macOS 提示"未签名 / 被 Gatekeeper 拦截"怎么办？**
A: V1 不签名。两种解法：(1) 用 `cargo install ai-switch` 路径（自行编译，无 Gatekeeper 问题）；(2) 下载二进制后 `xattr -d com.apple.quarantine /path/to/ais` 解除隔离。

**Q: Windows 把二进制放哪？**
A: 解压到任一 PATH 路径或自行加入 PATH。推荐与 `claude.exe` 同目录。

**Q: `AIS_HOME` 是什么？**
A: 高级 / 测试用环境变量，覆盖 `~/.ai-switch/` 默认根。用于隔离测试或多实例。**不**建议常驻 shell rc。

**Q: 删除 `~/.ai-switch/` 的后果？**
A: 所有 profile 与 key 元数据丢失。已生成的 `settings_<name>.json` 仍可独立用 `claude --settings <path>` 启动（escape hatch），但 ais 看不到它们。建议先备份目录或导出 `credentials.toml`。

## 卸载

    cargo uninstall ai-switch
    rm -rf ~/.ai-switch/

或对二进制安装方式：直接删除 `ais` / `ais.exe` 文件 + `rm -rf ~/.ai-switch/`。

## License

Dual-licensed under either:

- [MIT License](LICENSE-MIT)
- [Apache License 2.0](LICENSE-APACHE)

at your option.

## Rust library API 状态

`ai-switch` 仅以 CLI（`ais`）形式对外承诺。`pub` 模块（`ais` library）属于内部实现细节，docs.rs 上虽可见但**0.x 期间不受 SemVer 约束**——任何 patch 版本都可能调整 lib API。如需稳定的库接口，请等待 v1.0.0 评估。
```

- [ ] **Step 3：核对 README**
  - 安装两条路径完整：cargo install + GitHub Release 二进制 + SHA256SUMS 校验
  - 所有指向 `docs/...` 的链接已用 `https://github.com/YoungHong1992/ai-switch/blob/main/docs/...` 形式（spec §7.1 决议 D 配套约束）
  - FAQ 包含 spec §7.2 列出的全部 14 题
  - K vs k 在 quickstart 第 5 步与"关键设计"明示
  - lib API 状态段落存在
  - 行数 ≤ 300

```bash
wc -l README.md
```

- [ ] **Step 4：commit**

```bash
git add README.md
git commit -m "docs(readme): Public Preview 版 README

- 安装两条路径（cargo install + GH Release 二进制 + SHA256SUMS）
- Quickstart 5 步含 K 大写 keys panel 入口
- 关键设计 / 配置目录 / Doctor 6 项 / FAQ 14 题
- 链 docs/specs 用 GitHub absolute URL（crates.io tarball 排除 docs/）
- Rust library API 状态：0.x 内不受 SemVer 约束"
```

---

## Task 4：CHANGELOG.md（Keep a Changelog 1.1）

**Files:**
- Create: `CHANGELOG.md`

- [ ] **Step 1：写 CHANGELOG.md**

```markdown
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-05-XX

Initial Public Preview.

### Added

- TUI with five views: profiles, providers, keys, doctor, wizard
- Five-step profile creation wizard (provider × key × model → settings.json)
- Cross-provider key library (`~/.ai-switch/credentials.toml`, 0600 on Unix)
  with batch re-render: editing a key value automatically updates every
  profile that references it
- Built-in provider catalog: anthropic-official, deepseek, kimi, glm
  (+ openrouter if confirmed by sweep, otherwise documented as user
  override via providers.toml)
- Doctor panel with 6 health checks (claude in PATH, version parse,
  config dir writable, credentials.toml mode, index consistency,
  providers.toml load)
- `ais claude <name>` launch with full args / exit code passthrough
  (Unix `execvp`, Windows spawn + wait)
- `AIS_HOME` env var for overriding the root directory (advanced/testing)
- User-editable `~/.ai-switch/providers.toml` for custom Anthropic-compatible
  providers (overrides built-in catalog by id)

### Known limitations

- `credentials.toml` and generated `settings_<name>.json` store API keys
  in plaintext (0600 on Unix; default ACL on Windows)
- No automatic update mechanism; no telemetry
- No brew / scoop / winget distribution; release channels are
  GitHub Release prebuilt binaries + `cargo install ai-switch`
- Provider URLs verified against official docs as of 2026-05-01;
  upstream changes are not auto-tracked
- Rust library API exposed via crates.io is internal and **not** subject
  to SemVer in the 0.x line; only the `ais` CLI is supported

### Distribution

- crates.io: `cargo install ai-switch` (binary name: `ais`)
- GitHub Release: prebuilt binaries for x86_64-unknown-linux-musl,
  x86_64-apple-darwin, aarch64-apple-darwin, x86_64-pc-windows-msvc

[Unreleased]: https://github.com/YoungHong1992/ai-switch/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/YoungHong1992/ai-switch/releases/tag/v0.1.0
```

`2026-05-XX` 中 `XX` 在 T11 打 tag 时再替换为实际日期。

- [ ] **Step 2：再跑一次 cargo publish dry-run（README + CHANGELOG 都齐了）**

```bash
cargo publish --dry-run --locked 2>&1 | tee /tmp/publish-dryrun-2.log
grep -E "warning|error" /tmp/publish-dryrun-2.log || echo "no warnings/errors"
```

预期：无 warning / error；最终一行 `Packaged: <N> files, <size>`，size < 1MB。

- [ ] **Step 3：commit**

```bash
git add CHANGELOG.md
git commit -m "docs(changelog): CHANGELOG.md 起步 + [0.1.0] Public Preview 条目

格式：Keep a Changelog 1.1 + SemVer 2.0
- Added: TUI / wizard / key 库 / Doctor / launch 语义 / providers.toml
- Known limitations: 明文 key / 无遥测 / 无 brew/scoop/winget /
  URLs verified 2026-05-01 / lib API 不受 SemVer
- Distribution: crates.io + GH Release（4 targets）"
```

---

## Task 5：release.yml 工作流

**Files:**
- Create: `.github/workflows/release.yml`

- [ ] **Step 1：写 release.yml 完整文件**

```yaml
name: release

on:
  push:
    tags: ['v*.*.*']

concurrency:
  group: release-${{ github.ref }}
  cancel-in-progress: false

permissions:
  contents: read

env:
  CARGO_TERM_COLOR: always

jobs:
  preflight:
    name: preflight (gates 1-3)
    runs-on: ubuntu-latest
    steps:
      - name: checkout
        uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2

      - name: gate 1 — strict SemVer + tag commit equals origin/main HEAD
        run: |
          set -euo pipefail
          tag="${GITHUB_REF_NAME}"
          if [[ ! "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
            echo "::error::tag '$tag' is not strict SemVer (expected vMAJOR.MINOR.PATCH)"
            exit 1
          fi
          ver="${tag#v}"
          cargo_ver=$(cargo metadata --no-deps --format-version 1 \
            | python3 -c 'import sys,json;d=json.load(sys.stdin);print(d["packages"][0]["version"])')
          if [[ "$ver" != "$cargo_ver" ]]; then
            echo "::error::tag version $ver != Cargo.toml version $cargo_ver"
            exit 1
          fi
          # 比 is-ancestor 更严：tag commit 必须等于 origin/main 当前 HEAD（防止误打旧 commit）
          git fetch --no-tags --depth=1 origin main
          tag_commit=$(git rev-parse "${tag}^{commit}")
          main_commit=$(git rev-parse origin/main)
          if [[ "$tag_commit" != "$main_commit" ]]; then
            echo "::error::tag commit $tag_commit != origin/main HEAD $main_commit"
            echo "::error::release tags must point to the current main HEAD; rebase or repoint the tag"
            exit 1
          fi

      - name: install rust stable
        uses: dtolnay/rust-toolchain@b3b07ba8b418998c39fb20f53e8b695cdcc8de1b # stable
        with:
          components: rustfmt, clippy

      - name: cache cargo
        uses: Swatinem/rust-cache@98c8021b550208e191a6a3145459bfc9fb29c4c0 # v2.7.7

      - name: gate 2 — fmt + clippy + test
        run: |
          cargo fmt --all -- --check
          cargo clippy --workspace --all-targets -- -D warnings
          cargo test --workspace --all-targets --locked

      - name: gate 3 — cargo publish dry-run
        run: cargo publish --dry-run --locked

  build:
    name: build (${{ matrix.target }})
    needs: preflight
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-musl
            archive: tar.gz
          - os: macos-13
            target: x86_64-apple-darwin
            archive: tar.gz
          - os: macos-14
            target: aarch64-apple-darwin
            archive: tar.gz
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            archive: zip
    steps:
      - name: checkout
        uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2

      - name: install rust target
        uses: dtolnay/rust-toolchain@b3b07ba8b418998c39fb20f53e8b695cdcc8de1b # stable
        with:
          targets: ${{ matrix.target }}

      - name: cache cargo
        uses: Swatinem/rust-cache@98c8021b550208e191a6a3145459bfc9fb29c4c0 # v2.7.7
        with:
          key: ${{ matrix.target }}

      - name: install musl-tools (Linux only)
        if: matrix.target == 'x86_64-unknown-linux-musl'
        run: sudo apt-get update && sudo apt-get install -y musl-tools

      - name: derive version
        id: ver
        shell: bash
        run: echo "ver=${GITHUB_REF_NAME#v}" >> "$GITHUB_OUTPUT"

      - name: cargo build
        run: cargo build --release --locked --target ${{ matrix.target }}

      - name: platform binary assertions (Linux musl)
        if: matrix.target == 'x86_64-unknown-linux-musl'
        run: |
          set -euo pipefail
          file "target/${{ matrix.target }}/release/ais" | tee /tmp/file.txt
          grep -q "statically linked" /tmp/file.txt
          # ldd on a static binary returns non-zero; capture & inspect rather than letting set -e kill us
          ldd_out=$(ldd "target/${{ matrix.target }}/release/ais" 2>&1 || true)
          echo "$ldd_out" | grep -q "not a dynamic executable"

      - name: platform binary assertions (macOS)
        if: startsWith(matrix.target, 'x86_64-apple-darwin') || startsWith(matrix.target, 'aarch64-apple-darwin')
        run: |
          set -euo pipefail
          lipo -info "target/${{ matrix.target }}/release/ais" | tee /tmp/lipo.txt
          case "${{ matrix.target }}" in
            x86_64-apple-darwin)  grep -q "x86_64" /tmp/lipo.txt ;;
            aarch64-apple-darwin) grep -q "arm64" /tmp/lipo.txt ;;
          esac

      - name: platform binary assertions (Windows)
        if: matrix.target == 'x86_64-pc-windows-msvc'
        shell: pwsh
        run: |
          $bin = "target\${{ matrix.target }}\release\ais.exe"
          if (-not (Test-Path $bin)) { throw "binary missing: $bin" }
          # 不依赖 dumpbin (在普通 PS PATH 不一定有)；直接读 PE header 验证 IMAGE_FILE_MACHINE_AMD64 (0x8664)
          $bytes = [System.IO.File]::ReadAllBytes($bin)
          # PE signature offset 在 0x3C 处的 4 字节 (DWORD)
          $peOffset = [System.BitConverter]::ToInt32($bytes, 0x3C)
          # PE\0\0 magic
          if ($bytes[$peOffset] -ne 0x50 -or $bytes[$peOffset+1] -ne 0x45) {
            throw "invalid PE signature at offset $peOffset"
          }
          # IMAGE_FILE_HEADER.Machine 在 PE signature 后 4 字节 (offset PE+4)
          $machine = [System.BitConverter]::ToUInt16($bytes, $peOffset + 4)
          if ($machine -ne 0x8664) {
            throw "expected IMAGE_FILE_MACHINE_AMD64 (0x8664), got 0x$($machine.ToString('X4'))"
          }
          Write-Host "PE header check: x86_64 ok"

      - name: build archive
        shell: bash
        run: |
          set -euo pipefail
          ver="${{ steps.ver.outputs.ver }}"
          target="${{ matrix.target }}"
          dir="ais-v${ver}-${target}"
          mkdir -p "$dir"
          if [[ "$target" == *windows* ]]; then
            cp "target/${target}/release/ais.exe" "$dir/"
          else
            cp "target/${target}/release/ais" "$dir/"
          fi
          cp LICENSE-MIT LICENSE-APACHE README.md "$dir/"
          if [[ "${{ matrix.archive }}" == "zip" ]]; then
            (cd . && 7z a -tzip "${dir}.zip" "$dir")
            archive="${dir}.zip"
          else
            tar -czf "${dir}.tar.gz" "$dir"
            archive="${dir}.tar.gz"
          fi
          # checksum sidecar
          if [[ "$RUNNER_OS" == "macOS" ]]; then
            shasum -a 256 "$archive" > "${archive}.sha256"
          else
            sha256sum "$archive" > "${archive}.sha256"
          fi
          echo "ARCHIVE=$archive" >> "$GITHUB_ENV"

      - name: archive smoke
        shell: bash
        run: |
          set -euo pipefail
          ver="${{ steps.ver.outputs.ver }}"
          target="${{ matrix.target }}"
          dir="ais-v${ver}-${target}"
          smoke=/tmp/smoke
          rm -rf "$smoke" && mkdir -p "$smoke"
          if [[ "${{ matrix.archive }}" == "zip" ]]; then
            7z x "${dir}.zip" -o"$smoke" -y
            bin="$smoke/$dir/ais.exe"
          else
            tar -xzf "${dir}.tar.gz" -C "$smoke"
            bin="$smoke/$dir/ais"
            chmod +x "$bin"
          fi
          out=$("$bin" --version)
          echo "$out"
          echo "$out" | grep -E "^(ais|ai-switch)\s+${ver}" \
            || (echo "::error::archive smoke failed: --version did not match ${ver}"; exit 1)
          "$bin" --help > /dev/null

      - name: upload archive artifact
        uses: actions/upload-artifact@b4b15b8c7c6ac21ea08fcf65892d2ee8f75cf882 # v4.4.3
        with:
          name: archive-${{ matrix.target }}
          path: |
            ais-v${{ steps.ver.outputs.ver }}-${{ matrix.target }}.${{ matrix.archive }}
            ais-v${{ steps.ver.outputs.ver }}-${{ matrix.target }}.${{ matrix.archive }}.sha256
          retention-days: 7

  publish:
    name: publish (3-stage)
    needs: build
    runs-on: ubuntu-latest
    environment: release-prod
    permissions:
      contents: write
    steps:
      - name: checkout
        uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2

      - name: derive version
        id: ver
        run: echo "ver=${GITHUB_REF_NAME#v}" >> "$GITHUB_OUTPUT"

      - name: download all archives
        uses: actions/download-artifact@fa0a91b85d4f404e444e00e005971372dc801d16 # v4.1.8
        with:
          path: dist
          pattern: archive-*
          merge-multiple: true

      - name: aggregate SHA256SUMS
        run: |
          set -euo pipefail
          cd dist
          : > SHA256SUMS
          for f in *.sha256; do
            cat "$f" >> SHA256SUMS
          done
          cat SHA256SUMS

      - name: build release-notes.md
        run: |
          ver="${{ steps.ver.outputs.ver }}"
          cat > /tmp/release-notes.md <<EOF
          # ai-switch v${ver} — Public Preview

          > 0.1.x 期间 CLI / 磁盘配置 / provider catalog / Rust library API 仍可能调整；不强承诺向后兼容。

          ## 功能总览

          - TUI：profiles / providers / keys / doctor / wizard 五视图
          - 5 步 wizard 按 (provider × key × model) 生成 settings.json
          - 跨 provider key 库：编辑一个 key 自动重渲染所有引用它的 profile
          - Doctor 6 项健康检查
          - \`ais claude <name>\`：等价 \`claude --settings <path>\`，全 args / exit code 透传

          ## 安装

          ### cargo install（推荐）

              cargo install ai-switch
              # crate 名 ai-switch，命令名 ais

          ### 预编译二进制

          - \`ais-v${ver}-x86_64-unknown-linux-musl.tar.gz\` (Linux x86_64, static)
          - \`ais-v${ver}-x86_64-apple-darwin.tar.gz\` (macOS Intel)
          - \`ais-v${ver}-aarch64-apple-darwin.tar.gz\` (macOS Apple Silicon)
          - \`ais-v${ver}-x86_64-pc-windows-msvc.zip\` (Windows x86_64)
          - \`SHA256SUMS\`：校验用

          ## 已知限制

          详见 [CHANGELOG.md](https://github.com/YoungHong1992/ai-switch/blob/v${ver}/CHANGELOG.md) 中的 **Known limitations** 段。

          EOF

      - name: stage A — create draft release with archives (idempotent)
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          set -euo pipefail
          tag="${GITHUB_REF_NAME}"
          # 幂等：如有同 tag 已存在的 release（包含 draft），先删（不删 tag）
          if gh release view "$tag" >/dev/null 2>&1; then
            echo "existing release for $tag; deleting (cleanup-tag=false)"
            gh release delete "$tag" --yes --cleanup-tag=false
          fi
          gh release create "$tag" \
            --draft \
            --prerelease \
            --title "ai-switch ${tag} — Public Preview" \
            --notes-file /tmp/release-notes.md \
            dist/*.tar.gz dist/*.zip dist/*.sha256 dist/SHA256SUMS

      - name: install rust stable
        uses: dtolnay/rust-toolchain@b3b07ba8b418998c39fb20f53e8b695cdcc8de1b # stable

      - name: stage B — cargo publish (idempotent)
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CRATES_TOKEN }}
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}  # 失败分支调 gh release delete 需要
        run: |
          set -euo pipefail
          ver="${{ steps.ver.outputs.ver }}"
          if cargo publish --locked; then
            echo "cargo publish ok"
          else
            # client may time out after server-side success; check
            sleep 30
            if cargo search "ai-switch" --limit 1 | grep -E "^ai-switch = \"${ver}\"" >/dev/null; then
              echo "cargo publish detected on crates.io despite client error; proceeding"
            else
              echo "::error::cargo publish failed and crate not found on crates.io"
              gh release delete "${GITHUB_REF_NAME}" --yes --cleanup-tag=false || true
              exit 1
            fi
          fi

      - name: stage C — promote draft to published release
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: gh release edit "${GITHUB_REF_NAME}" --draft=false

  finalize:
    name: finalize (cargo install smoke)
    needs: publish
    runs-on: ubuntu-latest
    container:
      image: rust:1
    steps:
      - name: derive version
        id: ver
        run: echo "ver=${GITHUB_REF_NAME#v}" >> "$GITHUB_OUTPUT"

      - name: install build deps (defensive)
        run: |
          apt-get update
          apt-get install -y --no-install-recommends \
            ca-certificates pkg-config build-essential

      - name: wait for crates.io index
        run: |
          set -euo pipefail
          ver="${{ steps.ver.outputs.ver }}"
          for i in $(seq 1 30); do
            if cargo search "ai-switch" --limit 1 | grep -E "^ai-switch = \"${ver}\"" >/dev/null; then
              echo "found ai-switch v${ver} on crates.io"
              exit 0
            fi
            echo "waiting for crates.io ($i/30)..."
            sleep 10
          done
          echo "::error::ai-switch v${ver} did not appear on crates.io within 5 minutes"
          exit 1

      - name: cargo install smoke
        run: |
          set -euo pipefail
          ver="${{ steps.ver.outputs.ver }}"
          cargo install ai-switch --version "${ver}" --locked
          out=$(ais --version)
          echo "$out"
          echo "$out" | grep -E "${ver}" \
            || (echo "::error::ais --version did not contain ${ver}"; exit 1)
```

注意：每个 third-party action 的 SHA pin 是写本计划时检索到的稳定版本。**执行者在动手前应核对 SHA 仍指向预期 tag 的 commit**：

| action | 期望 tag | 核对命令 |
|---|---|---|
| `actions/checkout` | v4.2.2 | `gh api repos/actions/checkout/git/ref/tags/v4.2.2 --jq .object.sha` |
| `actions/upload-artifact` | v4.4.3 | `gh api repos/actions/upload-artifact/git/ref/tags/v4.4.3 --jq .object.sha` |
| `actions/download-artifact` | v4.1.8 | `gh api repos/actions/download-artifact/git/ref/tags/v4.1.8 --jq .object.sha` |
| `Swatinem/rust-cache` | v2 latest | `gh api repos/Swatinem/rust-cache/releases/latest --jq .tag_name,.target_commitish` |
| `dtolnay/rust-toolchain` | stable branch | `gh api repos/dtolnay/rust-toolchain/git/ref/heads/stable --jq .object.sha` |

如某 action 的 SHA 已不指向预期 tag → 替换 SHA + 同步注释中的 tag 名。spec D20 要求 pin commit SHA，不允许回退到 pin tag。

- [ ] **Step 2：actionlint 静态校验（必须）**

```bash
# 安装（任选其一）
go install github.com/rhysd/actionlint/cmd/actionlint@latest
# 或
brew install actionlint
# 或 docker
docker run --rm -v "$PWD":/repo -w /repo rhysd/actionlint:latest \
  -color .github/workflows/release.yml
```

预期：actionlint 退出码 0，无错误输出。如有 `shellcheck` 警告涉及 `set -euo pipefail` 或变量引号，按提示修。**不允许跳过此步**——release workflow 是 release 风险面最大单点，actionlint 是最便宜的防线。

- [ ] **Step 3：本地干跑 cargo install smoke 预演（finalize 行为前置验证）**

在 release.yml 提交前，本地用 docker 模拟 finalize job 的 cargo install smoke：

```bash
# 先打包当前工作树为本地 crate（不上 crates.io）
cargo package --locked
ls target/package/ai-switch-0.1.0.crate

# 在 docker 内用本地 crate 测试 install + run（模拟 finalize 步骤）
docker run --rm -v "$PWD/target/package":/pkg rust:1 bash -c '
  apt-get update -qq && apt-get install -qq -y --no-install-recommends \
    ca-certificates pkg-config build-essential >/dev/null
  cargo install --path /tmp/extracted --locked || true
  cd /tmp && tar -xzf /pkg/ai-switch-0.1.0.crate
  cd /tmp/ai-switch-0.1.0 && cargo install --path . --locked
  ais --version
'
```

预期：`ais --version` 输出含 `0.1.0`。如失败说明 finalize job 也会失败 → 修依赖 / 修 Cargo.toml `include`。

- [ ] **Step 4：commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci(release): tag 触发三段式自动发布

- preflight 3 gates: SemVer regex + tag commit on main ancestry +
  fmt/clippy/test + cargo publish --dry-run
- build matrix×4: linux musl x86_64 / macos x86_64 / macos arm64 /
  windows msvc x86_64；含平台二进制断言 + archive smoke
- publish 三段式: GH Release draft 上传 → cargo publish (含网络抖动
  幂等) → 把 draft 改 published
- environment: release-prod（首次发布需手动 approve）
- finalize: 5 分钟轮询 crates.io 索引 + cargo install smoke
- 所有 third-party action uses: pin commit SHA（D20）
- 全部 version 字面量从 \${GITHUB_REF_NAME#v} 派生（D17）"
```

---

## Task 6：TUI 文案对齐核对

**Files:**
- 视核对结果决定是否 modify

`src/tui/widgets.rs:175-176` 已确认 Help 弹窗写 `K               keys panel`，与代码绑定一致。本任务核对**其它出现快捷键提示的位置**是否也对齐。

- [ ] **Step 1：搜全部 footer / 提示字符串中的 `[k]` `k ` 是否仍指 keys panel**

```bash
grep -rnE "\\[k\\]|k\s+keys|press k" src/ 2>/dev/null
```

预期输出：仅有 `(no profiles — press [n] to create one)` 之类与 keys 无关的字符串；如发现 `[k] keys`、`press k` 等指向 keys panel 的过时文案 → 改为 `[K]` / `press K`。

- [ ] **Step 2：搜 README 之前的内置文档/注释里的快捷键说明**

```bash
grep -rnE "keys panel|key 面板|keys 面板" src/ 2>/dev/null | head -10
```

预期：`src/tui/views/profiles.rs` 顶部注释（约第 9 行）含 `↑↓/jk、p / K / d 切面板`，已正确。如发现其它文件里有过时 `k 切面板` 文案 → 改为 `K`。

- [ ] **Step 3：跑全仓测试确认无破坏**

```bash
cargo test --workspace --all-targets --locked
```

预期：全部 PASS（68/68）。

- [ ] **Step 4：commit（如有改动）**

如有改动：

```bash
git add -u
git commit -m "docs(tui): 文案对齐 — keys panel 入口统一为大写 K

与 Plan B 实际绑定（src/tui/views/profiles.rs:108）一致；
小写 k 是 vim 上移，已被 KeyCode::Up 复用。"
```

如无改动则跳过 commit。

---

## Task 7：walkthrough + receipt 草稿

**Files:**
- Create: `docs/superpowers/release-receipts/v0.1.0.md`

**前置**：T0-T6 全部完成；本地 main 已是 PR 待合状态；`cargo build --release` 与 `cargo test` 全绿。

- [ ] **Step 1：备份真实 ~/.claude/settings.json（防御性，walkthrough 不应触碰它）**

```bash
test -f ~/.claude/settings.json && cp ~/.claude/settings.json ~/.claude/settings.json.bak.$(date +%s) || echo "no global settings.json (skip backup)"
```

- [ ] **Step 2：建隔离测试根**

```bash
export AIS_HOME=/tmp/ais-walkthrough-2026-05-01
rm -rf "$AIS_HOME"
mkdir -p "$AIS_HOME"
```

整 walkthrough 期间该 shell 保留 `AIS_HOME` 导出。

- [ ] **Step 3：记录环境基线**

```bash
ais_ver=$(./target/release/ais --version)
claude_ver=$(claude --version)
echo "ais: $ais_ver"
echo "claude: $claude_ver"
```

输出抄到 receipt 草稿。

- [ ] **Step 4：item 1 — build**

```bash
cargo build --release --locked
```

预期：`Finished release [optimized] target(s) in <N>s`。CI 三平台绿在 PR 提交后由 GitHub Actions 自动验证（Step 8 会贴 ci run URL）。

- [ ] **Step 5：item 2 — TUI 进出 termios 干净**

```bash
stty -a > /tmp/stty-before.txt
./target/release/ais
# 在 TUI 内按 q 退出
stty -a > /tmp/stty-after.txt
diff /tmp/stty-before.txt /tmp/stty-after.txt && echo "PASS: termios clean" || echo "FAIL"
```

预期：`diff` 输出为空；echo "PASS: termios clean"。

- [ ] **Step 6：item 6 (preflight) — Doctor 6 项**

```bash
./target/release/ais
# 在 TUI 内按 d，观察 6 项是否全 PASS；按 q 退出
```

把 Doctor 输出（每项一行）抄到 receipt。如有 FAIL → 排查环境（最常见：`claude` 不在 PATH、`AIS_HOME` 权限）；修好再回到本步。

- [ ] **Step 7：item 3 — wizard 创建 deepseek profile + 裸跑 claude**

在 TUI 内：
1. 按 `n` 进 wizard
2. Step 1：选 `deepseek`
3. Step 2：按 "+ 添加新 key"，输入真账号 deepseek key（仅本地、不入截图）
4. Step 3：从 `/v1/models` 拉的列表中选一个 model（如 `deepseek-chat`）
5. Step 4：接受默认建议名（如 `deepseek_deepseek-chat`）
6. Step 5：按 Enter 写盘

退出 TUI 后：

```bash
profile_path="$AIS_HOME/claude/settings_deepseek_deepseek-chat.json"
test -f "$profile_path" && echo "profile written: $profile_path"
claude --settings "$profile_path" --version
```

预期：`claude --settings` 退出码 0；stdout 是 claude 版本号。把 exit code 与 stdout 抄到 receipt（key 字段如有出现以 `sk-a...fswv` 形式脱敏）。

- [ ] **Step 8：item 4 — `ais claude <name>` 启动等价**

```bash
./target/release/ais claude deepseek_deepseek-chat --version 2>/tmp/ais-stderr.txt
echo "exit=$?"
cat /tmp/ais-stderr.txt
```

判据（**严格区分 ais 与 claude 的输出归属**）：
- exit code 与 item 3 相同（一般是 0）
- `/tmp/ais-stderr.txt` **应为空** —— Unix 上 ais 在 `execvp` 之前**不**应向 stderr 写任何内容；execvp 之后该进程已被 claude 替换，stderr 上的所有内容都来自 claude 子进程，**不**算 ais 污染（设计 §8 契约："ais 不在 stdout/stderr 写任何东西"指 ais 自身的 pre-exec 阶段）
- stdout 不重定向；显示的内容都是 claude 写的，不归因于 ais

如 `/tmp/ais-stderr.txt` **非空且首行不是 claude 自己的 stderr 输出**（例如出现 `ais: error: ...` 或 `Error: ...`）→ ais 违反契约，立即停 walkthrough 排查 `src/claude.rs` 启动路径。

把 exit code + stderr 内容（如果是 claude 的 stderr 则原样保留 + 注明"claude 来源"）抄到 receipt。

- [ ] **Step 9：item 5 — key 轮换批量同步**

```bash
cp "$profile_path" /tmp/profile-before.json
```

```bash
./target/release/ais
# 在 TUI 内：
#   1. 主页按 K（大写）进 keys 面板
#   2. 选中刚加的 deepseek key
#   3. 按 e 编辑 → 改 value（任意改一个字符也可，例如末位 +1）→ 弹"已影响 1 个 profile" → 确认
#   4. 按 q 退出
```

```bash
diff <(jq -S . /tmp/profile-before.json) <(jq -S . "$profile_path") | tee /tmp/profile-diff.txt
```

预期：`diff` 输出**仅** `env.ANTHROPIC_API_KEY` 那一行变化；其它字段（`ANTHROPIC_BASE_URL`、`ANTHROPIC_MODEL`、可能的 `permissions` 占位）不动。把 diff 输出（key 已脱敏）抄到 receipt。

- [ ] **Step 10：item 6 (final) — Doctor 再跑一遍**

```bash
./target/release/ais
# 主页按 d，观察 6 项均 PASS
```

抄输出到 receipt。

- [ ] **Step 11：cleanup**

```bash
rm -rf /tmp/ais-walkthrough-2026-05-01
unset AIS_HOME
```

确认在 receipt 里写 "cleanup: rm -rf done at <timestamp>"。

- [ ] **Step 12：写 `docs/superpowers/release-receipts/v0.1.0.md` 草稿**

```bash
mkdir -p docs/superpowers/release-receipts
```

文件内容：

```markdown
# ai-switch v0.1.0 — Release Receipt

> 状态：草稿（PR `feature/plan-c-release` 内）；merge 后由 T9 单独 commit 完成（去除"草稿"标记）。

## 元信息

- Tag: `v0.1.0` (pending until T11)
- Host: `<uname -a>`
- ais build: `<ais --version>`
- claude version: `<claude --version>`
- AIS_HOME: `/tmp/ais-walkthrough-2026-05-01/`
- Date: 2026-05-01

## §14.4 Walkthrough（Linux 亲测）

### 1. cargo build --release
PASS — `Finished release [...]`
CI run（PR 提交后回填）: <URL>

### 2. TUI 进出 termios 干净
PASS — `stty -a` before/after diff 为空

### 3. wizard + 裸 claude
- Profile: `deepseek_deepseek-chat`
- Path: `/tmp/ais-walkthrough-2026-05-01/claude/settings_deepseek_deepseek-chat.json`
- `claude --settings <path> --version` exit=0, stdout: `<claude version>`

### 4. `ais claude <name>` 启动等价
- `ais claude deepseek_deepseek-chat --version` exit=0
- stderr: empty（ais 不污染 stdout/stderr，符合设计 §8 契约）

### 5. Key 轮换批量同步
- Edited key id: `sk-a...fswv`（脱敏）
- Affected profiles: 1
- jq diff: 仅 `env.ANTHROPIC_API_KEY` 行变化
```diff
<贴 jq diff 输出，key 字段以 sk-a...fswv 脱敏>
```

### 6. Doctor 6 项

| # | 检查 | preflight | final |
|---|---|---|---|
| 1 | `claude` 在 PATH | PASS | PASS |
| 2 | Claude Code 版本 | PASS（<version>） | PASS |
| 3 | `~/.ai-switch/` 可读写 | PASS | PASS |
| 4 | `credentials.toml` 权限 0600 | PASS | PASS |
| 5 | `.ais-index.toml` 一致性 | PASS（0 indexed, 0 on disk） | PASS（1 indexed, 1 on disk） |
| 6 | Provider 表加载 | PASS | PASS |

### Cleanup

`rm -rf /tmp/ais-walkthrough-2026-05-01/` ✓ at <timestamp>

---

## Provider Catalog Sweep（2026-05-01）

| provider | doc URL | verdict | reason |
|---|---|---|---|
| anthropic-official | <URL> | keep | 字段保持原值 |
| deepseek           | <URL> | keep | 字段保持原值 |
| openrouter         | <URL> | <verdict> | <reason> |
| kimi               | <URL> | <verdict> | <reason> |
| glm                | <URL> | <verdict> | <reason> |

详情见各 provider 段落。

### anthropic-official
- doc: <URL>
- 4 字段最终值：`anthropic_base_url=https://api.anthropic.com`、`openai_base_url=None`、`auth=XApiKey`、`models_endpoint_path=/v1/models`
- 处置：保留

### deepseek
- doc: <URL>
- 4 字段：<...>
- 处置：保留 + 真账号亲测 wizard 全流程通过

### openrouter
- doc: <URL>
- <T2 sweep 结论>

### kimi
- doc: <URL>
- <T2 sweep 结论>

### glm
- doc: <URL>
- <T2 sweep 结论>

---

## crate name verification

```
$ cargo search ai-switch --limit 5
<output>
crates.io page check: 404 (name available)
date: 2026-05-01
```
```

把 `<...>` 处替换为 walkthrough 实际记录。

- [ ] **Step 13：commit receipt 草稿**

```bash
git add docs/superpowers/release-receipts/v0.1.0.md
git commit -m "docs(receipt): v0.1.0 walkthrough + sweep 回执草稿

- §14.4 1-6 Linux 亲测全 PASS
- 5 家 provider sweep 结论归档
- crate name verification 记录
- 状态：草稿；T9 在 merge 后单独 commit 去掉草稿标记"
```

---

## Task 8：PR 提交 → review → merge

**Files:** 无（GitHub UI 操作）。

- [ ] **Step 1：push feature 分支**

```bash
git push -u origin feature/plan-c-release
```

- [ ] **Step 2：创建 PR**

```bash
gh pr create \
  --base main \
  --head feature/plan-c-release \
  --title "ai-switch V1 Plan C — Release prep (M6)" \
  --body-file - <<'EOF'
## 概述

落地 [Plan C 设计](docs/superpowers/specs/2026-05-01-ai-switch-v1-plan-c-release-design.md) 与 [Plan C 实施计划](docs/superpowers/plans/2026-05-01-ai-switch-v1-plan-c-release.md) 的全部 P0 产物。merge 后人工打 tag `v0.1.0` 触发 release.yml 三段式自动发布。

## 改动清单

- 新增 LICENSE-MIT / LICENSE-APACHE；删除旧 LICENSE
- Cargo.toml：crates.io 元数据补齐 + include 白名单 + 不写 rust-version
- src/catalog.rs：5 家 provider sweep 后的字面量调整 + 单元测试同步
- README.md：Public Preview 重写
- CHANGELOG.md：Keep a Changelog 1.1 + [0.1.0] 条目
- .github/workflows/release.yml：tag 触发三段式发布
- docs/superpowers/release-receipts/v0.1.0.md：walkthrough + sweep 回执草稿

## 验收

§14.4 1-6 Linux 亲测全 PASS（详见 receipt 文件）。

## merge 后步骤

1. T9：去掉 receipt 文件的"草稿"标记，单独 commit 到 main
2. T10：在 GitHub 仓库设置中确认 protected tag 规则（`v*`）+ `release-prod` environment 已配 reviewer
3. T11：在 main 上打 tag `v0.1.0` 并 push，等 release.yml 完成

EOF
```

- [ ] **Step 3：等 ci.yml 三平台绿**

```bash
gh pr checks --watch
```

预期：`ci / test (ubuntu-latest)`、`ci / test (macos-latest)`、`ci / test (windows-latest)` 全 PASS。

回填 PR 描述中的 ci run URL：`gh run list --branch feature/plan-c-release --limit 5` 取最新 run 的 URL，`gh pr edit --body-file <updated.md>` 更新 PR 描述。

- [ ] **Step 4：自我 code-review**

```bash
gh pr diff
```

确认：
- 无任何未脱敏 API key（搜 `sk-` 全 diff 行确认全部为 `sk-a...fswv` 形式）
- 无 `<TODO>` / `<...>` 残留
- license / metadata 字段拼写正确

- [ ] **Step 5：merge PR**

squash merge 到 main：

```bash
gh pr merge --squash --delete-branch
```

merge 成功后 main 已含 PR 全部内容（除 receipt 还在草稿状态）。

- [ ] **Step 6：拉新 main**

```bash
git checkout main
git pull --ff-only origin main
git log -1 --oneline
```

预期：最新 commit 是 squash merge 的合并 commit。

---

## Task 9：GitHub 仓库侧加固（在 tag push 之前）

**Files:** 无（GitHub UI / Settings 操作；可用 `gh api` 配 但 UI 更直观）。

- [ ] **Step 1：启用 protected tag 规则**（推荐：Repository Rulesets，旧 tag protection API 已 deprecated）

GitHub UI 路径：仓库 → Settings → **Rules** → Rulesets → "New ruleset" → "New tag ruleset"：
- Ruleset Name: `release-tags`
- Enforcement status: Active
- Target tags：Include by pattern → `v*`
- Rules：
  - Restrict creations: 仅 Repository admins / maintainers 可创建匹配 tag
  - Restrict updates: 同上
  - Restrict deletions: 同上
- 保存

> **不要**用旧的 `gh api -X POST /repos/.../tags/protection` 接口——GitHub 已将该 API 标记为 deprecated 并迁移到 Repository Rulesets。

如需 API 形式（自动化）参考：

```bash
gh api -X POST "repos/YoungHong1992/ai-switch/rulesets" \
  -F name='release-tags' \
  -F target='tag' \
  -F enforcement='active' \
  -F 'conditions[ref_name][include][]=refs/tags/v*' \
  -F 'rules[][type]=creation' \
  -F 'rules[][type]=update' \
  -F 'rules[][type]=deletion'
```

实际字段格式以 GitHub Rulesets API 当前文档为准（执行时核对 `gh api repos/{owner}/{repo}/rulesets` schema）。

- [ ] **Step 2：创建 `release-prod` GitHub Environment**

GitHub UI：仓库 → Settings → Environments → "New environment" → 名为 `release-prod`：
- 添加 "Required reviewers"（自己作为 maintainer，首次发布需手动 approve）
- 不设 deployment branch 限制（tag 不在 branch 上）
- 不设 wait timer

- [ ] **Step 3：添加 `CRATES_TOKEN` secret 到 `release-prod` environment**

在 crates.io（`https://crates.io/me`）创建 token，**优先尝试 crate-scoped 形式**：

- Name: `ai-switch-release-prod`
- Endpoint scopes: `publish-new` + `publish-update`（首发需 `publish-new`，后续 patch 需 `publish-update`）
- Crate scopes: 输入 `ai-switch`

> crates.io token scope 的 crate name pattern 是按 publish 时匹配的，理论上允许指向**尚未发布**的 crate name。如 crates.io UI 拒绝为不存在的 crate 创建 crate-scoped token：
>
> 1. 临时创建一个**最小权限**的 token（仅 `publish-new` + `publish-update`，crate scope 留空意味着账号级，但**不**带 `yank` / `change-owners` 等高危 scope）
> 2. 完成首次 publish 后，**立刻** rotate：到 crates.io 撤销该临时 token；新建带 `ai-switch` crate scope 的限定 token；替换 GitHub `release-prod` environment 的 secret

GitHub UI：Settings → Environments → release-prod → "Environment secrets" → 添加 `CRATES_TOKEN` = 上述 token 值。

> **关于紧急 yank 流程**：本 token 不含 `yank` scope。如发布后需 yank（见 T11），需另外用 `cargo login` 在本地登录账号（或创建临时含 yank scope 的 token），跑 `cargo yank ai-switch --version <ver>`。**不**把 yank scope 加到 release-prod 自动化 token，遵循最小权限。

- [ ] **Step 4：完成 receipt 草稿（去掉 pending 标记）**

```bash
git checkout main
git pull --ff-only origin main
```

编辑 `docs/superpowers/release-receipts/v0.1.0.md`：
- 把元信息段的 `Tag: v0.1.0 (pending until T11)` 改为 `Tag: v0.1.0`
- 把"状态：草稿"行改为 `状态：定稿`
- 在 walkthrough item 1 段补回填的 ci run URL

- [ ] **Step 5：commit + push receipt 终稿**

> **如 main 启用 branch protection** 拒绝直接 push：把这个 commit 放到 `release/v0.1.0-receipt` 分支并开 PR fast-track 合并；不要为绕过保护用 `--force` 或临时关保护。本项目当前 main 无 branch protection（前两个 spec/plan commit 直接 push 通过），故下文用直接 push 形式。

```bash
git add docs/superpowers/release-receipts/v0.1.0.md
git commit -m "docs(receipt): v0.1.0 walkthrough 回执定稿

把 PR feature/plan-c-release 内的草稿 receipt 去除 pending 标记，
补 ci run URL，归档到 docs/superpowers/release-receipts/。"
git push origin main
```

---

## Task 10：打 tag `v0.1.0` → release.yml 自动发布

**Files:** 无（git tag 操作）。

- [ ] **Step 1：核对 main 当前状态**

```bash
git checkout main
git pull --ff-only origin main
git log -1 --oneline
git diff origin/main HEAD  # 应为空
cargo metadata --no-deps --format-version 1 \
  | python3 -c 'import sys,json;d=json.load(sys.stdin);print(d["packages"][0]["version"])'
# 预期: 0.1.0
```

- [ ] **Step 2：确认 CHANGELOG 日期占位已替换**

```bash
grep "2026-05-XX" CHANGELOG.md && echo "WARNING: 占位未替换" || echo "ok"
```

如有占位 → 改为今日日期（如 `2026-05-01`）→ commit + push（同 T9 Step 5 的 branch protection 注意事项）：

```bash
sed -i 's/2026-05-XX/2026-05-01/' CHANGELOG.md
git add CHANGELOG.md
git commit -m "docs(changelog): [0.1.0] 日期定稿"
git push origin main
```

- [ ] **Step 3：打 annotated tag**

```bash
git tag -a v0.1.0 -m "ai-switch v0.1.0 — Public Preview

详见 CHANGELOG.md 与 GitHub Release notes。"
```

- [ ] **Step 4：push tag**

```bash
git push origin v0.1.0
```

push 后 release.yml 立即触发。

- [ ] **Step 5：监控 release.yml 进度**

```bash
# 拿到本次 push 触发的 release run id
run_id=$(gh run list --workflow=release.yml --branch=v0.1.0 --limit 1 --json databaseId --jq '.[0].databaseId')
echo "run_id=$run_id"
gh run watch "$run_id"
```

观察 4 个 job 顺序：
1. **preflight**：~2 min；任一 gate 失败 → workflow 红，**立即检查 main 状态 + Cargo.toml version + tag 名**
2. **build (matrix×4)**：~5-10 min；任一 archive smoke 失败 → 排查对应平台
3. **publish**：进入 `release-prod` environment 时**会暂停等审批**（手动按 Approve）；批准后跑三段
4. **finalize**：~1-5 min（轮询 crates.io 索引）

- [ ] **Step 6：在 publish job 暂停时审批**

GitHub UI：Actions → release run → 顶部"Review pending deployments" → 选 `release-prod` → Approve

- [ ] **Step 7：等 finalize 全绿**

```bash
gh run watch
```

预期：所有 4 job ✓。

---

## Task 11：发布后核验 + 异常处理

**Files:** 无。

- [ ] **Step 1：浏览 GitHub Release 页**

`https://github.com/YoungHong1992/ai-switch/releases/tag/v0.1.0`

确认：
- Release 标题 "ai-switch v0.1.0 — Public Preview"
- 标记为 **Pre-release**
- assets 列出 4 个 archive + 4 个 `.sha256` + 1 个 `SHA256SUMS`
- Release notes 含安装速查 + Known limitations 链

- [ ] **Step 2：浏览 crates.io 页**

`https://crates.io/crates/ai-switch`

确认：
- 版本 0.1.0 可见
- 元数据：description / repository / homepage / documentation 与 Cargo.toml 一致
- categories: command-line-utilities, development-tools

- [ ] **Step 3：本地从 crates.io 实测**

```bash
# 干净环境（用 docker 或临时目录均可）
docker run --rm rust:1-slim bash -c '
  cargo install ai-switch --version 0.1.0 --locked &&
  ais --version
'
```

预期：`ais --version` 输出含 `0.1.0`。

- [ ] **Step 4：本地从 GitHub Release 二进制实测（Linux 当前主机）**

```bash
cd /tmp
curl -fsSLO https://github.com/YoungHong1992/ai-switch/releases/download/v0.1.0/ais-v0.1.0-x86_64-unknown-linux-musl.tar.gz
curl -fsSLO https://github.com/YoungHong1992/ai-switch/releases/download/v0.1.0/SHA256SUMS
sha256sum -c SHA256SUMS --ignore-missing
tar -xzf ais-v0.1.0-x86_64-unknown-linux-musl.tar.gz
./ais-v0.1.0-x86_64-unknown-linux-musl/ais --version
```

预期：sha256 校验通过 + `ais --version` 输出含 `0.1.0`。

- [ ] **Step 5：异常处理（仅在异常发生时执行）**

| 故障 | 处置 |
|---|---|
| preflight Gate 1 失败：tag 非 SemVer 或 version 不匹配或 tag commit ≠ origin/main HEAD | 删 tag（`git tag -d v0.1.0; git push origin :refs/tags/v0.1.0`）→ 修 Cargo.toml 或 rebase tag 到 main HEAD → 重打 tag |
| preflight Gate 2 失败：fmt/clippy/test | 修代码 → push main → 删 tag → 重打 tag |
| build matrix 某平台失败 | 看 log；常见：musl-tools 装载失败、`lipo -info` 输出 unexpected → 改 release.yml 重打 tag |
| publish Stage A（draft）失败 | release.yml Stage A 已实现幂等（自动删旧 draft 重创建）；如脚本本身失败 → 手动 `gh release delete v0.1.0 --cleanup-tag=false` 清理 → 修 → 重打 tag |
| publish Stage B（cargo publish）失败但 crate 已上 crates.io | release.yml 内已实现幂等（`cargo search` 检查后 proceeding）；如脚本未识别到 → 手动 `gh release edit v0.1.0 --draft=false` 完成 Stage C |
| publish Stage B 失败且 crate 未上 crates.io | release.yml 自动删 GH draft；修 + 重打 tag |
| publish Stage C（promote）失败 | crate 已不可逆；手动 `gh release edit v0.1.0 --draft=false` 即可 |
| finalize cargo install smoke 失败但 publish 成功 | 人工跑 Step 3 docker 实测；通过则 Plan C 完成；失败说明已发布版本有 cargo install 阻塞 bug → 立刻 yank（见下） + 准备 `0.1.1` 补丁 |

**Yank 应急流程**（仅在已发布版本有阻塞 bug 时执行）：

`CRATES_TOKEN` release token 不含 `yank` scope（T9 Step 3 决议），所以需另外认证：

```bash
# 选项 A：本地登录账号 token（一次性；事后退出）
cargo login   # 粘账号级 token；按提示交互
cargo yank ai-switch --version 0.1.0
cargo logout  # 立刻退出

# 选项 B：临时创建仅含 yank scope 的 token
# 到 https://crates.io/me 创建 token，scope = yank-versions，crate = ai-switch
CARGO_REGISTRY_TOKEN=<临时 token> cargo yank ai-switch --version 0.1.0
# 用完立刻到 crates.io 撤销该 token
```

注意：`cargo yank` 接受 `--version` 与 `--vers` 两种形式，本计划统一用 `--version`（更可读）。**Yank 不是 unpublish**——已下载或已 lock 引用的用户仍可拉到该版本，仅阻止新解析。同时立刻发 `0.1.1` 补丁。

**Token 失窃应急流程**（CRATES_TOKEN 或 GitHub PAT 泄露）：

1. **立刻撤销**：`https://crates.io/me` 撤销该 token；GitHub Settings → Environments → release-prod → 删 secret
2. **核查发布物**：`cargo search ai-switch` 看是否有非预期版本；GitHub Releases 页看是否有非预期 tag/release；`git log v0.1.0..HEAD` 看是否有非预期 commit
3. **如有未授权 publish**：立刻 yank 该版本（按上文 Yank 流程）；如 GH Release 被改：`gh release delete <tag>`（不删 tag 让 git 历史保留证据）
4. **轮换**：crates.io 创建新 token（按 T9 Step 3 最小权限规则）；GitHub 添加新 secret
5. **公开通报**（如有真实危害）：在 GitHub Issues 起一个 Security Advisory；评估是否需要在 README / CHANGELOG 顶部加警示横幅
6. **事后复盘**：commit 一份 incident note 到 `docs/superpowers/release-receipts/incidents/<date>-<topic>.md`（目录在首次事件时创建）

- [ ] **Step 6：rotate `CRATES_TOKEN`（仅在 T9 Step 3 用了临时账号级 token 时）**

如 T9 Step 3 因 crates.io UI 限制临时使用账号级 token，发布成功后到 `https://crates.io/me`：
- 创建新 token，crate scope = `ai-switch`，endpoint scope = `publish-update`（不再需要 `publish-new`，crate 已存在）
- 替换 GitHub `release-prod` environment 的 `CRATES_TOKEN` secret
- 撤销旧临时 token

如 T9 Step 3 已用 crate-scoped token 完成首发，本步可跳过——但仍建议 6 个月内 rotate 一次（最佳实践）。

- [ ] **Step 7：宣布 Plan C 完成**

verify §12 验收门槛 6 条全部满足：

```bash
# 1. main 上有 v0.1.0 tag
git rev-parse v0.1.0 || echo "FAIL: no v0.1.0 tag"

# 2. GH Release 已 publish 含 4 archive + SHA256SUMS
gh release view v0.1.0 --json assets --jq '.assets[].name'
# 预期 9 行：4 archive + 4 .sha256 + 1 SHA256SUMS

# 3. crates.io 0.1.0 可见 + cargo install smoke
# (已在 Step 3 验证)

# 4. walkthrough receipt 已归档
test -f docs/superpowers/release-receipts/v0.1.0.md && echo "ok"

# 5. CHANGELOG [0.1.0] 条目齐
grep -q '^## \[0.1.0\]' CHANGELOG.md && echo "ok"

# 6. README 链接均为 GitHub absolute URL
grep -nE "\]\(docs/" README.md && echo "FAIL: 仍有相对链接" || echo "ok"
```

全部 ok → Plan C 完成；任一失败 → 回查并修复（spec §12 不允许"差不多就发"）。

---

## 验收门槛索引（来自 spec §12）

| # | 条件 | 在哪 task 落地 |
|---|---|---|
| 1 | main 上有 tag `v0.1.0` 含全部 §2.1 产物 | T1-T8（PR）+ T10（tag） |
| 2 | GH Release v0.1.0 publish + 4 archive + SHA256SUMS + 含双 LICENSE/README | T5（workflow）+ T10（tag）+ T11 Step 1（人工核验） |
| 3 | crates.io 可见 + cargo install smoke ok | T5 finalize + T11 Step 3（docker 实测） |
| 4 | Linux walkthrough §14.4 1-6 全过 + receipt 归档 | T7（草稿）+ T9 Step 4-5（终稿） |
| 5 | CHANGELOG [0.1.0] 条目齐 + [Unreleased] 段保留 | T4 |
| 6 | README 自审过：安装两路径、quickstart、FAQ 14 题、docs 链接为 GitHub absolute URL | T3 + T11 Step 7 |

---
