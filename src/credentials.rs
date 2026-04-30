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
