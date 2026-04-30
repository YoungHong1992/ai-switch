use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("could not locate user home directory")]
    HomeDirNotFound,

    #[error("profile not found: {name}")]
    ProfileNotFound { name: String },

    #[error("`claude` not found in PATH; install Claude Code first")]
    ClaudeNotInPath,

    #[error("could not parse `claude --version` output: {0}")]
    ClaudeVersionParse(String),

    #[error("could not parse settings.json at {path}: {source}")]
    SettingsParse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("credentials.toml at {path} is corrupted: {source}")]
    CredentialsCorrupted {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("providers.toml at {path} is corrupted: {source}")]
    ProvidersCorrupted {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error(".ais-index.toml at {path} is corrupted: {source}")]
    IndexCorrupted {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("key id `{id}` already exists for provider `{provider}`")]
    KeyIdConflict { provider: String, id: String },

    #[error("invalid profile name `{name}`: {reason}")]
    InvalidProfileName { name: String, reason: String },

    #[error("invalid key id `{id}`: {reason}")]
    InvalidKeyId { id: String, reason: String },

    #[error("provider `{id}` is not registered")]
    ProviderNotFound { id: String },

    #[error("provider `{id}` has no anthropic_base_url; cannot use as Claude profile")]
    ProviderMissingAnthropicUrl { id: String },

    #[error("key value too short to derive id automatically (need >=12 chars, got {len})")]
    KeyValueTooShortForAutoId { len: usize },

    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[cfg(unix)]
    #[error("file {path} permission too open (mode {mode:o}); should be 0600")]
    PermissionTooOpen { path: PathBuf, mode: u32 },
}
