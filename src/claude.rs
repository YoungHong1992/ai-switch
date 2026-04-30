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
        assert_eq!(
            args,
            vec!["--settings".to_string(), "/x/s.json".to_string()]
        );
    }
}
