use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Runtime};
use url::Url;

use crate::modules::{floating_card_window, logger};

pub const EXTERNAL_PROVIDER_IMPORT_EVENT: &str = "external:provider-import";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalProviderImportPayload {
    pub provider_id: String,
    pub page: String,
    pub token: String,
    pub auto_import: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_url: Option<String>,
}

static PENDING_EXTERNAL_PROVIDER_IMPORT: LazyLock<Mutex<Option<ExternalProviderImportPayload>>> =
    LazyLock::new(|| Mutex::new(None));

fn normalize_lookup_key(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

fn parse_boolean_like(value: Option<&String>) -> bool {
    let Some(value) = value else {
        return false;
    };
    let normalized = value.trim().to_ascii_lowercase();
    matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
}

fn resolve_provider_and_page(value: &str) -> Option<(&'static str, &'static str)> {
    let normalized = normalize_lookup_key(value);
    match normalized.as_str() {
        "antigravity" | "overview" | "accounts" => Some(("antigravity", "overview")),
        "codex" => Some(("codex", "codex")),
        "github_copilot" | "githubcopilot" | "ghcp" => Some(("github-copilot", "github-copilot")),
        "windsurf" => Some(("windsurf", "windsurf")),
        "kiro" => Some(("kiro", "kiro")),
        "cursor" => Some(("cursor", "cursor")),
        "gemini" => Some(("gemini", "gemini")),
        "codebuddy" => Some(("codebuddy", "codebuddy")),
        "codebuddy_cn" | "codebuddycn" => Some(("codebuddy_cn", "codebuddy-cn")),
        "qoder" => Some(("qoder", "qoder")),
        "trae" => Some(("trae", "trae")),
        "workbuddy" => Some(("workbuddy", "workbuddy")),
        "zed" => Some(("zed", "zed")),
        _ => None,
    }
}

fn is_supported_scheme(scheme: &str) -> bool {
    matches!(scheme, "cockpit-tools" | "cockpittools")
}

fn is_import_action(url: &Url) -> bool {
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    if matches!(
        host.as_str(),
        "import" | "provider-import" | "account-import"
    ) {
        return true;
    }

    let path = url.path().trim_matches('/').to_ascii_lowercase();
    matches!(
        path.as_str(),
        "import" | "provider-import" | "account-import"
    )
}

fn parse_query_map(url: &Url) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for (key, value) in url.query_pairs() {
        let normalized_key = normalize_lookup_key(key.as_ref());
        if normalized_key.is_empty() {
            continue;
        }
        map.entry(normalized_key)
            .or_insert_with(|| value.into_owned());
    }
    map
}

fn parse_external_import_url(raw_url: &str) -> Option<ExternalProviderImportPayload> {
    let parsed = Url::parse(raw_url).ok()?;
    if !is_supported_scheme(parsed.scheme()) {
        return None;
    }
    if !is_import_action(&parsed) {
        return None;
    }

    let query = parse_query_map(&parsed);
    let provider_raw = query
        .get("provider")
        .or_else(|| query.get("provider_id"))
        .or_else(|| query.get("providerid"))
        .or_else(|| query.get("platform"))
        .or_else(|| query.get("platform_id"))
        .or_else(|| query.get("platformid"))
        .or_else(|| query.get("target"))
        .or_else(|| query.get("page"))?;
    let (provider_id, page) = resolve_provider_and_page(provider_raw)?;

    let token = query
        .get("token")
        .or_else(|| query.get("import_token"))
        .or_else(|| query.get("importtoken"))
        .or_else(|| query.get("payload"))
        .or_else(|| query.get("import_payload"))
        .or_else(|| query.get("importpayload"))?
        .trim()
        .to_string();
    if token.is_empty() {
        return None;
    }

    let auto_import = parse_boolean_like(
        query
            .get("auto_import")
            .or_else(|| query.get("autoimport"))
            .or_else(|| query.get("auto_submit"))
            .or_else(|| query.get("autosubmit")),
    );

    Some(ExternalProviderImportPayload {
        provider_id: provider_id.to_string(),
        page: page.to_string(),
        token,
        auto_import,
        source: None,
        raw_url: None,
    })
}

fn set_pending(payload: ExternalProviderImportPayload) {
    if let Ok(mut guard) = PENDING_EXTERNAL_PROVIDER_IMPORT.lock() {
        *guard = Some(payload);
    }
}

pub fn take_pending_external_import() -> Option<ExternalProviderImportPayload> {
    let Ok(mut guard) = PENDING_EXTERNAL_PROVIDER_IMPORT.lock() else {
        return None;
    };
    guard.take()
}

fn emit_external_import_payload<R: Runtime>(
    app: &AppHandle<R>,
    payload: &ExternalProviderImportPayload,
) {
    if let Err(err) = app.emit(EXTERNAL_PROVIDER_IMPORT_EVENT, payload.clone()) {
        logger::log_warn(&format!("[ExternalImport] 发送外部导入事件失败: {}", err));
    }
}

pub fn handle_external_import_args<R: Runtime>(
    app: &AppHandle<R>,
    args: &[String],
    source: &str,
) -> bool {
    for arg in args {
        let candidate = arg.trim();
        if candidate.is_empty() {
            continue;
        }

        let Some(mut payload) = parse_external_import_url(candidate) else {
            continue;
        };
        payload.source = Some(source.to_string());
        payload.raw_url = Some(candidate.to_string());

        set_pending(payload.clone());

        if let Err(err) = floating_card_window::show_main_window_and_navigate(app, &payload.page) {
            logger::log_warn(&format!("[ExternalImport] 唤醒主窗口并导航失败: {}", err));
        }
        emit_external_import_payload(app, &payload);

        logger::log_info(&format!(
            "[ExternalImport] 已接收外部导入请求: provider={}, page={}, source={}",
            payload.provider_id, payload.page, source
        ));
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::parse_external_import_url;

    #[test]
    fn parse_basic_import_link() {
        let raw = "cockpit-tools://import?provider=codex&token=abc123";
        let payload = parse_external_import_url(raw).expect("payload");
        assert_eq!(payload.provider_id, "codex");
        assert_eq!(payload.page, "codex");
        assert_eq!(payload.token, "abc123");
        assert!(!payload.auto_import);
    }

    #[test]
    fn parse_alias_and_boolean() {
        let raw =
            "cockpit-tools://provider-import?platform=codebuddy-cn&payload=%7B%7D&auto_import=true";
        let payload = parse_external_import_url(raw).expect("payload");
        assert_eq!(payload.provider_id, "codebuddy_cn");
        assert_eq!(payload.page, "codebuddy-cn");
        assert_eq!(payload.token, "{}");
        assert!(payload.auto_import);
    }

    #[test]
    fn parse_antigravity_overview_alias() {
        let raw = "cockpittools://account-import?page=overview&token=1%2F%2F0gTokenDemo";
        let payload = parse_external_import_url(raw).expect("payload");
        assert_eq!(payload.provider_id, "antigravity");
        assert_eq!(payload.page, "overview");
        assert_eq!(payload.token, "1//0gTokenDemo");
        assert!(!payload.auto_import);
    }
}
