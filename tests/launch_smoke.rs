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
