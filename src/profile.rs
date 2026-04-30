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
