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
        assert_eq!(
            s.env["ANTHROPIC_BASE_URL"],
            "https://api.deepseek.com/anthropic"
        );
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
