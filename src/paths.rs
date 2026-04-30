use std::path::PathBuf;

use crate::error::Error;

#[derive(Debug, Clone)]
pub struct Paths {
    pub root: PathBuf,
}

impl Paths {
    /// 解析配置 root：
    /// 1. 若环境变量 `AIS_HOME` 非空，直接作为 root（用于测试与高级用户自定义路径）
    /// 2. 否则使用 `directories::BaseDirs` 推导出的 `<home>/.ai-switch`
    pub fn from_home() -> Result<Self, Error> {
        if let Some(raw) = std::env::var_os("AIS_HOME")
            && !raw.is_empty()
        {
            return Ok(Self::with_root(PathBuf::from(raw)));
        }
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
        assert_eq!(
            p.credentials(),
            PathBuf::from("/tmp/test-ais/credentials.toml")
        );
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
