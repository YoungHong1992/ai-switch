use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum AuthScheme {
    Bearer,
    XApiKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provider {
    pub id: String,
    pub display_name: String,
    pub anthropic_base_url: Option<String>,
    pub openai_base_url: Option<String>,
    pub auth: AuthScheme,
    pub models_endpoint_path: String,
}

/// Built-in provider catalog (compile-time constant via fn).
///
/// 实际 URL 与端点路径在 v0.1.0 发布前会再 sweep 一次官方文档校准；
/// 当前值用于跑通整套数据流。
pub fn builtins() -> Vec<Provider> {
    vec![
        Provider {
            id: "anthropic-official".into(),
            display_name: "Anthropic Official".into(),
            anthropic_base_url: Some("https://api.anthropic.com".into()),
            openai_base_url: None,
            auth: AuthScheme::XApiKey,
            models_endpoint_path: "/v1/models".into(),
        },
        Provider {
            id: "deepseek".into(),
            display_name: "DeepSeek".into(),
            anthropic_base_url: Some("https://api.deepseek.com/anthropic".into()),
            openai_base_url: Some("https://api.deepseek.com/v1".into()),
            auth: AuthScheme::Bearer,
            models_endpoint_path: "/v1/models".into(),
        },
        Provider {
            id: "openrouter".into(),
            display_name: "OpenRouter".into(),
            anthropic_base_url: Some("https://openrouter.ai/api/v1".into()),
            openai_base_url: Some("https://openrouter.ai/api/v1".into()),
            auth: AuthScheme::Bearer,
            models_endpoint_path: "/v1/models".into(),
        },
        Provider {
            id: "kimi".into(),
            display_name: "Kimi (Moonshot)".into(),
            anthropic_base_url: Some("https://api.moonshot.cn/anthropic".into()),
            openai_base_url: Some("https://api.moonshot.cn/v1".into()),
            auth: AuthScheme::Bearer,
            models_endpoint_path: "/v1/models".into(),
        },
        Provider {
            id: "glm".into(),
            display_name: "GLM (智谱)".into(),
            anthropic_base_url: Some("https://open.bigmodel.cn/api/anthropic".into()),
            openai_base_url: Some("https://open.bigmodel.cn/api/paas/v4".into()),
            auth: AuthScheme::Bearer,
            models_endpoint_path: "/v1/models".into(),
        },
    ]
}

pub fn find(id: &str) -> Option<Provider> {
    builtins().into_iter().find(|p| p.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn builtins_includes_anthropic_official() {
        let p = find("anthropic-official").unwrap();
        assert_eq!(p.auth, AuthScheme::XApiKey);
        assert_eq!(p.openai_base_url, None);
        assert!(p.anthropic_base_url.is_some());
    }

    #[test]
    fn builtins_deepseek_has_both_endpoints() {
        let p = find("deepseek").unwrap();
        assert_eq!(p.auth, AuthScheme::Bearer);
        assert!(p.anthropic_base_url.is_some());
        assert!(p.openai_base_url.is_some());
    }

    #[test]
    fn unknown_provider_returns_none() {
        assert!(find("nonexistent").is_none());
    }

    #[test]
    fn all_builtins_have_unique_ids() {
        let bs = builtins();
        let ids: BTreeSet<_> = bs.iter().map(|p| p.id.clone()).collect();
        assert_eq!(ids.len(), bs.len());
    }
}
