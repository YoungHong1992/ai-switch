//! 同步 HTTP 客户端：从 OpenAI 兼容 /v1/models 端点拉模型 id 列表。
//!
//! 设计取舍（spec §16 已声明可接受）：
//! - **同步阻塞**：用 ureq，5 秒 timeout；调用方 (wizard) 在 fetch 期间 UI 短暂 freeze。
//! - **不重试**：网络异常 / 解析失败 → 上层 wizard 切换到自由输入子状态。
//! - **不持 Authorization**：/v1/models 在多数 provider 下要鉴权，但 V1 假定 wizard
//!   step 3 时用户已选好 key（在 step 2），把 key value 透传 fetch_models 的 `bearer` 入参。

use std::time::Duration;

use serde::Deserialize;

use crate::error::Error;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// OpenAI-style /v1/models 响应：
/// ```json
/// {"data": [{"id": "deepseek-chat"}, {"id": "deepseek-coder"}]}
/// ```
#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<ModelEntry>,
}

#[derive(Debug, Deserialize)]
struct ModelEntry {
    id: String,
}

/// 拉取并返回模型 id 列表（按返回顺序保留）。
///
/// `base_url` + `endpoint_path` 拼成 URL；空字符串 base 视为非法，返回 HttpFetch。
/// `bearer` 为空时不带 Authorization 头（少数 provider 的 /v1/models 可匿名拉）。
pub fn fetch_models(
    base_url: &str,
    endpoint_path: &str,
    bearer: Option<&str>,
) -> Result<Vec<String>, Error> {
    let url = build_url(base_url, endpoint_path);
    let agent = ureq::AgentBuilder::new().timeout(DEFAULT_TIMEOUT).build();
    let mut req = agent.get(&url);
    if let Some(token) = bearer.filter(|t| !t.is_empty()) {
        req = req.set("Authorization", &format!("Bearer {token}"));
    }
    let resp = req.call().map_err(|e| Error::HttpFetch {
        url: url.clone(),
        message: e.to_string(),
    })?;
    let text = resp.into_string().map_err(|e| Error::HttpFetch {
        url: url.clone(),
        message: e.to_string(),
    })?;
    parse_models_response(&text).map_err(|msg| Error::HttpJson { url, message: msg })
}

/// 拼 URL（trim 末尾 /，确保 endpoint 以 / 开头）。
fn build_url(base: &str, endpoint: &str) -> String {
    let base = base.trim_end_matches('/');
    if endpoint.starts_with('/') {
        format!("{base}{endpoint}")
    } else {
        format!("{base}/{endpoint}")
    }
}

/// 把 JSON 响应解析成模型 id 列表。
///
/// 把这一步独立出来纯函数化，便于离线单元测试。
pub fn parse_models_response(json: &str) -> Result<Vec<String>, String> {
    let parsed: ModelsResponse =
        serde_json::from_str(json).map_err(|e| format!("parse error: {e}"))?;
    Ok(parsed.data.into_iter().map(|m| m.id).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_url_strips_trailing_slash() {
        assert_eq!(
            build_url("https://api.example.com/v1/", "/models"),
            "https://api.example.com/v1/models"
        );
        assert_eq!(
            build_url("https://api.example.com/v1", "models"),
            "https://api.example.com/v1/models"
        );
    }

    #[test]
    fn build_url_handles_empty_endpoint() {
        // 空 endpoint 退化为 base + "/" — 由 ureq 报"unsupported method/url"，
        // 但本测试只关心 build_url 自身行为：尾斜杠规范化。
        assert_eq!(
            build_url("https://api.example.com/v1/", ""),
            "https://api.example.com/v1/"
        );
    }

    #[test]
    fn parse_extracts_ids_in_order() {
        let json = r#"{"data":[{"id":"a"},{"id":"b"},{"id":"c"}]}"#;
        assert_eq!(parse_models_response(json).unwrap(), vec!["a", "b", "c"]);
    }

    #[test]
    fn parse_empty_data_returns_empty_vec() {
        let json = r#"{"data":[]}"#;
        assert!(parse_models_response(json).unwrap().is_empty());
    }

    #[test]
    fn parse_invalid_json_returns_err() {
        assert!(parse_models_response("not json {{{").is_err());
    }

    #[test]
    fn parse_missing_data_field_returns_err() {
        assert!(parse_models_response(r#"{"foo":1}"#).is_err());
    }

    #[test]
    fn parse_ignores_extra_fields() {
        let json = r#"{"data":[{"id":"x","object":"model","owned_by":"demo"}],"object":"list"}"#;
        assert_eq!(parse_models_response(json).unwrap(), vec!["x"]);
    }
}
