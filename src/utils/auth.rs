//! Shared authentication utilities for extracting data from `JWT` tokens.

use anyhow::Result;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::Value;

/// Usage limits and subscription information extracted from the `JWT` token.
#[derive(Debug, Clone)]
pub struct UsageInfo {
    /// Authenticated user email
    pub email: String,
    /// Subscription plan type (personal / team / enterprise)
    pub plan_type: String,
    /// ISO 8601 subscription start date
    pub subscription_start: Option<String>,
    /// ISO 8601 subscription end date
    pub subscription_end: Option<String>,
    /// `OpenAI` account identifier
    pub account_id: String,
    /// Organisation memberships with roles
    pub organizations: Vec<String>,
}

/// Extract the email claim from a `JWT` `id_token` string.
///
/// The token is decoded without signature verification — only the payload is
/// inspected.  Returns `None` when the token is malformed or carries no email.
#[must_use]
pub fn extract_email_from_token(id_token: &str) -> Option<String> {
    let parts: Vec<&str> = id_token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let payload = URL_SAFE_NO_PAD.decode(parts[1]).ok()?;
    let payload_str = std::str::from_utf8(&payload).ok()?;
    let payload_json: Value = serde_json::from_str(payload_str).ok()?;
    payload_json
        .get("email")
        .and_then(|e| e.as_str())
        .map(std::string::ToString::to_string)
}

/// Extract email from a parsed `auth.json` [`Value`].
#[must_use]
pub fn extract_email_from_auth_json(auth_json: &Value) -> Option<String> {
    let id_token = auth_json
        .get("tokens")
        .and_then(|t| t.get("id_token"))
        .and_then(|t| t.as_str())?;
    extract_email_from_token(id_token)
}

/// Detect the authentication mode represented by `auth.json`.
///
/// Returns one of:
/// - `chatgpt`: ChatGPT/Codex session token based auth
/// - `api_key`: API key based auth
/// - `chatgpt+api_key`: both are present
/// - `unknown`: neither signal is present
#[must_use]
pub fn detect_auth_mode(auth_json: &Value) -> String {
    let has_chatgpt_token = auth_json
        .get("tokens")
        .and_then(|t| t.get("id_token"))
        .and_then(|t| t.as_str())
        .is_some();

    let has_api_key = auth_json
        .get("api_key")
        .or_else(|| auth_json.get("key"))
        .or_else(|| auth_json.get("access_token"))
        .and_then(Value::as_str)
        .is_some();

    match (has_chatgpt_token, has_api_key) {
        (true, true) => "chatgpt+api_key".to_string(),
        (true, false) => "chatgpt".to_string(),
        (false, true) => "api_key".to_string(),
        (false, false) => "unknown".to_string(),
    }
}

/// Whether the auth mode includes ChatGPT/Codex session auth.
#[must_use]
pub fn auth_mode_has_chatgpt(auth_mode: &str) -> bool {
    matches!(auth_mode, "chatgpt" | "chatgpt+api_key")
}

/// Whether the auth mode includes OpenAI API key auth.
#[must_use]
pub fn auth_mode_has_api_key(auth_mode: &str) -> bool {
    matches!(auth_mode, "api_key" | "chatgpt+api_key")
}

/// Human-readable label for an auth mode string.
#[must_use]
pub fn auth_mode_label(auth_mode: &str) -> &'static str {
    match auth_mode {
        "chatgpt" => "ChatGPT/Codex",
        "api_key" => "OpenAI API key",
        "chatgpt+api_key" => "ChatGPT/Codex + OpenAI API key",
        _ => "Unknown",
    }
}

/// Read `auth.json` from `codex_dir` and extract the authenticated email.
///
/// Returns `None` if the file is absent, unreadable, or contains no email.
pub async fn read_email_from_codex_dir(codex_dir: &std::path::Path) -> Option<String> {
    let auth_path = codex_dir.join("auth.json");
    if !auth_path.exists() {
        return None;
    }
    let content = tokio::fs::read_to_string(&auth_path).await.ok()?;
    let json: Value = serde_json::from_str(&content).ok()?;
    extract_email_from_auth_json(&json)
}

/// Extract usage and subscription information from a parsed `auth.json` value.
///
/// # Errors
///
/// Returns an error if the `JWT` is missing, malformed, or lacks the expected
/// `OpenAI` custom claims.
pub fn extract_usage_info(auth_json: &Value) -> Result<UsageInfo> {
    let id_token = auth_json
        .get("tokens")
        .and_then(|t| t.get("id_token"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| anyhow::anyhow!("No id_token found"))?;

    let parts: Vec<&str> = id_token.split('.').collect();
    if parts.len() != 3 {
        anyhow::bail!("Invalid JWT format");
    }

    let payload = URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|e| anyhow::anyhow!("Failed to decode JWT: {e}"))?;

    let payload_str =
        std::str::from_utf8(&payload).map_err(|e| anyhow::anyhow!("Invalid UTF-8 in JWT: {e}"))?;

    let payload_json: Value = serde_json::from_str(payload_str)
        .map_err(|e| anyhow::anyhow!("Failed to parse JWT payload: {e}"))?;

    let email = payload_json
        .get("email")
        .and_then(|e| e.as_str())
        .unwrap_or("unknown")
        .to_string();

    let openai_auth = payload_json
        .get("https://api.openai.com/auth")
        .ok_or_else(|| anyhow::anyhow!("No `OpenAI` auth claims found"))?;

    let plan_type = openai_auth
        .get("chatgpt_plan_type")
        .and_then(|p| p.as_str())
        .unwrap_or("unknown")
        .to_string();

    let subscription_start = openai_auth
        .get("chatgpt_subscription_active_start")
        .and_then(|s| s.as_str())
        .map(String::from);

    let subscription_end = openai_auth
        .get("chatgpt_subscription_active_until")
        .and_then(|s| s.as_str())
        .map(String::from);

    let account_id = openai_auth
        .get("chatgpt_account_id")
        .and_then(|a| a.as_str())
        .unwrap_or("unknown")
        .to_string();

    let organizations: Vec<String> = openai_auth
        .get("organizations")
        .and_then(|o| o.as_array())
        .map(|orgs| {
            orgs.iter()
                .filter_map(|org| {
                    let title = org.get("title").and_then(|t| t.as_str());
                    let role = org.get("role").and_then(|r| r.as_str());
                    match (title, role) {
                        (Some(t), Some(r)) => Some(format!("{t} ({r})")),
                        (Some(t), None) => Some(t.to_string()),
                        _ => None,
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(UsageInfo {
        email,
        plan_type,
        subscription_start,
        subscription_end,
        account_id,
        organizations,
    })
}

#[cfg(test)]
mod tests {
    use super::{auth_mode_has_api_key, auth_mode_has_chatgpt, auth_mode_label, detect_auth_mode};

    #[test]
    fn test_detect_auth_mode_chatgpt() {
        let auth = serde_json::json!({
            "tokens": {
                "id_token": "header.payload.signature"
            }
        });
        assert_eq!(detect_auth_mode(&auth), "chatgpt");
    }

    #[test]
    fn test_detect_auth_mode_api_key() {
        let auth = serde_json::json!({
            "api_key": "sk-test"
        });
        assert_eq!(detect_auth_mode(&auth), "api_key");
    }

    #[test]
    fn test_detect_auth_mode_hybrid() {
        let auth = serde_json::json!({
            "tokens": {
                "id_token": "header.payload.signature"
            },
            "api_key": "sk-test"
        });
        assert_eq!(detect_auth_mode(&auth), "chatgpt+api_key");
    }

    #[test]
    fn test_detect_auth_mode_unknown() {
        let auth = serde_json::json!({
            "foo": "bar"
        });
        assert_eq!(detect_auth_mode(&auth), "unknown");
    }

    #[test]
    fn test_auth_mode_capabilities() {
        assert!(auth_mode_has_chatgpt("chatgpt"));
        assert!(auth_mode_has_chatgpt("chatgpt+api_key"));
        assert!(!auth_mode_has_chatgpt("api_key"));

        assert!(auth_mode_has_api_key("api_key"));
        assert!(auth_mode_has_api_key("chatgpt+api_key"));
        assert!(!auth_mode_has_api_key("chatgpt"));
    }

    #[test]
    fn test_auth_mode_labels() {
        assert_eq!(auth_mode_label("chatgpt"), "ChatGPT/Codex");
        assert_eq!(auth_mode_label("api_key"), "OpenAI API key");
        assert_eq!(
            auth_mode_label("chatgpt+api_key"),
            "ChatGPT/Codex + OpenAI API key"
        );
        assert_eq!(auth_mode_label("unknown"), "Unknown");
    }
}
