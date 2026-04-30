use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::catalog::{self, AuthScheme, Provider};
use crate::error::Error;

#[derive(Debug, Deserialize, Serialize)]
struct UserProvider {
    display_name: Option<String>,
    anthropic_base_url: Option<String>,
    openai_base_url: Option<String>,
    auth: AuthScheme,
    #[serde(default = "default_models_endpoint")]
    models_endpoint_path: String,
}

fn default_models_endpoint() -> String {
    "/v1/models".into()
}

fn parse_user_file(path: &Path) -> Result<BTreeMap<String, UserProvider>, Error> {
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let s = fs::read_to_string(path).map_err(|e| Error::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    toml::from_str(&s).map_err(|e| Error::ProvidersCorrupted {
        path: path.to_path_buf(),
        source: e,
    })
}

/// Load full provider table = built-ins overlaid by user-defined providers.
///
/// 同名 provider：用户定义覆盖内置（spec §5.3）。
pub fn load_all(user_providers_path: &Path) -> Result<Vec<Provider>, Error> {
    let user = parse_user_file(user_providers_path)?;

    let mut by_id: BTreeMap<String, Provider> = catalog::builtins()
        .into_iter()
        .map(|p| (p.id.clone(), p))
        .collect();

    for (id, up) in user {
        let provider = Provider {
            id: id.clone(),
            display_name: up.display_name.unwrap_or_else(|| id.clone()),
            anthropic_base_url: up.anthropic_base_url,
            openai_base_url: up.openai_base_url,
            auth: up.auth,
            models_endpoint_path: up.models_endpoint_path,
        };
        by_id.insert(id, provider);
    }

    Ok(by_id.into_values().collect())
}

/// Convenience: find a provider by id, considering user overrides.
pub fn find(user_providers_path: &Path, id: &str) -> Result<Option<Provider>, Error> {
    Ok(load_all(user_providers_path)?.into_iter().find(|p| p.id == id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_root() -> PathBuf {
        let id = format!(
            "ais-providers-test-{}-{}",
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
    fn no_user_file_returns_only_builtins() {
        let root = TempRoot(temp_root());
        let path = root.0.join("providers.toml");
        let all = load_all(&path).unwrap();
        assert_eq!(all.len(), catalog::builtins().len());
    }

    #[test]
    fn user_can_add_new_provider() {
        let root = TempRoot(temp_root());
        let path = root.0.join("providers.toml");
        fs::write(
            &path,
            r#"
[my-relay]
display_name = "My Private Relay"
anthropic_base_url = "https://my-relay.example.com/anthropic"
openai_base_url = "https://my-relay.example.com/v1"
auth = "Bearer"
"#,
        )
        .unwrap();
        let all = load_all(&path).unwrap();
        let mine = all.iter().find(|p| p.id == "my-relay").unwrap();
        assert_eq!(
            mine.anthropic_base_url.as_deref(),
            Some("https://my-relay.example.com/anthropic")
        );
        assert_eq!(mine.auth, AuthScheme::Bearer);
        assert_eq!(mine.models_endpoint_path, "/v1/models"); // default
    }

    #[test]
    fn user_override_wins_against_builtin() {
        let root = TempRoot(temp_root());
        let path = root.0.join("providers.toml");
        fs::write(
            &path,
            r#"
[deepseek]
anthropic_base_url = "https://my-deepseek-mirror.example.com/anthropic"
openai_base_url = "https://my-deepseek-mirror.example.com/v1"
auth = "Bearer"
"#,
        )
        .unwrap();
        let all = load_all(&path).unwrap();
        let ds = all.iter().find(|p| p.id == "deepseek").unwrap();
        assert_eq!(
            ds.anthropic_base_url.as_deref(),
            Some("https://my-deepseek-mirror.example.com/anthropic")
        );
    }

    #[test]
    fn corrupted_file_returns_error() {
        let root = TempRoot(temp_root());
        let path = root.0.join("providers.toml");
        fs::write(&path, "this is not toml === [[[").unwrap();
        let err = load_all(&path).unwrap_err();
        assert!(matches!(err, Error::ProvidersCorrupted { .. }));
    }
}
