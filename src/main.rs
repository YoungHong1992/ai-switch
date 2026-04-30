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
