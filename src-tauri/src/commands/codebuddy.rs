use std::time::Instant;
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};
use tauri::{AppHandle, Emitter, Manager};

use crate::models::codebuddy::{CodebuddyAccount, CodebuddyOAuthStartResponse};
use crate::modules::{codebuddy_account, codebuddy_oauth, logger};

const CB_QUOTA_WEBVIEW_LABEL: &str = "cb-quota-wv";
const CB_OAUTH_WEBVIEW_LABEL: &str = "cb-oauth-wv";
const CB_WEBVIEW_SIGNAL_PATH: &str = "/__agtools_cb_quota_result__";
const CB_OAUTH_PREAUTH_SIGNAL_PATH: &str = "/__agtools_cb_oauth_quota_result__";
const CB_OAUTH_ACTION_SIGNAL_PATH: &str = "/__agtools_cb_oauth_action__";
const CODEBUDDY_WEB_ORIGIN: &str = "https://www.codebuddy.ai";
const CODEBUDDY_OAUTH_USAGE_URL: &str = "https://www.codebuddy.ai/agents?source=ide_login";
const CODEBUDDY_PROFILE_USAGE_URL: &str = "https://www.codebuddy.ai/profile/usage";

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodebuddyOauthWebviewUiTexts {
    pub manual_url_placeholder: Option<String>,
    pub manual_url_go: Option<String>,
    pub manual_url_invalid: Option<String>,
    pub quota_failure_prompt: Option<String>,
    pub quota_failure_title: Option<String>,
    pub quota_failure_retry_label: Option<String>,
    pub quota_failure_skip_label: Option<String>,
    pub oauth_success_close_prompt: Option<String>,
    pub oauth_success_close_title: Option<String>,
    pub oauth_success_close_now_label: Option<String>,
    pub oauth_success_close_later_label: Option<String>,
    pub oauth_success_close_now_status: Option<String>,
    pub oauth_success_close_later_status: Option<String>,
    pub oauth_step_quota_authorize: Option<String>,
    pub oauth_step_quota_bind: Option<String>,
    pub oauth_step_quota_complete: Option<String>,
    pub oauth_step_prepare: Option<String>,
    pub oauth_step_authorize: Option<String>,
    pub oauth_step_bind: Option<String>,
    pub oauth_step_quota: Option<String>,
    pub oauth_step_complete: Option<String>,
    pub oauth_status_login_confirm: Option<String>,
}

#[derive(Debug, Clone)]
struct ResolvedCodebuddyOauthWebviewUiTexts {
    manual_url_placeholder: String,
    manual_url_go: String,
    manual_url_invalid: String,
    quota_failure_prompt: String,
    quota_failure_title: String,
    quota_failure_retry_label: String,
    quota_failure_skip_label: String,
    oauth_success_close_prompt: String,
    oauth_success_close_title: String,
    oauth_success_close_now_label: String,
    oauth_success_close_later_label: String,
    oauth_success_close_now_status: String,
    oauth_success_close_later_status: String,
    oauth_step_quota_authorize: String,
    oauth_step_quota_bind: String,
    oauth_step_quota_complete: String,
    oauth_step_prepare: String,
    oauth_step_authorize: String,
    oauth_step_bind: String,
    oauth_step_quota: String,
    oauth_step_complete: String,
    oauth_status_login_confirm: String,
}

fn require_codebuddy_ui_text(value: Option<&String>, field: &str) -> Result<String, String> {
    let text = value
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| format!("ERR_CODEBUDDY_UI_TEXTS_MISSING_FIELD:{}", field))?;
    Ok(text.to_string())
}

fn resolve_codebuddy_oauth_webview_ui_texts(
    ui_texts: Option<CodebuddyOauthWebviewUiTexts>,
) -> Result<ResolvedCodebuddyOauthWebviewUiTexts, String> {
    let cfg = ui_texts.ok_or_else(|| "ERR_CODEBUDDY_UI_TEXTS_REQUIRED".to_string())?;
    Ok(ResolvedCodebuddyOauthWebviewUiTexts {
        manual_url_placeholder: require_codebuddy_ui_text(
            cfg.manual_url_placeholder.as_ref(),
            "manualUrlPlaceholder",
        )?,
        manual_url_go: require_codebuddy_ui_text(cfg.manual_url_go.as_ref(), "manualUrlGo")?,
        manual_url_invalid: require_codebuddy_ui_text(
            cfg.manual_url_invalid.as_ref(),
            "manualUrlInvalid",
        )?,
        quota_failure_prompt: require_codebuddy_ui_text(
            cfg.quota_failure_prompt.as_ref(),
            "quotaFailurePrompt",
        )?,
        quota_failure_title: require_codebuddy_ui_text(
            cfg.quota_failure_title.as_ref(),
            "quotaFailureTitle",
        )?,
        quota_failure_retry_label: require_codebuddy_ui_text(
            cfg.quota_failure_retry_label.as_ref(),
            "quotaFailureRetryLabel",
        )?,
        quota_failure_skip_label: require_codebuddy_ui_text(
            cfg.quota_failure_skip_label.as_ref(),
            "quotaFailureSkipLabel",
        )?,
        oauth_success_close_prompt: require_codebuddy_ui_text(
            cfg.oauth_success_close_prompt.as_ref(),
            "oauthSuccessClosePrompt",
        )?,
        oauth_success_close_title: require_codebuddy_ui_text(
            cfg.oauth_success_close_title.as_ref(),
            "oauthSuccessCloseTitle",
        )?,
        oauth_success_close_now_label: require_codebuddy_ui_text(
            cfg.oauth_success_close_now_label.as_ref(),
            "oauthSuccessCloseNowLabel",
        )?,
        oauth_success_close_later_label: require_codebuddy_ui_text(
            cfg.oauth_success_close_later_label.as_ref(),
            "oauthSuccessCloseLaterLabel",
        )?,
        oauth_success_close_now_status: require_codebuddy_ui_text(
            cfg.oauth_success_close_now_status.as_ref(),
            "oauthSuccessCloseNowStatus",
        )?,
        oauth_success_close_later_status: require_codebuddy_ui_text(
            cfg.oauth_success_close_later_status.as_ref(),
            "oauthSuccessCloseLaterStatus",
        )?,
        oauth_step_quota_authorize: require_codebuddy_ui_text(
            cfg.oauth_step_quota_authorize.as_ref(),
            "oauthStepQuotaAuthorize",
        )?,
        oauth_step_quota_bind: require_codebuddy_ui_text(
            cfg.oauth_step_quota_bind.as_ref(),
            "oauthStepQuotaBind",
        )?,
        oauth_step_quota_complete: require_codebuddy_ui_text(
            cfg.oauth_step_quota_complete.as_ref(),
            "oauthStepQuotaComplete",
        )?,
        oauth_step_prepare: require_codebuddy_ui_text(
            cfg.oauth_step_prepare.as_ref(),
            "oauthStepPrepare",
        )?,
        oauth_step_authorize: require_codebuddy_ui_text(
            cfg.oauth_step_authorize.as_ref(),
            "oauthStepAuthorize",
        )?,
        oauth_step_bind: require_codebuddy_ui_text(cfg.oauth_step_bind.as_ref(), "oauthStepBind")?,
        oauth_step_quota: require_codebuddy_ui_text(
            cfg.oauth_step_quota.as_ref(),
            "oauthStepQuota",
        )?,
        oauth_step_complete: require_codebuddy_ui_text(
            cfg.oauth_step_complete.as_ref(),
            "oauthStepComplete",
        )?,
        oauth_status_login_confirm: require_codebuddy_ui_text(
            cfg.oauth_status_login_confirm.as_ref(),
            "oauthStatusLoginConfirm",
        )?,
    })
}

fn handle_oauth_webview_snapshot(app: &AppHandle, raw_payload: String) -> Result<(), String> {
    let encoded =
        enrich_webview_snapshot_with_native_cookie(app, CB_OAUTH_WEBVIEW_LABEL, raw_payload, "OAuth");

    codebuddy_oauth::cache_pre_auth_quota_snapshot(&encoded)?;
    let _ = app.emit(
        "codebuddy-oauth-webview-stage",
        serde_json::json!({ "stage": "quota_ready" }),
    );
    logger::log_info("[CodeBuddy OAuth] WebView 已通过 IPC 提交预拉取配额快照");
    Ok(())
}

fn handle_quota_webview_snapshot(
    app: &AppHandle,
    account_id: String,
    raw_payload: String,
) -> Result<(), String> {
    let encoded =
        enrich_webview_snapshot_with_native_cookie(app, CB_QUOTA_WEBVIEW_LABEL, raw_payload, "Quota");
    let app_emit = app.clone();

    tauri::async_runtime::spawn(async move {
        match codebuddy_account::apply_webview_quota_result(&account_id, &encoded).await {
            Ok(account) => {
                if let Err(e) = codebuddy_account::run_quota_alert_if_needed() {
                    logger::log_warn(&format!(
                        "[QuotaAlert][CodeBuddy] WebView 查询后预警检查失败: {}",
                        e
                    ));
                }
                let _ = crate::modules::tray::update_tray_menu(&app_emit);
                let _ = app_emit.emit("codebuddy-webview-quota-success", &account);
                logger::log_info(&format!(
                    "[CodeBuddy WebView Quota] 查询成功: account_id={}, email={}",
                    account.id, account.email
                ));
            }
            Err(e) => {
                logger::log_warn(&format!("[CodeBuddy WebView Quota] 处理失败: {}", e));
                let _ = app_emit.emit("codebuddy-webview-quota-error", &e);
            }
        }
        if let Some(w) = app_emit.get_webview_window(CB_QUOTA_WEBVIEW_LABEL) {
            let _ = w.close();
        }
    });

    Ok(())
}

fn dispatch_oauth_webview_action(
    app: &AppHandle,
    action: &str,
    auth_url: Option<&str>,
) -> Result<(), String> {
    match action {
        "quota_retry" | "quota_skip" => {
            codebuddy_oauth::set_quota_bind_decision(action)?;
            logger::log_info(&format!(
                "[CodeBuddy OAuth] 收到 WebView 配额失败决策: action={}",
                action
            ));
            Ok(())
        }
        "retry_bind" => {
            logger::log_info("[CodeBuddy OAuth] 收到 WebView 绑定重试请求");
            let _ = app.emit(
                "codebuddy-oauth-webview-action",
                serde_json::json!({ "action": action }),
            );
            Ok(())
        }
        "close_webview" => {
            logger::log_info("[CodeBuddy OAuth] 收到关闭 WebView 指令");
            close_codebuddy_oauth_webview(app);
            Ok(())
        }
        "retry_quota" => {
            let window = app
                .get_webview_window(CB_OAUTH_WEBVIEW_LABEL)
                .ok_or_else(|| "ERR_CODEBUDDY_OAUTH_WEBVIEW_NOT_OPEN".to_string())?;
            logger::log_info("[CodeBuddy OAuth] WebView 手动重试：重新执行配额抓取");
            let _ = window
                .eval("try{sessionStorage.removeItem('__agtools_cb_oauth_phase__')}catch(_e){}");
            let usage_url = CODEBUDDY_OAUTH_USAGE_URL
                .parse::<url::Url>()
                .map_err(|e| format!("CodeBuddy usage URL parse error: {}", e))?;
            window
                .navigate(usage_url)
                .map_err(|e| format!("ERR_CODEBUDDY_OAUTH_RETRY_QUOTA_FAILED:{}", e))?;
            let _ = app.emit(
                "codebuddy-oauth-webview-stage",
                serde_json::json!({ "stage": "usage_page" }),
            );
            Ok(())
        }
        "goto_authorize" => {
            let window = app
                .get_webview_window(CB_OAUTH_WEBVIEW_LABEL)
                .ok_or_else(|| "ERR_CODEBUDDY_OAUTH_WEBVIEW_NOT_OPEN".to_string())?;
            let auth_url_text =
                auth_url.ok_or_else(|| "ERR_CODEBUDDY_OAUTH_AUTH_URL_MISSING".to_string())?;
            logger::log_info(&format!(
                "[CodeBuddy OAuth] WebView 手动推进：跳转授权 URL: {}",
                auth_url_text
            ));
            let _ = window.eval(
                "try{sessionStorage.setItem('__agtools_cb_oauth_phase__','authorize')}catch(_e){}",
            );
            let target = auth_url_text
                .parse::<url::Url>()
                .map_err(|e| format!("OAuth URL parse error: {}", e))?;
            window
                .navigate(target)
                .map_err(|e| format!("ERR_CODEBUDDY_OAUTH_GOTO_AUTHORIZE_FAILED:{}", e))?;
            let _ = app.emit(
                "codebuddy-oauth-webview-stage",
                serde_json::json!({ "stage": "wait_authorize" }),
            );
            Ok(())
        }
        _ => Err(format!(
            "ERR_CODEBUDDY_OAUTH_WEBVIEW_ACTION_UNKNOWN:{}",
            action
        )),
    }
}

fn safe_webview_url_summary(nav_url: &url::Url) -> String {
    let host = nav_url.host_str().unwrap_or("-");
    let query_len = nav_url.query().map(|q| q.len()).unwrap_or(0);
    format!(
        "host={}, path={}, query_len={}",
        host,
        nav_url.path(),
        query_len
    )
}

fn emit_stage_log_once(
    stage_ref: &Arc<Mutex<String>>,
    logger_prefix: &str,
    stage: &str,
    nav_url: &url::Url,
) {
    let mut changed = true;
    if let Ok(mut guard) = stage_ref.lock() {
        if guard.as_str() == stage {
            changed = false;
        } else {
            *guard = stage.to_string();
        }
    }
    if changed {
        logger::log_info(&format!(
            "[{}] stage={} ({})",
            logger_prefix,
            stage,
            safe_webview_url_summary(nav_url)
        ));
    }
}

fn resolve_oauth_webview_stage(nav_url: &url::Url) -> &'static str {
    let path = nav_url.path().to_lowercase();
    let query = nav_url.query().unwrap_or("").to_lowercase();
    if path.starts_with(CB_OAUTH_PREAUTH_SIGNAL_PATH) {
        return "quota_ready";
    }
    if path.contains("/profile/usage") || path.contains("/agents") {
        return "usage_page";
    }
    if path.contains("/login") {
        return "wait_login";
    }
    if query.contains("platform=ide")
        || query.contains("state=")
        || path.contains("/oauth")
        || path.contains("/authorize")
    {
        return "wait_authorize";
    }
    if path.contains("/profile") || path.contains("/workspace") {
        return "authorized";
    }
    "navigating"
}

fn resolve_quota_webview_stage(nav_url: &url::Url) -> &'static str {
    let path = nav_url.path().to_lowercase();
    if path.starts_with(CB_WEBVIEW_SIGNAL_PATH) {
        return "signal_received";
    }
    if path.contains("/login") {
        return "wait_login";
    }
    if path.contains("/profile/usage") {
        return "usage_page";
    }
    "navigating"
}

fn default_quota_time_range() -> (String, String) {
    let now = chrono::Utc::now();
    let begin = now.format("%Y-%m-%d %H:%M:%S").to_string();
    let end = (now + chrono::Duration::days(365 * 100))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    (begin, end)
}

fn merge_cookie_headers(primary: &str, secondary: &str) -> String {
    let mut seen = HashSet::new();
    let mut merged = Vec::new();
    for source in [primary, secondary] {
        for segment in source.split(';') {
            let part = segment.trim();
            if part.is_empty() {
                continue;
            }
            let mut pieces = part.splitn(2, '=');
            let key = pieces.next().unwrap_or("").trim();
            let value = pieces.next().unwrap_or("").trim();
            if key.is_empty() || value.is_empty() {
                continue;
            }
            let lowered = key.to_lowercase();
            if seen.insert(lowered) {
                merged.push(format!("{}={}", key, value));
            }
        }
    }
    merged.join("; ")
}

fn resolve_native_cookie_header(app: &AppHandle, label: &str) -> Result<String, String> {
    let window = app
        .get_webview_window(label)
        .ok_or_else(|| format!("WebView 不存在: {}", label))?;
    let targets = [
        "https://www.codebuddy.ai/billing/meter/get-user-resource",
        "https://www.codebuddy.ai/",
    ];

    let mut merged_all = String::new();
    let mut errors: Vec<String> = Vec::new();

    for target in targets {
        let url = match target.parse::<url::Url>() {
            Ok(v) => v,
            Err(e) => {
                errors.push(format!("{}:parse_error={}", target, e));
                continue;
            }
        };

        let cookies = match window.cookies_for_url(url) {
            Ok(v) => v,
            Err(e) => {
                errors.push(format!("{}:read_error={}", target, e));
                continue;
            }
        };

        let mut seen = HashSet::new();
        let mut merged = Vec::new();
        for cookie in cookies {
            let key = cookie.name().trim();
            let value = cookie.value().trim();
            if key.is_empty() || value.is_empty() {
                continue;
            }
            let lowered = key.to_lowercase();
            if seen.insert(lowered) {
                merged.push(format!("{}={}", key, value));
            }
        }
        if merged.is_empty() {
            errors.push(format!("{}:empty", target));
            continue;
        }
        let header = merged.join("; ");
        if merged_all.is_empty() {
            merged_all = header;
        } else {
            merged_all = merge_cookie_headers(&merged_all, &header);
        }
    }

    if merged_all.is_empty() {
        let detail = if errors.is_empty() {
            "unknown".to_string()
        } else {
            errors.join(" | ")
        };
        return Err(format!("CookieStore 为空: {}", detail));
    }
    Ok(merged_all)
}

fn inject_cookie_binding_payload(
    raw_json: &str,
    cookie_header: &str,
    scene: &str,
) -> Result<String, String> {
    let mut payload: serde_json::Value = serde_json::from_str(raw_json)
        .map_err(|e| format!("解析 WebView 信号 JSON 失败: {}", e))?;
    let Some(root) = payload.as_object_mut() else {
        return Err("WebView 信号 payload 不是对象".to_string());
    };
    let binding_entry = root
        .entry("binding".to_string())
        .or_insert_with(|| serde_json::json!({}));
    if !binding_entry.is_object() {
        *binding_entry = serde_json::json!({});
    }
    let binding = binding_entry
        .as_object_mut()
        .ok_or_else(|| "构造 binding 对象失败".to_string())?;

    let existing_cookie_header = binding
        .get("cookieHeader")
        .or_else(|| binding.get("cookie_header"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let merged_cookie_header = merge_cookie_headers(cookie_header, existing_cookie_header);
    if merged_cookie_header.is_empty() {
        return Err("可用 Cookie 为空".to_string());
    }

    let (default_begin, default_end) = default_quota_time_range();
    binding.insert(
        "cookieHeader".to_string(),
        serde_json::Value::String(merged_cookie_header.clone()),
    );
    if !binding.contains_key("productCode") {
        binding.insert(
            "productCode".to_string(),
            serde_json::Value::String("p_tcaca".to_string()),
        );
    }
    if !binding.contains_key("status") {
        binding.insert("status".to_string(), serde_json::json!([0, 3]));
    }
    if !binding.contains_key("packageEndTimeRangeBegin") {
        binding.insert(
            "packageEndTimeRangeBegin".to_string(),
            serde_json::Value::String(default_begin),
        );
    }
    if !binding.contains_key("packageEndTimeRangeEnd") {
        binding.insert(
            "packageEndTimeRangeEnd".to_string(),
            serde_json::Value::String(default_end),
        );
    }
    if !binding.contains_key("pageNumber") {
        binding.insert(
            "pageNumber".to_string(),
            serde_json::Value::Number(serde_json::Number::from(1)),
        );
    }
    if !binding.contains_key("pageSize") {
        binding.insert(
            "pageSize".to_string(),
            serde_json::Value::Number(serde_json::Number::from(100)),
        );
    }

    let request_url = binding
        .get("requestUrl")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("{}/billing/meter/get-user-resource", CODEBUDDY_WEB_ORIGIN));
    let request_method = binding
        .get("requestMethod")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "POST".to_string());
    let default_request_referrer = if scene.eq_ignore_ascii_case("oauth") {
        CODEBUDDY_OAUTH_USAGE_URL.to_string()
    } else {
        format!("{}/profile/usage", CODEBUDDY_WEB_ORIGIN)
    };
    let request_referrer = binding
        .get("requestReferrer")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default_request_referrer.clone());

    let existing_headers = binding
        .get("requestHeaders")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let read_header = |name: &str| -> Option<String> {
        existing_headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .and_then(|(_, v)| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };

    let mut request_headers = serde_json::Map::new();
    request_headers.insert(
        "Accept".to_string(),
        serde_json::Value::String(
            read_header("Accept")
                .unwrap_or_else(|| "application/json, text/plain, */*".to_string()),
        ),
    );
    request_headers.insert(
        "Accept-Language".to_string(),
        serde_json::Value::String(
            read_header("Accept-Language").unwrap_or_else(|| "zh-CN,zh;q=0.9".to_string()),
        ),
    );
    request_headers.insert(
        "Content-Type".to_string(),
        serde_json::Value::String(
            read_header("Content-Type").unwrap_or_else(|| "application/json".to_string()),
        ),
    );
    request_headers.insert(
        "Origin".to_string(),
        serde_json::Value::String(
            read_header("Origin").unwrap_or_else(|| CODEBUDDY_WEB_ORIGIN.to_string()),
        ),
    );
    request_headers.insert(
        "Referer".to_string(),
        serde_json::Value::String(
            read_header("Referer").unwrap_or_else(|| request_referrer.clone()),
        ),
    );
    request_headers.insert(
        "User-Agent".to_string(),
        serde_json::Value::String(read_header("User-Agent").unwrap_or_else(|| {
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36".to_string()
        })),
    );
    request_headers.insert(
        "Sec-Fetch-Site".to_string(),
        serde_json::Value::String(
            read_header("Sec-Fetch-Site").unwrap_or_else(|| "same-origin".to_string()),
        ),
    );
    request_headers.insert(
        "Sec-Fetch-Mode".to_string(),
        serde_json::Value::String(
            read_header("Sec-Fetch-Mode").unwrap_or_else(|| "cors".to_string()),
        ),
    );
    request_headers.insert(
        "Sec-Fetch-Dest".to_string(),
        serde_json::Value::String(
            read_header("Sec-Fetch-Dest").unwrap_or_else(|| "empty".to_string()),
        ),
    );
    request_headers.insert(
        "Cookie".to_string(),
        serde_json::Value::String(merged_cookie_header.clone()),
    );

    let request_body = binding
        .get("requestBody")
        .cloned()
        .unwrap_or_else(|| {
            serde_json::json!({
                "PageNumber": binding.get("pageNumber").cloned().unwrap_or_else(|| serde_json::Value::Number(serde_json::Number::from(1))),
                "PageSize": binding.get("pageSize").cloned().unwrap_or_else(|| serde_json::Value::Number(serde_json::Number::from(100))),
                "ProductCode": binding.get("productCode").cloned().unwrap_or_else(|| serde_json::Value::String("p_tcaca".to_string())),
                "Status": binding.get("status").cloned().unwrap_or_else(|| serde_json::json!([0, 3])),
                "PackageEndTimeRangeBegin": binding
                    .get("packageEndTimeRangeBegin")
                    .cloned()
                    .unwrap_or_else(|| serde_json::Value::String(default_quota_time_range().0)),
                "PackageEndTimeRangeEnd": binding
                    .get("packageEndTimeRangeEnd")
                    .cloned()
                    .unwrap_or_else(|| serde_json::Value::String(default_quota_time_range().1)),
            })
        });

    binding.insert(
        "requestUrl".to_string(),
        serde_json::Value::String(request_url.clone()),
    );
    binding.insert(
        "requestMethod".to_string(),
        serde_json::Value::String(request_method.clone()),
    );
    binding.insert(
        "requestReferrer".to_string(),
        serde_json::Value::String(request_referrer.clone()),
    );
    binding.insert(
        "requestHeaders".to_string(),
        serde_json::Value::Object(request_headers.clone()),
    );
    binding.insert("requestBody".to_string(), request_body.clone());

    serde_json::to_string(&payload).map_err(|e| format!("序列化 WebView 信号 JSON 失败: {}", e))
}

fn enrich_webview_snapshot_with_native_cookie(
    app: &AppHandle,
    label: &str,
    raw_json: String,
    scene: &str,
) -> String {
    match resolve_native_cookie_header(app, label) {
        Ok(cookie_header) => {
            match inject_cookie_binding_payload(&raw_json, &cookie_header, scene) {
                Ok(enriched) => {
                    logger::log_info(&format!(
                        "[CodeBuddy {}] 已注入原生 CookieStore 数据",
                        scene
                    ));
                    enriched
                }
                Err(err) => {
                    logger::log_warn(&format!(
                        "[CodeBuddy {}] 注入 Cookie 绑定失败，回退原始 payload: {}",
                        scene, err
                    ));
                    raw_json
                }
            }
        }
        Err(err) => {
            logger::log_warn(&format!(
                "[CodeBuddy {}] 读取原生 CookieStore 失败，回退原始 payload: {}",
                scene, err
            ));
            raw_json
        }
    }
}

fn close_codebuddy_webview(app: &AppHandle, label: &str) {
    if let Some(existing) = app.get_webview_window(label) {
        let _ = existing.close();
    }
}

fn close_codebuddy_oauth_webview(app: &AppHandle) {
    close_codebuddy_webview(app, CB_OAUTH_WEBVIEW_LABEL);
}

fn mark_codebuddy_oauth_webview_success(app: &AppHandle) {
    let Some(window) = app.get_webview_window(CB_OAUTH_WEBVIEW_LABEL) else {
        logger::log_warn(
            "[CodeBuddy OAuth Debug] mark_success 时未找到 OAuth WebView，无法注入完成弹窗",
        );
        return;
    };
    let script = r#"(function(){
  try{
    var payload={
      href:String(window.location.href||''),
      hasCustomMarkSuccess:typeof window.__agtools_cb_oauth_mark_success==='function',
      hasPanel:!!document.getElementById('__agtools_oauth_panel'),
      hasMsg:!!document.getElementById('__agtools_oauth_msg'),
      readyState:String(document.readyState||'')
    };
    console.info('[AGTOOLS_CB_OAUTH_DEBUG][mark_success_inject] '+JSON.stringify(payload));
  }catch(_e){}
  if(typeof window.__agtools_cb_oauth_mark_success==='function'){
    window.__agtools_cb_oauth_mark_success();
    return;
  }
  var bar=document.getElementById('__agtools_oauth_status');
  if(!bar){
    bar=document.createElement('div');
    bar.id='__agtools_oauth_status';
    bar.style.cssText='position:fixed;top:0;left:0;right:0;z-index:2147483647;padding:10px 14px;font:600 13px/1.4 -apple-system,BlinkMacSystemFont,Segoe UI,sans-serif;color:#fff;text-align:center;background:#16a34a;box-shadow:0 2px 8px rgba(0,0,0,.25);';
    (document.body||document.documentElement).appendChild(bar);
  } else {
    bar.style.background='#16a34a';
  }
  bar.textContent='✅ 授权成功，请手动关闭此窗口';
})();"#;

    logger::log_info("[CodeBuddy OAuth Debug] 开始注入 mark_success 脚本");
    if let Err(err) = window.eval(script) {
        logger::log_warn(&format!(
            "[CodeBuddy OAuth] 写入成功提示失败（可忽略）: {}",
            err
        ));
    } else {
        logger::log_info("[CodeBuddy OAuth] WebView 已展示成功提示，等待用户手动关闭");
    }
}

fn mark_codebuddy_oauth_webview_account_bound(app: &AppHandle) {
    let Some(window) = app.get_webview_window(CB_OAUTH_WEBVIEW_LABEL) else {
        return;
    };
    let script = r#"(function(){
  if(typeof window.__agtools_cb_oauth_mark_account_bound==='function'){
    window.__agtools_cb_oauth_mark_account_bound();
  }
})();"#;

    if let Err(err) = window.eval(script) {
        logger::log_warn(&format!(
            "[CodeBuddy OAuth] 写入“账号已绑定”提示失败（可忽略）: {}",
            err
        ));
    } else {
        logger::log_info("[CodeBuddy OAuth] WebView 已标记“账号已绑定”，继续 Cookie 绑定阶段");
    }
}

fn prompt_codebuddy_oauth_webview_quota_failure(app: &AppHandle, reason: &str) {
    let Some(window) = app.get_webview_window(CB_OAUTH_WEBVIEW_LABEL) else {
        return;
    };
    let reason_literal = serde_json::to_string(reason).unwrap_or_else(|_| "\"\"".to_string());
    let script = format!(
        r#"(function(){{
  if(typeof window.__agtools_cb_oauth_prompt_quota_failure==='function'){{
    window.__agtools_cb_oauth_prompt_quota_failure({0});
    return;
  }}
  if(typeof window.__agtools_cb_oauth_notify_quota_failure==='function'){{
    window.__agtools_cb_oauth_notify_quota_failure({0});
  }}
}})();"#,
        reason_literal
    );
    if let Err(err) = window.eval(&script) {
        logger::log_warn(&format!(
            "[CodeBuddy OAuth] 触发 WebView 配额失败弹框失败（可忽略）: {}",
            err
        ));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QuotaFailureDecision {
    Retry,
    Skip,
}

fn trigger_codebuddy_oauth_webview_retry_quota(app: &AppHandle) {
    let Some(window) = app.get_webview_window(CB_OAUTH_WEBVIEW_LABEL) else {
        return;
    };
    let script = r#"(function(){
  if(typeof window.__agtools_cb_oauth_retry_quota_capture==='function'){
    window.__agtools_cb_oauth_retry_quota_capture();
  }
})();"#;
    if let Err(err) = window.eval(script) {
        logger::log_warn(&format!(
            "[CodeBuddy OAuth] 触发 WebView 重试配额采集失败（可忽略）: {}",
            err
        ));
    } else {
        logger::log_info("[CodeBuddy OAuth] 已触发 WebView 重试配额采集");
    }
}

async fn wait_codebuddy_quota_failure_decision(
    app: &AppHandle,
    login_id: &str,
    reason: &str,
) -> Result<QuotaFailureDecision, String> {
    prompt_codebuddy_oauth_webview_quota_failure(app, reason);
    let action = codebuddy_oauth::wait_quota_bind_decision(login_id, 600).await?;
    let decision = match action.as_deref() {
        Some("quota_retry") => QuotaFailureDecision::Retry,
        Some("quota_skip") => QuotaFailureDecision::Skip,
        _ => {
            logger::log_warn("[CodeBuddy OAuth] 未收到配额失败决策，默认按跳过处理");
            QuotaFailureDecision::Skip
        }
    };
    Ok(decision)
}

fn mark_codebuddy_oauth_webview_quota_failed_done(app: &AppHandle) {
    let Some(window) = app.get_webview_window(CB_OAUTH_WEBVIEW_LABEL) else {
        return;
    };
    let script = r#"(function(){
  if(typeof window.__agtools_cb_oauth_mark_quota_failed_done==='function'){
    window.__agtools_cb_oauth_mark_quota_failed_done();
  }
})();"#;
    if let Err(err) = window.eval(script) {
        logger::log_warn(&format!(
            "[CodeBuddy OAuth] 写入“账号已添加但配额失败”提示失败（可忽略）: {}",
            err
        ));
    }
}

async fn refresh_codebuddy_account_after_login(account: CodebuddyAccount) -> CodebuddyAccount {
    let account_id = account.id.clone();
    match codebuddy_account::refresh_account_token(&account_id).await {
        Ok(refreshed) => refreshed,
        Err(e) => {
            logger::log_warn(&format!(
                "[CodeBuddy OAuth] 登录后自动刷新失败: account_id={}, error={}",
                account_id, e
            ));
            account
        }
    }
}

async fn refresh_codebuddy_account_after_login_strict(
    account: CodebuddyAccount,
) -> Result<CodebuddyAccount, String> {
    let account_id = account.id.clone();
    match codebuddy_account::refresh_account_token_strict(&account_id).await {
        Ok(refreshed) => Ok(refreshed),
        Err(e) => {
            logger::log_warn(&format!(
                "[CodeBuddy OAuth] 登录后严格刷新失败: account_id={}, error={}",
                account_id, e
            ));
            Err(e)
        }
    }
}

#[tauri::command]
pub fn list_codebuddy_accounts() -> Result<Vec<CodebuddyAccount>, String> {
    Ok(codebuddy_account::list_accounts())
}

#[tauri::command]
pub fn delete_codebuddy_account(account_id: String) -> Result<(), String> {
    codebuddy_account::remove_account(&account_id)
}

#[tauri::command]
pub fn delete_codebuddy_accounts(account_ids: Vec<String>) -> Result<(), String> {
    codebuddy_account::remove_accounts(&account_ids)
}

#[tauri::command]
pub fn import_codebuddy_from_json(json_content: String) -> Result<Vec<CodebuddyAccount>, String> {
    codebuddy_account::import_from_json(&json_content)
}

#[tauri::command]
pub async fn import_codebuddy_from_local(app: AppHandle) -> Result<Vec<CodebuddyAccount>, String> {
    let mut local_payload = match codebuddy_account::import_payload_from_local()? {
        Some(payload) => payload,
        None => return Err("未在本机 CodeBuddy 客户端中找到登录信息".to_string()),
    };

    match codebuddy_oauth::build_payload_from_token(&local_payload.access_token).await {
        Ok(mut payload) => {
            if payload.uid.is_none() {
                payload.uid = local_payload.uid.clone();
            }
            if payload.nickname.is_none() {
                payload.nickname = local_payload.nickname.clone();
            }
            if payload.refresh_token.is_none() {
                payload.refresh_token = local_payload.refresh_token.clone();
            }
            if payload.domain.is_none() {
                payload.domain = local_payload.domain.clone();
            }
            if payload.token_type.is_none() {
                payload.token_type = local_payload.token_type.clone();
            }
            if payload.expires_at.is_none() {
                payload.expires_at = local_payload.expires_at;
            }
            if payload.auth_raw.is_none() {
                payload.auth_raw = local_payload.auth_raw.clone();
            }
            if payload.profile_raw.is_none() {
                payload.profile_raw = local_payload.profile_raw.clone();
            }
            if payload.email.trim().is_empty() || payload.email == "unknown" {
                payload.email = local_payload.email.clone();
            }
            local_payload = payload;
        }
        Err(err) => {
            logger::log_warn(&format!(
                "[CodeBuddy Import Local] 拉取账号资料失败，将保留本地导入结果: {}",
                err
            ));
        }
    }

    let mut account = codebuddy_account::upsert_account(local_payload.clone())?;

    // 历史版本本地导入会先写入 unknown 占位账号；这里按同 token 清理旧占位记录。
    for existing in codebuddy_account::list_accounts() {
        if existing.id == account.id {
            continue;
        }
        if existing.access_token != account.access_token {
            continue;
        }
        let is_placeholder = existing.email.trim().eq_ignore_ascii_case("unknown")
            || existing.email.trim().is_empty()
            || existing
                .uid
                .as_deref()
                .map(|s| s.trim().is_empty())
                .unwrap_or(true);
        if is_placeholder {
            if let Err(err) = codebuddy_account::remove_account(&existing.id) {
                logger::log_warn(&format!(
                    "[CodeBuddy Import Local] 清理占位账号失败: id={}, error={}",
                    existing.id, err
                ));
            }
        }
    }

    account = refresh_codebuddy_account_after_login(account).await;
    let _ = crate::modules::tray::update_tray_menu(&app);
    Ok(vec![account])
}

#[tauri::command]
pub fn export_codebuddy_accounts(account_ids: Vec<String>) -> Result<String, String> {
    codebuddy_account::export_accounts(&account_ids)
}

#[tauri::command]
pub async fn refresh_codebuddy_token(
    app: AppHandle,
    account_id: String,
) -> Result<CodebuddyAccount, String> {
    let started_at = Instant::now();
    logger::log_info(&format!(
        "[CodeBuddy Command] 手动刷新账号开始: account_id={}",
        account_id
    ));

    match codebuddy_account::refresh_account_token(&account_id).await {
        Ok(account) => {
            if let Err(e) = codebuddy_account::run_quota_alert_if_needed() {
                logger::log_warn(&format!("[QuotaAlert][CodeBuddy] 预警检查失败: {}", e));
            }
            let _ = crate::modules::tray::update_tray_menu(&app);
            logger::log_info(&format!(
                "[CodeBuddy Command] 手动刷新账号完成: account_id={}, email={}, elapsed={}ms",
                account.id,
                account.email,
                started_at.elapsed().as_millis()
            ));
            Ok(account)
        }
        Err(err) => {
            logger::log_warn(&format!(
                "[CodeBuddy Command] 手动刷新账号失败: account_id={}, elapsed={}ms, error={}",
                account_id,
                started_at.elapsed().as_millis(),
                err
            ));
            Err(err)
        }
    }
}

#[tauri::command]
pub async fn refresh_all_codebuddy_tokens(app: AppHandle) -> Result<i32, String> {
    let started_at = Instant::now();
    logger::log_info("[CodeBuddy Command] 手动批量刷新开始");

    let results = codebuddy_account::refresh_all_tokens().await?;
    let success_count = results.iter().filter(|(_, item)| item.is_ok()).count();
    let failed_count = results.len().saturating_sub(success_count);

    logger::log_info(&format!(
        "[CodeBuddy Command] 手动批量刷新完成: success={}, failed={}, elapsed={}ms",
        success_count,
        failed_count,
        started_at.elapsed().as_millis()
    ));

    if success_count > 0 {
        if let Err(e) = codebuddy_account::run_quota_alert_if_needed() {
            logger::log_warn(&format!(
                "[QuotaAlert][CodeBuddy] 全量刷新后预警检查失败: {}",
                e
            ));
        }
    }

    let _ = crate::modules::tray::update_tray_menu(&app);
    Ok(success_count as i32)
}

#[tauri::command]
pub async fn query_codebuddy_quota_with_binding(
    app: AppHandle,
    account_id: String,
    cookie_header: String,
    product_code: Option<String>,
    status: Option<Vec<i32>>,
    package_end_time_range_begin: Option<String>,
    package_end_time_range_end: Option<String>,
    page_number: Option<i32>,
    page_size: Option<i32>,
) -> Result<CodebuddyAccount, String> {
    let started_at = Instant::now();
    logger::log_info(&format!(
        "[CodeBuddy Quota Query] 开始查询: account_id={}",
        account_id
    ));

    let updated = codebuddy_account::query_quota_with_binding(
        &account_id,
        &cookie_header,
        product_code,
        status,
        package_end_time_range_begin,
        package_end_time_range_end,
        page_number,
        page_size,
        None,
    )
    .await?;

    if let Err(e) = codebuddy_account::run_quota_alert_if_needed() {
        logger::log_warn(&format!(
            "[QuotaAlert][CodeBuddy] 手动查询后预警检查失败: {}",
            e
        ));
    }
    let _ = crate::modules::tray::update_tray_menu(&app);

    logger::log_info(&format!(
        "[CodeBuddy Quota Query] 查询完成: account_id={}, email={}, elapsed={}ms",
        updated.id,
        updated.email,
        started_at.elapsed().as_millis()
    ));
    Ok(updated)
}

#[tauri::command]
pub fn clear_codebuddy_quota_binding(
    app: AppHandle,
    account_id: String,
) -> Result<CodebuddyAccount, String> {
    logger::log_info(&format!(
        "[CodeBuddy Quota Query] 清除绑定: account_id={}",
        account_id
    ));
    let updated = codebuddy_account::clear_quota_binding(&account_id)?;
    let _ = crate::modules::tray::update_tray_menu(&app);
    logger::log_info(&format!(
        "[CodeBuddy Quota Query] 清除绑定完成: account_id={}, email={}",
        updated.id, updated.email
    ));
    Ok(updated)
}

#[tauri::command]
pub async fn codebuddy_oauth_login_start() -> Result<CodebuddyOAuthStartResponse, String> {
    logger::log_info("CodeBuddy OAuth start 命令触发");
    codebuddy_oauth::start_login().await
}

#[tauri::command]
pub async fn codebuddy_oauth_login_complete(
    app: AppHandle,
    login_id: String,
) -> Result<CodebuddyAccount, String> {
    logger::log_info(&format!(
        "CodeBuddy OAuth complete 命令触发: login_id={}",
        login_id
    ));
    let result: Result<CodebuddyAccount, String> = async {
        let payload = codebuddy_oauth::complete_login(&login_id).await?;
        let mut account = codebuddy_account::upsert_account(payload)?;
        mark_codebuddy_oauth_webview_account_bound(&app);

        let wait_pre_auth_snapshot = codebuddy_oauth::should_wait_pre_auth_snapshot(&login_id)?;
        if wait_pre_auth_snapshot {
            loop {
                let snapshot = codebuddy_oauth::wait_pre_auth_snapshot(
                    &login_id,
                    codebuddy_oauth::PRE_AUTH_SNAPSHOT_WAIT_SECONDS,
                )
                .await?;
                let Some(snapshot) = snapshot else {
                    let reason = "WebView 未捕获到有效配额会话（缺少 session/session_2）";
                    match wait_codebuddy_quota_failure_decision(&app, &login_id, reason).await? {
                        QuotaFailureDecision::Retry => {
                            trigger_codebuddy_oauth_webview_retry_quota(&app);
                            continue;
                        }
                        QuotaFailureDecision::Skip => {
                            if let Ok(cleared) = codebuddy_account::clear_quota_binding(&account.id)
                            {
                                account = cleared;
                            }
                            account = refresh_codebuddy_account_after_login(account).await;
                            break;
                        }
                    }
                };

                let snapshot_json = serde_json::to_string(&snapshot)
                    .map_err(|e| format!("序列化 WebView 快照失败: {}", e))?;
                match codebuddy_account::apply_webview_quota_result(&account.id, &snapshot_json)
                    .await
                {
                    Ok(updated) => {
                        account = updated;
                        match refresh_codebuddy_account_after_login_strict(account.clone()).await {
                            Ok(refreshed) => {
                                account = refreshed;
                                break;
                            }
                            Err(err) => {
                                match wait_codebuddy_quota_failure_decision(&app, &login_id, &err)
                                    .await?
                                {
                                    QuotaFailureDecision::Retry => {
                                        trigger_codebuddy_oauth_webview_retry_quota(&app);
                                        continue;
                                    }
                                    QuotaFailureDecision::Skip => {
                                        if let Ok(cleared) =
                                            codebuddy_account::clear_quota_binding(&account.id)
                                        {
                                            account = cleared;
                                        }
                                        account =
                                            refresh_codebuddy_account_after_login(account).await;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Err(err) => match wait_codebuddy_quota_failure_decision(&app, &login_id, &err)
                        .await?
                    {
                        QuotaFailureDecision::Retry => {
                            trigger_codebuddy_oauth_webview_retry_quota(&app);
                            continue;
                        }
                        QuotaFailureDecision::Skip => {
                            if let Ok(cleared) = codebuddy_account::clear_quota_binding(&account.id)
                            {
                                account = cleared;
                            }
                            account = refresh_codebuddy_account_after_login(account).await;
                            break;
                        }
                    },
                }
            }
        } else {
            account = refresh_codebuddy_account_after_login(account).await;
        }
        Ok(account)
    }
    .await;
    if let Err(err) = codebuddy_oauth::clear_pending_oauth_login(&login_id) {
        logger::log_warn(&format!(
            "[CodeBuddy OAuth] 清理待处理登录状态失败: login_id={}, error={}",
            login_id, err
        ));
    }
    let account = result?;
    let oauth_webview_exists = app.get_webview_window(CB_OAUTH_WEBVIEW_LABEL).is_some();
    logger::log_warn(&format!(
        "[CodeBuddy OAuth Debug] 准备触发完成弹窗: login_id={}, account_id={}, webview_exists={}",
        login_id, account.id, oauth_webview_exists
    ));
    mark_codebuddy_oauth_webview_success(&app);
    let _ = app.emit(
        "codebuddy-oauth-webview-stage",
        serde_json::json!({ "stage": "done" }),
    );
    logger::log_info(&format!(
        "CodeBuddy OAuth complete 成功: account_id={}, email={}",
        account.id, account.email
    ));
    let _ = crate::modules::tray::update_tray_menu(&app);
    Ok(account)
}

#[tauri::command]
pub fn codebuddy_oauth_login_cancel(
    app: AppHandle,
    login_id: Option<String>,
) -> Result<(), String> {
    logger::log_info(&format!(
        "CodeBuddy OAuth cancel 命令触发: login_id={}",
        login_id.as_deref().unwrap_or("<none>")
    ));
    let result = codebuddy_oauth::cancel_login(login_id.as_deref());
    close_codebuddy_oauth_webview(&app);
    result
}

#[tauri::command]
pub fn open_codebuddy_oauth_webview(
    app: AppHandle,
    auth_url: String,
    incognito: Option<bool>,
    authorize_only: Option<bool>,
    ui_texts: Option<CodebuddyOauthWebviewUiTexts>,
) -> Result<(), String> {
    let use_incognito = incognito.unwrap_or(false);
    let use_authorize_only = authorize_only.unwrap_or(false);
    let ui_texts = resolve_codebuddy_oauth_webview_ui_texts(ui_texts)?;
    let debug_trace_id = format!(
        "oauth-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    );
    logger::log_info(&format!(
        "[CodeBuddy OAuth] 打开 WebView: incognito={}, authorize_only={}, trace_id={}",
        use_incognito, use_authorize_only, debug_trace_id
    ));
    close_codebuddy_oauth_webview(&app);

    let auth_url_for_nav = auth_url.clone();
    let target_auth_url = auth_url
        .parse::<url::Url>()
        .map_err(|e| format!("OAuth URL parse error: {}", e))?;
    let auth_state = target_auth_url
        .query_pairs()
        .find(|(k, _)| k == "state")
        .map(|(_, v)| v.to_string());
    let _ = codebuddy_oauth::set_wait_pre_auth_snapshot_by_state(
        auth_state.as_deref(),
        !use_authorize_only,
    );

    let url = tauri::WebviewUrl::External(target_auth_url.clone());

    let oauth_stage_ref = Arc::new(Mutex::new(String::new()));
    let oauth_stage_nav = oauth_stage_ref.clone();
    let app_nav = app.clone();
    let debug_trace_id_for_nav = debug_trace_id.clone();

    tauri::WebviewWindowBuilder::new(&app, CB_OAUTH_WEBVIEW_LABEL, url)
        .title("CodeBuddy OAuth")
        .inner_size(980.0, 760.0)
        .center()
        .incognito(use_incognito)
        .initialization_script(&build_webview_oauth_script(
            use_authorize_only,
            &auth_url_for_nav,
            None,
            &ui_texts.manual_url_placeholder,
            &ui_texts.manual_url_go,
            &ui_texts.manual_url_invalid,
            &ui_texts.quota_failure_prompt,
            &ui_texts.quota_failure_title,
            &ui_texts.quota_failure_retry_label,
            &ui_texts.quota_failure_skip_label,
            &ui_texts.oauth_success_close_prompt,
            &ui_texts.oauth_success_close_title,
            &ui_texts.oauth_success_close_now_label,
            &ui_texts.oauth_success_close_later_label,
            &ui_texts.oauth_success_close_now_status,
            &ui_texts.oauth_success_close_later_status,
            &ui_texts.oauth_step_quota_authorize,
            &ui_texts.oauth_step_quota_bind,
            &ui_texts.oauth_step_quota_complete,
            &ui_texts.oauth_step_prepare,
            &ui_texts.oauth_step_authorize,
            &ui_texts.oauth_step_bind,
            &ui_texts.oauth_step_quota,
            &ui_texts.oauth_step_complete,
            &ui_texts.oauth_status_login_confirm,
            &debug_trace_id,
            false,
        ))
        .on_navigation(move |nav_url| {
            let stage = resolve_oauth_webview_stage(nav_url);
            emit_stage_log_once(&oauth_stage_nav, "CodeBuddy OAuth WebView", stage, nav_url);
            let _ = app_nav.emit(
                "codebuddy-oauth-webview-stage",
                serde_json::json!({ "stage": stage }),
            );

            if nav_url.path().starts_with(CB_OAUTH_ACTION_SIGNAL_PATH) {
                let action = nav_url
                    .query_pairs()
                    .find(|(k, _)| k == "action")
                    .map(|(_, v)| v.to_string())
                    .unwrap_or_default();
                if let Err(err) = dispatch_oauth_webview_action(&app_nav, &action, None) {
                    logger::log_warn(&format!(
                        "[CodeBuddy OAuth] 处理 WebView 动作失败: action={}, error={}",
                        action, err
                    ));
                }
                return false;
            }

            if !use_authorize_only && nav_url.path().starts_with(CB_OAUTH_PREAUTH_SIGNAL_PATH) {
                logger::log_warn(&format!(
                    "[CodeBuddy OAuth Debug] 命中预拉取信号: trace_id={}, nav=({}), return=false",
                    debug_trace_id_for_nav,
                    safe_webview_url_summary(nav_url)
                ));
                let raw_encoded = match nav_url.query_pairs().find(|(k, _)| k == "d") {
                    Some((_, v)) => v.to_string(),
                    None => {
                        logger::log_warn("[CodeBuddy OAuth] 预拉取配额信号缺少 d 参数");
                        let _ = app_nav.emit(
                            "codebuddy-oauth-webview-error",
                            "ERR_CODEBUDDY_OAUTH_PREAUTH_SIGNAL_MISSING_D",
                        );
                        return false;
                    }
                };
                logger::log_warn(&format!(
                    "[CodeBuddy OAuth Debug] 预拉取信号参数长度: trace_id={}, encoded_len={}, query_len={}",
                    debug_trace_id_for_nav,
                    raw_encoded.len(),
                    nav_url.query().map(|q| q.len()).unwrap_or(0)
                ));
                let encoded = enrich_webview_snapshot_with_native_cookie(
                    &app_nav,
                    CB_OAUTH_WEBVIEW_LABEL,
                    raw_encoded,
                    "OAuth",
                );
                logger::log_warn(&format!(
                    "[CodeBuddy OAuth Debug] 注入原生 Cookie 后长度: trace_id={}, encoded_len={}",
                    debug_trace_id_for_nav,
                    encoded.len()
                ));

                if let Err(err) = handle_oauth_webview_snapshot(&app_nav, encoded) {
                    logger::log_warn(&format!(
                        "[CodeBuddy OAuth] 处理预拉取配额结果失败: trace_id={}, error={}",
                        debug_trace_id_for_nav, err
                    ));
                    let _ = app_nav.emit("codebuddy-oauth-webview-error", err);
                } else {
                    logger::log_info(&format!(
                        "[CodeBuddy OAuth] 预拉取配额结果已缓存，等待绑定流程继续: trace_id={}",
                        debug_trace_id_for_nav
                    ));
                }
                return false;
            }
            true
        })
        .build()
        .map_err(|e| format!("ERR_CODEBUDDY_OAUTH_WEBVIEW_OPEN_FAILED:{}", e))?;

    Ok(())
}

#[tauri::command]
pub fn codebuddy_oauth_webview_action(
    app: AppHandle,
    action: String,
    auth_url: Option<String>,
) -> Result<(), String> {
    dispatch_oauth_webview_action(&app, &action, auth_url.as_deref())
}

#[tauri::command]
pub fn codebuddy_oauth_webview_submit_snapshot(
    app: AppHandle,
    payload: String,
) -> Result<(), String> {
    handle_oauth_webview_snapshot(&app, payload)
}

#[tauri::command]
pub fn codebuddy_quota_webview_submit_snapshot(
    app: AppHandle,
    account_id: String,
    payload: String,
) -> Result<(), String> {
    handle_quota_webview_snapshot(&app, account_id, payload)
}

#[tauri::command]
pub async fn add_codebuddy_account_with_token(
    app: AppHandle,
    access_token: String,
) -> Result<CodebuddyAccount, String> {
    let payload = codebuddy_oauth::build_payload_from_token(&access_token).await?;
    let account = codebuddy_account::upsert_account(payload)?;
    let _ = crate::modules::tray::update_tray_menu(&app);
    Ok(account)
}

#[tauri::command]
pub async fn update_codebuddy_account_tags(
    account_id: String,
    tags: Vec<String>,
) -> Result<CodebuddyAccount, String> {
    codebuddy_account::update_account_tags(&account_id, tags)
}

#[tauri::command]
pub fn get_codebuddy_accounts_index_path() -> Result<String, String> {
    codebuddy_account::accounts_index_path_string()
}

#[tauri::command]
pub async fn inject_codebuddy_to_vscode(
    app: AppHandle,
    account_id: String,
) -> Result<String, String> {
    let started_at = Instant::now();
    logger::log_info(&format!(
        "[CodeBuddy Switch] 开始切换账号: account_id={}",
        account_id
    ));

    let account = codebuddy_account::load_account(&account_id)
        .ok_or_else(|| format!("CodeBuddy account not found: {}", account_id))?;

    let state_db_path = codebuddy_account::get_default_codebuddy_state_db_path()
        .ok_or_else(|| "无法获取 CodeBuddy state.vscdb 路径".to_string())?;

    if !state_db_path.exists() {
        return Err(format!(
            "CodeBuddy state.vscdb 不存在: {}",
            state_db_path.display()
        ));
    }

    let session_json = build_session_json(&account);
    let secret_key =
        r#"{"extensionId":"tencent-cloud.coding-copilot","key":"planning-genie.new.accessToken"}"#;
    let db_key = format!("secret://{}", secret_key);

    crate::modules::vscode_inject::inject_secret_to_state_db_for_codebuddy(
        &state_db_path,
        &db_key,
        &session_json,
    )?;

    if let Err(err) = crate::modules::codebuddy_instance::update_default_settings(
        Some(Some(account_id.clone())),
        None,
        Some(false),
    ) {
        logger::log_warn(&format!("更新 CodeBuddy 默认实例绑定账号失败: {}", err));
    }

    let launch_warning = match crate::commands::codebuddy_instance::codebuddy_start_instance(
        "__default__".to_string(),
    )
    .await
    {
        Ok(_) => None,
        Err(err) => {
            if err.starts_with("APP_PATH_NOT_FOUND:") || err.contains("启动 CodeBuddy 失败") {
                logger::log_warn(&format!("CodeBuddy 默认实例启动失败: {}", err));
                if err.starts_with("APP_PATH_NOT_FOUND:") || err.contains("APP_PATH_NOT_FOUND:") {
                    let _ = app.emit(
                        "app:path_missing",
                        serde_json::json!({ "app": "codebuddy", "retry": { "kind": "default" } }),
                    );
                }
                Some(err)
            } else {
                return Err(err);
            }
        }
    };

    let _ = crate::modules::tray::update_tray_menu(&app);

    if let Some(err) = launch_warning {
        logger::log_warn(&format!(
            "[CodeBuddy Switch] 切号完成但启动失败: account_id={}, email={}, elapsed={}ms, error={}",
            account.id,
            account.email,
            started_at.elapsed().as_millis(),
            err
        ));
        Ok(format!("切换完成，但 CodeBuddy 启动失败: {}", err))
    } else {
        logger::log_info(&format!(
            "[CodeBuddy Switch] 切号成功: account_id={}, email={}, elapsed={}ms",
            account.id,
            account.email,
            started_at.elapsed().as_millis()
        ));
        Ok(format!("切换完成: {}", account.email))
    }
}

#[tauri::command]
pub async fn open_codebuddy_quota_webview(
    app: AppHandle,
    account_id: String,
    incognito: Option<bool>,
    ui_texts: Option<CodebuddyOauthWebviewUiTexts>,
) -> Result<(), String> {
    let use_incognito = incognito.unwrap_or(false);
    let ui_texts = resolve_codebuddy_oauth_webview_ui_texts(ui_texts)?;
    let debug_trace_id = format!(
        "quota-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    );
    logger::log_info(&format!(
        "[CodeBuddy WebView Quota] 打开 WebView: account_id={}, incognito={}, trace_id={}",
        account_id, use_incognito, debug_trace_id
    ));

    close_codebuddy_webview(&app, CB_QUOTA_WEBVIEW_LABEL);

    let app_nav = app.clone();
    let aid_nav = account_id.clone();
    let quota_stage_ref = Arc::new(Mutex::new(String::new()));
    let quota_stage_nav = quota_stage_ref.clone();

    let url = tauri::WebviewUrl::External(
        CODEBUDDY_PROFILE_USAGE_URL
            .parse()
            .map_err(|e| format!("URL parse error: {}", e))?,
    );

    tauri::WebviewWindowBuilder::new(&app, CB_QUOTA_WEBVIEW_LABEL, url)
        .title("CodeBuddy")
        .inner_size(1100.0, 750.0)
        .center()
        .incognito(use_incognito)
        .initialization_script(&build_webview_oauth_script(
            false,
            CODEBUDDY_PROFILE_USAGE_URL,
            Some(&account_id),
            &ui_texts.manual_url_placeholder,
            &ui_texts.manual_url_go,
            &ui_texts.manual_url_invalid,
            &ui_texts.quota_failure_prompt,
            &ui_texts.quota_failure_title,
            &ui_texts.quota_failure_retry_label,
            &ui_texts.quota_failure_skip_label,
            &ui_texts.oauth_success_close_prompt,
            &ui_texts.oauth_success_close_title,
            &ui_texts.oauth_success_close_now_label,
            &ui_texts.oauth_success_close_later_label,
            &ui_texts.oauth_success_close_now_status,
            &ui_texts.oauth_success_close_later_status,
            &ui_texts.oauth_step_quota_authorize,
            &ui_texts.oauth_step_quota_bind,
            &ui_texts.oauth_step_quota_complete,
            &ui_texts.oauth_step_prepare,
            &ui_texts.oauth_step_authorize,
            &ui_texts.oauth_step_bind,
            &ui_texts.oauth_step_quota,
            &ui_texts.oauth_step_complete,
            &ui_texts.oauth_status_login_confirm,
            &debug_trace_id,
            true,
        ))
        .on_navigation(move |nav_url| {
            let stage = resolve_quota_webview_stage(nav_url);
            emit_stage_log_once(&quota_stage_nav, "CodeBuddy WebView Quota", stage, nav_url);
            let _ = app_nav.emit(
                "codebuddy-quota-webview-stage",
                serde_json::json!({ "stage": stage }),
            );

            if !nav_url.path().starts_with(CB_WEBVIEW_SIGNAL_PATH) {
                return true;
            }
            logger::log_info(&format!(
                "[CodeBuddy WebView Quota] on_navigation 命中信号 URL, query len={}",
                nav_url.query().unwrap_or("").len()
            ));
            let encoded = match nav_url.query_pairs().find(|(k, _)| k == "d") {
                Some((_, v)) => v.to_string(),
                None => {
                    logger::log_warn("[CodeBuddy WebView Quota] 信号 URL 缺少 d 参数");
                    let _ = app_nav.emit(
                        "codebuddy-webview-quota-error",
                        "ERR_CODEBUDDY_QUOTA_SIGNAL_MISSING_D",
                    );
                    let a = app_nav.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Some(w) = a.get_webview_window(CB_QUOTA_WEBVIEW_LABEL) {
                            let _ = w.close();
                        }
                    });
                    return false;
                }
            };
            let encoded = enrich_webview_snapshot_with_native_cookie(
                &app_nav,
                CB_QUOTA_WEBVIEW_LABEL,
                encoded,
                "Quota",
            );
            if let Err(err) = handle_quota_webview_snapshot(&app_nav, aid_nav.clone(), encoded) {
                logger::log_warn(&format!(
                    "[CodeBuddy WebView Quota] 处理信号 URL 失败: {}",
                    err
                ));
                let _ = app_nav.emit("codebuddy-webview-quota-error", &err);
            }
            false
        })
        .build()
        .map_err(|e| format!("ERR_CODEBUDDY_QUOTA_WEBVIEW_OPEN_FAILED:{}", e))?;

    Ok(())
}

#[allow(dead_code)]
fn build_webview_quota_script() -> String {
    let signal_path = serde_json::to_string(CB_WEBVIEW_SIGNAL_PATH)
        .unwrap_or_else(|_| "\"/__agtools_cb_quota_result__\"".to_string());
    format!(
        r#"(function(){{
  var SIGNAL={};
  var done=false;
  var progress=8;

  /* ── status bar ── */
  var bar=document.createElement('div');
  bar.id='__agtools_status';
  bar.style.cssText='position:fixed;top:0;left:0;right:0;z-index:2147483647;padding:8px 14px 12px;font:600 13px/1.4 -apple-system,BlinkMacSystemFont,Segoe UI,sans-serif;color:#fff;text-align:center;transition:background .25s;background:#4f46e5;box-shadow:0 2px 8px rgba(0,0,0,.25);';
  var text=document.createElement('div');
  text.style.cssText='margin-bottom:6px;';
  var track=document.createElement('div');
  track.style.cssText='height:4px;background:rgba(255,255,255,.25);border-radius:999px;overflow:hidden;';
  var fill=document.createElement('div');
  fill.style.cssText='height:100%;width:8%;background:#fff;border-radius:999px;transition:width .35s ease;';
  track.appendChild(fill);
  bar.appendChild(text);
  bar.appendChild(track);

  function mount(){{ if(document.body){{document.body.appendChild(bar)}}else{{setTimeout(mount,100)}} }}
  mount();
  function status(msg,p,bg){{ progress=Math.max(progress,p); text.textContent=msg; fill.style.width=Math.min(progress,100)+'%'; bar.style.background=bg; }}
  status('\u23f3 \u7b49\u5f85\u767b\u5f55 CodeBuddy\u2026',8,'#4f46e5');

  function hasRequiredSessionCookies(raw){{
    var cookie=String(raw||'');
    return /(?:^|;\s*)session=/.test(cookie) && /(?:^|;\s*)session_2=/.test(cookie);
  }}

  function fmt(d){{ return d.toISOString().replace('T',' ').substring(0,19); }}
  async function run(){{
    if(done) return;
    try{{
      status('\ud83d\udd10 \u6b63\u5728\u68c0\u67e5\u767b\u5f55\u72b6\u6001\u2026',18,'#4f46e5');
      var cookieHeader=String(document.cookie||'').trim();
      if(!cookieHeader){{
        status('\u23f3 \u8bf7\u5148\u5b8c\u6210\u767b\u5f55\uff0c\u7136\u540e\u4f1a\u81ea\u52a8\u7ee7\u7eed\u2026',18,'#4f46e5');
        return;
      }}
      if(!hasRequiredSessionCookies(cookieHeader)){{
        status('\u23f3 \u5df2\u767b\u5f55\uff0c\u4f46\u672a\u62ff\u5230 session/session_2\uff0c\u6b63\u5728\u7b49\u5f85\u9875\u9762\u521d\u59cb\u5316\u2026',22,'#4f46e5');
        return;
      }}
      status('\ud83d\udd0d \u5df2\u767b\u5f55\uff0c\u6b63\u5728\u91c7\u96c6 Cookie\u2026',45,'#0ea5e9');
      var now=new Date();
      var begin=fmt(now);
      var end=fmt(new Date(now.getTime()+100*365.25*86400000));
      var requestPayload={{
        PageNumber:1,PageSize:100,
        ProductCode:'p_tcaca',Status:[0,3],
        PackageEndTimeRangeBegin:begin,
        PackageEndTimeRangeEnd:end
      }};
      var requestReferrer=String(window.location.href||'https://www.codebuddy.ai/profile/usage');
      var requestHeaders={{
        'Accept':'application/json, text/plain, */*',
        'Accept-Language': String(navigator.language||'zh-CN'),
        'Content-Type':'application/json',
        'Origin': String(window.location.origin||'https://www.codebuddy.ai'),
        'Referer': requestReferrer,
        'User-Agent': String(navigator.userAgent||''),
        'Sec-Fetch-Site':'same-origin',
        'Sec-Fetch-Mode':'cors',
        'Sec-Fetch-Dest':'empty'
      }};
      var binding={{
        cookieHeader:cookieHeader,
        productCode:'p_tcaca',
        status:[0,3],
        packageEndTimeRangeBegin:begin,
        packageEndTimeRangeEnd:end,
        pageNumber:1,
        pageSize:100,
        requestUrl:String(window.location.origin||'https://www.codebuddy.ai')+'/billing/meter/get-user-resource',
        requestMethod:'POST',
        requestReferrer:requestReferrer,
        requestHeaders:requestHeaders,
        requestBody:requestPayload
      }};
      done=true;
      status('\u2705 Cookie \u5df2\u91c7\u96c6\uff0c\u6b63\u5728\u4ea4\u7531\u672c\u5730\u540e\u7aef\u67e5\u8be2\u914d\u989d\u2026',95,'#22c55e');
      var payload=JSON.stringify({{binding:binding}});
      setTimeout(function(){{
        status('\u2705 \u53c2\u6570\u5df2\u53d1\u9001\uff0c\u7b49\u5f85\u672c\u5730\u5237\u65b0\u7ed3\u679c\u2026',100,'#22c55e');
        try{{ window.location.href=window.location.origin+SIGNAL+'?d='+encodeURIComponent(payload); }}catch(e){{}}
        setTimeout(function(){{ try{{window.close();}}catch(e){{}} }},1200);
      }},1200);
    }}catch(e){{
      status('\u231b \u91c7\u96c6\u5931\u8d25\uff0c\u6b63\u5728\u81ea\u52a8\u91cd\u8bd5\u2026',24,'#4f46e5');
    }}
  }}
  setTimeout(run,2000);
  var iv=setInterval(function(){{ if(done){{clearInterval(iv);return;}} run(); }},3000);
  setTimeout(function(){{ if(!done){{ clearInterval(iv); status('\u26a0\ufe0f \u8d85\u65f6\uff0c\u8bf7\u5173\u95ed\u7a97\u53e3\u91cd\u8bd5',100,'#ef4444'); }} }},300000);
}})();"#,
        signal_path
    )
}

fn build_webview_oauth_script(
    force_authorize: bool,
    auth_url: &str,
    quota_account_id: Option<&str>,
    manual_url_placeholder: &str,
    manual_url_go: &str,
    manual_url_invalid: &str,
    quota_failure_prompt: &str,
    quota_failure_title: &str,
    quota_failure_retry_label: &str,
    quota_failure_skip_label: &str,
    oauth_success_close_prompt: &str,
    oauth_success_close_title: &str,
    oauth_success_close_now_label: &str,
    oauth_success_close_later_label: &str,
    oauth_success_close_now_status: &str,
    oauth_success_close_later_status: &str,
    oauth_step_quota_authorize: &str,
    oauth_step_quota_bind: &str,
    oauth_step_quota_complete: &str,
    oauth_step_prepare: &str,
    oauth_step_authorize: &str,
    oauth_step_bind: &str,
    oauth_step_quota: &str,
    oauth_step_complete: &str,
    oauth_status_login_confirm: &str,
    debug_trace_id: &str,
    quota_only_mode: bool,
) -> String {
    let signal_path = if quota_only_mode {
        serde_json::to_string(CB_WEBVIEW_SIGNAL_PATH)
            .unwrap_or_else(|_| "\"/__agtools_cb_quota_result__\"".to_string())
    } else {
        serde_json::to_string(CB_OAUTH_PREAUTH_SIGNAL_PATH)
            .unwrap_or_else(|_| "\"/__agtools_cb_oauth_quota_result__\"".to_string())
    };
    let action_signal_path = serde_json::to_string(CB_OAUTH_ACTION_SIGNAL_PATH)
        .unwrap_or_else(|_| "\"/__agtools_cb_oauth_action__\"".to_string());
    let auth_url_literal = serde_json::to_string(auth_url).unwrap_or_else(|_| "\"\"".to_string());
    let quota_account_id_literal = serde_json::to_string(quota_account_id.unwrap_or(""))
        .unwrap_or_else(|_| "\"\"".to_string());
    let force_authorize_literal = if force_authorize { "true" } else { "false" };
    let quota_only_mode_literal = if quota_only_mode { "true" } else { "false" };
    let manual_url_placeholder_literal =
        serde_json::to_string(manual_url_placeholder).unwrap_or_else(|_| "\"\"".to_string());
    let manual_url_go_literal =
        serde_json::to_string(manual_url_go).unwrap_or_else(|_| "\"\"".to_string());
    let manual_url_invalid_literal =
        serde_json::to_string(manual_url_invalid).unwrap_or_else(|_| "\"\"".to_string());
    let quota_failure_prompt_literal =
        serde_json::to_string(quota_failure_prompt).unwrap_or_else(|_| "\"\"".to_string());
    let quota_failure_title_literal =
        serde_json::to_string(quota_failure_title).unwrap_or_else(|_| "\"\"".to_string());
    let quota_failure_retry_label_literal =
        serde_json::to_string(quota_failure_retry_label).unwrap_or_else(|_| "\"\"".to_string());
    let quota_failure_skip_label_literal =
        serde_json::to_string(quota_failure_skip_label).unwrap_or_else(|_| "\"\"".to_string());
    let oauth_success_close_prompt_literal =
        serde_json::to_string(oauth_success_close_prompt).unwrap_or_else(|_| "\"\"".to_string());
    let oauth_success_close_title_literal =
        serde_json::to_string(oauth_success_close_title).unwrap_or_else(|_| "\"\"".to_string());
    let oauth_success_close_now_label_literal =
        serde_json::to_string(oauth_success_close_now_label).unwrap_or_else(|_| "\"\"".to_string());
    let oauth_success_close_later_label_literal =
        serde_json::to_string(oauth_success_close_later_label)
            .unwrap_or_else(|_| "\"\"".to_string());
    let oauth_success_close_now_status_literal =
        serde_json::to_string(oauth_success_close_now_status)
            .unwrap_or_else(|_| "\"\"".to_string());
    let oauth_success_close_later_status_literal =
        serde_json::to_string(oauth_success_close_later_status)
            .unwrap_or_else(|_| "\"\"".to_string());
    let oauth_step_quota_authorize_literal =
        serde_json::to_string(oauth_step_quota_authorize).unwrap_or_else(|_| "\"\"".to_string());
    let oauth_step_quota_bind_literal =
        serde_json::to_string(oauth_step_quota_bind).unwrap_or_else(|_| "\"\"".to_string());
    let oauth_step_quota_complete_literal =
        serde_json::to_string(oauth_step_quota_complete).unwrap_or_else(|_| "\"\"".to_string());
    let oauth_step_prepare_literal =
        serde_json::to_string(oauth_step_prepare).unwrap_or_else(|_| "\"\"".to_string());
    let oauth_step_authorize_literal =
        serde_json::to_string(oauth_step_authorize).unwrap_or_else(|_| "\"\"".to_string());
    let oauth_step_bind_literal =
        serde_json::to_string(oauth_step_bind).unwrap_or_else(|_| "\"\"".to_string());
    let oauth_step_quota_literal =
        serde_json::to_string(oauth_step_quota).unwrap_or_else(|_| "\"\"".to_string());
    let oauth_step_complete_literal =
        serde_json::to_string(oauth_step_complete).unwrap_or_else(|_| "\"\"".to_string());
    let oauth_status_login_confirm_literal =
        serde_json::to_string(oauth_status_login_confirm).unwrap_or_else(|_| "\"\"".to_string());
    let debug_trace_id_literal =
        serde_json::to_string(debug_trace_id).unwrap_or_else(|_| "\"unknown\"".to_string());
    let oauth_usage_url_literal = if quota_only_mode {
        serde_json::to_string(auth_url)
            .unwrap_or_else(|_| format!("\"{}\"", CODEBUDDY_PROFILE_USAGE_URL))
    } else {
        serde_json::to_string(CODEBUDDY_OAUTH_USAGE_URL)
            .unwrap_or_else(|_| "\"https://www.codebuddy.ai/agents?source=ide_login\"".to_string())
    };
    format!(
        r#"(function(){{
  var SIGNAL={};
  var ACTION_SIGNAL={};
  var AUTH_URL={};
  var QUOTA_ACCOUNT_ID={};
  var FORCE_AUTHORIZE={};
  var QUOTA_ONLY_MODE={};
  var MANUAL_URL_PLACEHOLDER={};
  var MANUAL_URL_GO={};
  var MANUAL_URL_INVALID={};
  var QUOTA_FAIL_PROMPT={};
  var QUOTA_FAIL_TITLE={};
  var QUOTA_FAIL_RETRY_LABEL={};
  var QUOTA_FAIL_SKIP_LABEL={};
  var OAUTH_SUCCESS_CLOSE_PROMPT={};
  var OAUTH_SUCCESS_CLOSE_TITLE={};
  var OAUTH_SUCCESS_CLOSE_NOW_LABEL={};
  var OAUTH_SUCCESS_CLOSE_LATER_LABEL={};
  var OAUTH_SUCCESS_CLOSE_NOW_STATUS={};
  var OAUTH_SUCCESS_CLOSE_LATER_STATUS={};
  var OAUTH_STEP_QUOTA_AUTHORIZE={};
  var OAUTH_STEP_QUOTA_BIND={};
  var OAUTH_STEP_QUOTA_COMPLETE={};
  var OAUTH_STEP_PREPARE={};
  var OAUTH_STEP_AUTHORIZE={};
  var OAUTH_STEP_BIND={};
  var OAUTH_STEP_QUOTA={};
  var OAUTH_STEP_COMPLETE={};
  var OAUTH_STATUS_LOGIN_CONFIRM={};
  var OAUTH_USAGE_URL={};
  var DEBUG_TRACE_ID={};
  var PHASE_KEY='__agtools_cb_oauth_phase__';
  var AUTH_BOOTSTRAP_KEY='__agtools_cb_oauth_authurl_bootstrapped__';
  var PANEL_ID='__agtools_oauth_panel';
  var MSG_ID='__agtools_oauth_msg';
  var QUOTA_FAIL_DIALOG_ID='__agtools_oauth_quota_fail_dialog';
  var QUOTA_LOADING_DIALOG_ID='__agtools_oauth_quota_loading_dialog';
  var LOADING_SPINNER_STYLE_ID='__agtools_oauth_loading_spinner_style';
  var SUCCESS_CLOSE_DIALOG_ID='__agtools_oauth_success_close_dialog';
  var done=false;
  var authProbeRunning=false;
  var quotaStarted=false;
  var authInterval=null;
  var authTimeout=null;
  function dbg(tag, extra){{
    try {{
      var payload=extra||{{}};
      payload.href=String(window.location.href||'');
      payload.readyState=String(document.readyState||'');
      try{{ payload.phase=String(sessionStorage.getItem(PHASE_KEY)||''); }}catch(_e){{ payload.phase=''; }}
      console.info('[AGTOOLS_CB_OAUTH_DEBUG]['+DEBUG_TRACE_ID+']['+String(tag||'event')+'] '+JSON.stringify(payload));
    }} catch(_e2) {{}}
  }}
  function invokeHostCommand(cmd, payload){{
    try {{
      if(window.__TAURI_INTERNALS__ && typeof window.__TAURI_INTERNALS__.invoke==='function') {{
        return window.__TAURI_INTERNALS__.invoke(cmd, payload || {{}});
      }}
    }} catch(_e) {{}}
    return Promise.reject(new Error('Tauri invoke unavailable'));
  }}
  function sendActionToHost(action){{
    return invokeHostCommand('codebuddy_oauth_webview_action', {{ action: String(action||'') }});
  }}
  function submitQuotaSnapshotToHost(payloadText){{
    if(QUOTA_ONLY_MODE){{
      if(!QUOTA_ACCOUNT_ID){{
        return Promise.reject(new Error('Missing quota account id'));
      }}
      return invokeHostCommand('codebuddy_quota_webview_submit_snapshot', {{
        accountId: String(QUOTA_ACCOUNT_ID),
        payload: String(payloadText||'')
      }});
    }}
    return invokeHostCommand('codebuddy_oauth_webview_submit_snapshot', {{
      payload: String(payloadText||'')
    }});
  }}
  window.addEventListener('error', function(ev){{
    dbg('window_error', {{
      message:String((ev&&ev.message)||''),
      filename:String((ev&&ev.filename)||''),
      line:Number((ev&&ev.lineno)||0),
      column:Number((ev&&ev.colno)||0)
    }});
  }});
  window.addEventListener('unhandledrejection', function(ev){{
    var reason='';
    try {{
      var r=ev&&ev.reason;
      reason=String((r&&r.stack)||r||'');
    }} catch(_e) {{
      reason='[unreadable]';
    }}
    dbg('unhandled_rejection', {{reason:reason}});
  }});
  var STEPS=QUOTA_ONLY_MODE
    ? [
        {{key:'authorize',label:String(OAUTH_STEP_QUOTA_AUTHORIZE||'')}},
        {{key:'quota',label:String(OAUTH_STEP_QUOTA_BIND||'')}},
        {{key:'complete',label:String(OAUTH_STEP_QUOTA_COMPLETE||'')}}
      ]
    : [
        {{key:'prepare',label:String(OAUTH_STEP_PREPARE||'')}},
        {{key:'authorize',label:String(OAUTH_STEP_AUTHORIZE||'')}},
        {{key:'bind',label:String(OAUTH_STEP_BIND||'')}},
        {{key:'quota',label:String(OAUTH_STEP_QUOTA||'')}},
        {{key:'complete',label:String(OAUTH_STEP_COMPLETE||'')}}
      ];
  var stepState={{prepare:'success',authorize:'pending',quota:'pending',bind:'pending',complete:'pending'}};

  function ensurePanel(){{
    if(document.getElementById(PANEL_ID)) return;
    var panel=document.createElement('div');
    panel.id=PANEL_ID;
    panel.style.cssText='position:fixed;top:0;left:0;right:0;z-index:2147483647;padding:10px 14px 8px;background:#f8fafc;border-bottom:1px solid #dbeafe;box-shadow:0 1px 8px rgba(15,23,42,.08);font-family:-apple-system,BlinkMacSystemFont,Segoe UI,sans-serif;';

    var row=document.createElement('div');
    row.style.cssText='display:flex;gap:8px;overflow-x:auto;white-space:nowrap;padding-bottom:6px;';

    for(var i=0;i<STEPS.length;i++) {{
      var chip=document.createElement('span');
      chip.id='__agtools_oauth_step_'+STEPS[i].key;
      chip.style.cssText='display:inline-flex;align-items:center;gap:6px;padding:6px 10px;border-radius:999px;border:1px solid #cbd5e1;background:#f8fafc;color:#334155;font:600 12px/1;';
      chip.textContent=STEPS[i].label;
      var canRetryStep = QUOTA_ONLY_MODE
        ? (STEPS[i].key==='quota')
        : (STEPS[i].key==='authorize'||STEPS[i].key==='quota'||STEPS[i].key==='bind');
      if(canRetryStep){{
        chip.style.cursor='pointer';
        chip.title='';
        (function(stepKey){{
          chip.addEventListener('click', function(){{ retryStep(stepKey); }});
        }})(STEPS[i].key);
      }}
      row.appendChild(chip);
    }}

    var msg=document.createElement('div');
    msg.id=MSG_ID;
    msg.style.cssText='font-size:12px;line-height:1.45;color:#0f172a;padding:0 2px 2px;';
    msg.textContent=String(OAUTH_STEP_PREPARE||'');

    var navRow=document.createElement('div');
    navRow.style.cssText='display:flex;gap:6px;padding:2px 2px 0;';
    var navInput=document.createElement('input');
    navInput.id='__agtools_oauth_manual_url';
    navInput.type='text';
    navInput.spellcheck=false;
    navInput.autocomplete='off';
    navInput.autocapitalize='off';
    navInput.placeholder=MANUAL_URL_PLACEHOLDER||'';
    navInput.value=String(window.location.href||AUTH_URL||'');
    navInput.style.cssText='flex:1;min-width:0;height:30px;padding:0 10px;border:1px solid #cbd5e1;border-radius:8px;font-size:12px;color:#0f172a;background:#fff;';
    navInput.addEventListener('keydown', function(ev){{
      if((ev.key||'').toLowerCase()!=='enter') return;
      ev.preventDefault();
      navigateManualUrlFromInput();
    }});

    var navBtn=document.createElement('button');
    navBtn.type='button';
    navBtn.textContent=MANUAL_URL_GO||'';
    navBtn.style.cssText='height:30px;padding:0 12px;border:1px solid #93c5fd;border-radius:8px;background:#eff6ff;color:#1d4ed8;font:600 12px/1;cursor:pointer;';
    navBtn.addEventListener('click', function(){{ navigateManualUrlFromInput(); }});
    navRow.appendChild(navInput);
    navRow.appendChild(navBtn);

    panel.appendChild(row);
    panel.appendChild(msg);
    if(!QUOTA_ONLY_MODE) panel.appendChild(navRow);

    function mount(){{
      if(document.body) {{
        document.body.appendChild(panel);
        syncPanelOffset();
        dbg('panel_mounted', {{panelId:PANEL_ID}});
      }} else {{
        setTimeout(mount,100);
      }}
    }}
    mount();
    window.addEventListener('resize', syncPanelOffset);
  }}

  function syncPanelOffset(){{
    var panel=document.getElementById(PANEL_ID);
    var body=document.body||document.documentElement;
    if(!panel||!body) return;
    if(body.dataset.agtoolsOauthBasePadding===undefined){{
      var current=window.getComputedStyle(body).paddingTop||'0';
      body.dataset.agtoolsOauthBasePadding=String(parseFloat(current)||0);
    }}
    var base=parseFloat(body.dataset.agtoolsOauthBasePadding||'0')||0;
    var extra=Math.ceil(panel.getBoundingClientRect().height)+8;
    body.style.paddingTop=(base+extra)+'px';
  }}

  function paintStep(key){{
    var chip=document.getElementById('__agtools_oauth_step_'+key);
    if(!chip) return;
    var state=stepState[key]||'pending';
    if(state==='success'){{
      chip.style.borderColor='#86efac';
      chip.style.background='#f0fdf4';
      chip.style.color='#166534';
      return;
    }}
    if(state==='running'){{
      chip.style.borderColor='#93c5fd';
      chip.style.background='#eff6ff';
      chip.style.color='#1d4ed8';
      return;
    }}
    if(state==='error'){{
      chip.style.borderColor='#fca5a5';
      chip.style.background='#fef2f2';
      chip.style.color='#b91c1c';
      return;
    }}
    chip.style.borderColor='#cbd5e1';
    chip.style.background='#f8fafc';
    chip.style.color='#334155';
  }}

  function setStep(key,state){{
    stepState[key]=state;
    paintStep(key);
  }}

  function setSteps(prepare,authorize,bind,quota,complete){{
    setStep('prepare',prepare);
    setStep('authorize',authorize);
    setStep('bind',bind);
    setStep('quota',quota);
    setStep('complete',complete);
  }}

  function setStatus(msg,color){{
    var node=document.getElementById(MSG_ID);
    if(!node) return;
    node.textContent=msg;
    if(color) node.style.color=color;
  }}

  function navigateManualUrlFromInput(){{
    var input=document.getElementById('__agtools_oauth_manual_url');
    if(!input) return;
    var raw=String(input.value||'').trim();
    if(!raw){{
      setStatus(MANUAL_URL_INVALID||'', '#b91c1c');
      return;
    }}
    var nextUrl='';
    try {{
      nextUrl=new URL(raw).toString();
    }} catch(_e) {{
      try {{
        nextUrl=new URL(raw, window.location.origin).toString();
      }} catch(_e2) {{
        setStatus(MANUAL_URL_INVALID||'', '#b91c1c');
        return;
      }}
    }}
    if(nextUrl===String(window.location.href||'')) return;
    try {{
      window.location.href=nextUrl;
    }} catch(_e3) {{
      setStatus(MANUAL_URL_INVALID||'', '#b91c1c');
    }}
  }}

  function triggerQuotaRetryCapture(){{
    sessionStorage.setItem(PHASE_KEY,'quota');
    quotaStarted=true;
    done=false;
    setSteps('success','success','success','running','pending');
    setStatus(String(OAUTH_STEP_QUOTA||''), '#0f172a');
    try{{ window.location.href=OAUTH_USAGE_URL; }}catch(_e){{}}
    setTimeout(function(){{ runQuotaFetch(); }},300);
  }}

  function retryStep(stepKey){{
    if(stepKey==='authorize'){{
      sessionStorage.setItem(PHASE_KEY,'authorize');
      quotaStarted=false;
      done=false;
      setSteps('success','running','pending','pending','pending');
      setStatus(String(OAUTH_STEP_AUTHORIZE||''), '#0f172a');
      if(AUTH_URL) {{
        window.location.href=AUTH_URL;
      }}
      return;
    }}
    if(stepKey==='quota'){{
      triggerQuotaRetryCapture();
      return;
    }}
    if(stepKey==='bind'){{
      setSteps('success','success','running','pending','pending');
      setStatus(String(OAUTH_STEP_BIND||''), '#0f172a');
      sendActionToHost('retry_bind').catch(function(err){{
        var errText='';
        try{{ errText=String((err&&err.stack)||err||''); }}catch(_e){{ errText='[unreadable]'; }}
        dbg('retry_bind_error', {{error:errText}});
        setStep('bind','error');
        setStatus(String(OAUTH_STEP_BIND||''), '#b91c1c');
      }});
    }}
  }}

  window.__agtools_cb_oauth_retry_quota_capture=function(){{
    triggerQuotaRetryCapture();
  }};

  function removeQuotaFailureDialog(){{
    var node=document.getElementById(QUOTA_FAIL_DIALOG_ID);
    if(!node) return;
    try{{ node.remove(); }}catch(_e){{}}
  }}

  function removeSuccessCloseDialog(){{
    var node=document.getElementById(SUCCESS_CLOSE_DIALOG_ID);
    if(!node) return;
    try{{ node.remove(); }}catch(_e){{}}
  }}

  function ensureLoadingSpinnerStyle(){{
    if(document.getElementById(LOADING_SPINNER_STYLE_ID)) return;
    var style=document.createElement('style');
    style.id=LOADING_SPINNER_STYLE_ID;
    style.textContent='@keyframes agtoolsOauthLoadingSpin{{from{{transform:rotate(0deg)}}to{{transform:rotate(360deg)}}}}';
    (document.head||document.documentElement).appendChild(style);
  }}

  function removeQuotaLoadingDialog(){{
    var node=document.getElementById(QUOTA_LOADING_DIALOG_ID);
    if(!node) return;
    try{{ node.remove(); }}catch(_e){{}}
  }}

  function showQuotaLoadingDialog(){{
    removeQuotaFailureDialog();
    removeSuccessCloseDialog();
    removeQuotaLoadingDialog();
    ensureLoadingSpinnerStyle();

    var root=document.createElement('div');
    root.id=QUOTA_LOADING_DIALOG_ID;
    root.style.cssText='position:fixed;inset:0;z-index:2147483647;background:rgba(15,23,42,.45);display:flex;align-items:center;justify-content:center;padding:20px;';

    var card=document.createElement('div');
    card.style.cssText='width:min(520px,96vw);background:#fff;border-radius:12px;border:1px solid #e2e8f0;box-shadow:0 12px 32px rgba(15,23,42,.24);padding:16px;';

    var head=document.createElement('div');
    head.style.cssText='display:flex;align-items:center;gap:10px;';

    var spinner=document.createElement('span');
    spinner.style.cssText='display:inline-block;width:18px;height:18px;border-radius:999px;border:2px solid #bfdbfe;border-top-color:#2563eb;animation:agtoolsOauthLoadingSpin 1s linear infinite;';

    var title=document.createElement('div');
    title.style.cssText='font:700 16px/1.35 -apple-system,BlinkMacSystemFont,Segoe UI,sans-serif;color:#0f172a;';
    title.textContent=String(OAUTH_STEP_QUOTA||OAUTH_STEP_QUOTA_BIND||'');

    var content=document.createElement('div');
    content.style.cssText='font:500 13px/1.6 -apple-system,BlinkMacSystemFont,Segoe UI,sans-serif;color:#334155;white-space:pre-wrap;margin-top:10px;';
    content.textContent=String(OAUTH_STEP_QUOTA_BIND||OAUTH_STEP_QUOTA||'');

    head.appendChild(spinner);
    head.appendChild(title);
    card.appendChild(head);
    card.appendChild(content);
    root.appendChild(card);
    (document.body||document.documentElement).appendChild(root);
  }}

  function sendQuotaFailureDecision(action){{
    removeQuotaLoadingDialog();
    removeQuotaFailureDialog();
    sendActionToHost(action).catch(function(err){{
      var errText='';
      try{{ errText=String((err&&err.stack)||err||''); }}catch(_e){{ errText='[unreadable]'; }}
      dbg('quota_failure_decision_error', {{ action:String(action||''), error:errText }});
      setStatus(String(QUOTA_FAIL_TITLE||''), '#b91c1c');
    }});
  }}

  window.__agtools_cb_oauth_prompt_quota_failure=function(reason){{
    removeQuotaLoadingDialog();
    removeQuotaFailureDialog();
    var detail=String(reason||'').trim();
    var promptText=String(QUOTA_FAIL_PROMPT||'');
    if(detail){{
      promptText += '\n\n' + detail;
    }}
    var root=document.createElement('div');
    root.id=QUOTA_FAIL_DIALOG_ID;
    root.style.cssText='position:fixed;inset:0;z-index:2147483647;background:rgba(15,23,42,.45);display:flex;align-items:center;justify-content:center;padding:20px;';

    var card=document.createElement('div');
    card.style.cssText='width:min(520px,96vw);background:#fff;border-radius:12px;border:1px solid #e2e8f0;box-shadow:0 12px 32px rgba(15,23,42,.24);padding:16px;';

    var title=document.createElement('div');
    title.style.cssText='font:700 16px/1.35 -apple-system,BlinkMacSystemFont,Segoe UI,sans-serif;color:#0f172a;margin-bottom:10px;';
    title.textContent=String(QUOTA_FAIL_TITLE||'');

    var content=document.createElement('div');
    content.style.cssText='font:500 13px/1.6 -apple-system,BlinkMacSystemFont,Segoe UI,sans-serif;color:#334155;white-space:pre-wrap;';
    content.textContent=promptText;

    var actions=document.createElement('div');
    actions.style.cssText='display:flex;justify-content:flex-end;gap:10px;margin-top:14px;';

    var skipBtn=document.createElement('button');
    skipBtn.type='button';
    skipBtn.style.cssText='height:34px;padding:0 14px;border-radius:8px;border:1px solid #cbd5e1;background:#fff;color:#334155;font:600 13px/1 -apple-system,BlinkMacSystemFont,Segoe UI,sans-serif;cursor:pointer;';
    skipBtn.textContent=String(QUOTA_FAIL_SKIP_LABEL||'');
    skipBtn.addEventListener('click', function(){{ sendQuotaFailureDecision('quota_skip'); }});

    var retryBtn=document.createElement('button');
    retryBtn.type='button';
    retryBtn.style.cssText='height:34px;padding:0 14px;border-radius:8px;border:1px solid #2563eb;background:#2563eb;color:#fff;font:600 13px/1 -apple-system,BlinkMacSystemFont,Segoe UI,sans-serif;cursor:pointer;';
    retryBtn.textContent=String(QUOTA_FAIL_RETRY_LABEL||'');
    retryBtn.addEventListener('click', function(){{ sendQuotaFailureDecision('quota_retry'); }});

    actions.appendChild(skipBtn);
    actions.appendChild(retryBtn);
    card.appendChild(title);
    card.appendChild(content);
    card.appendChild(actions);
    root.appendChild(card);
    (document.body||document.documentElement).appendChild(root);
  }};

  function promptSuccessCloseDialog(){{
    removeQuotaLoadingDialog();
    removeSuccessCloseDialog();
    return new Promise(function(resolve){{
      var root=document.createElement('div');
      root.id=SUCCESS_CLOSE_DIALOG_ID;
      root.style.cssText='position:fixed;inset:0;z-index:2147483647;background:rgba(15,23,42,.45);display:flex;align-items:center;justify-content:center;padding:20px;';

      var card=document.createElement('div');
      card.style.cssText='width:min(520px,96vw);background:#fff;border-radius:12px;border:1px solid #e2e8f0;box-shadow:0 12px 32px rgba(15,23,42,.24);padding:16px;';

      var title=document.createElement('div');
      title.style.cssText='font:700 16px/1.35 -apple-system,BlinkMacSystemFont,Segoe UI,sans-serif;color:#0f172a;margin-bottom:10px;';
      title.textContent=String(OAUTH_SUCCESS_CLOSE_TITLE||'');

      var content=document.createElement('div');
      content.style.cssText='font:500 13px/1.6 -apple-system,BlinkMacSystemFont,Segoe UI,sans-serif;color:#334155;white-space:pre-wrap;';
      content.textContent=String(OAUTH_SUCCESS_CLOSE_PROMPT||'');

      var actions=document.createElement('div');
      actions.style.cssText='display:flex;justify-content:flex-end;gap:10px;margin-top:14px;';

      var laterBtn=document.createElement('button');
      laterBtn.type='button';
      laterBtn.style.cssText='height:34px;padding:0 14px;border-radius:8px;border:1px solid #cbd5e1;background:#fff;color:#334155;font:600 13px/1 -apple-system,BlinkMacSystemFont,Segoe UI,sans-serif;cursor:pointer;';
      laterBtn.textContent=String(OAUTH_SUCCESS_CLOSE_LATER_LABEL||'');
      laterBtn.addEventListener('click', function(){{
        removeSuccessCloseDialog();
        resolve(false);
      }});

      var closeBtn=document.createElement('button');
      closeBtn.type='button';
      closeBtn.style.cssText='height:34px;padding:0 14px;border-radius:8px;border:1px solid #2563eb;background:#2563eb;color:#fff;font:600 13px/1 -apple-system,BlinkMacSystemFont,Segoe UI,sans-serif;cursor:pointer;';
      closeBtn.textContent=String(OAUTH_SUCCESS_CLOSE_NOW_LABEL||'');
      closeBtn.addEventListener('click', function(){{
        removeSuccessCloseDialog();
        resolve(true);
      }});

      actions.appendChild(laterBtn);
      actions.appendChild(closeBtn);
      card.appendChild(title);
      card.appendChild(content);
      card.appendChild(actions);
      root.appendChild(card);
      (document.body||document.documentElement).appendChild(root);
    }});
  }}

  function fmt(d){{ return d.toISOString().replace('T',' ').substring(0,19); }}

  function ensureAuthorizeBootstrap(){{
    if(!AUTH_URL) return;
    var currentHost=String(window.location.hostname||'').toLowerCase();
    if(currentHost!=='www.codebuddy.ai'&&currentHost!=='codebuddy.ai') return;
    var bootstrapped=false;
    try{{ bootstrapped=sessionStorage.getItem(AUTH_BOOTSTRAP_KEY)==='1'; }}catch(_e){{}}
    if(bootstrapped) return;
    try{{ sessionStorage.setItem(AUTH_BOOTSTRAP_KEY,'1'); }}catch(_e){{}}
    if(String(window.location.href||'').indexOf(AUTH_URL)===0) return;
    try{{ window.location.href=AUTH_URL; }}catch(_e){{}}
  }}

  function isAuthorizeEntryUrl(){{
    try {{
      var href=String(window.location.href||'');
      if(!href) return false;
      var url=new URL(href);
      var path=String(url.pathname||'').toLowerCase();
      if(path.indexOf('/login')<0) return false;
      var hasPlatformIde=String(url.searchParams.get('platform')||'').toLowerCase()==='ide';
      var hasState=String(url.searchParams.get('state')||'').trim().length>0;
      return hasPlatformIde&&hasState;
    }} catch(_e) {{
      return false;
    }}
  }}

  function buildQuotaBody(){{
    var now=new Date();
    var begin=fmt(now);
    var end=fmt(new Date(now.getTime()+100*365.25*86400000));
    var requestReferrer=String(window.location.href||OAUTH_USAGE_URL);
    var requestHeaders={{
      'Accept':'application/json, text/plain, */*',
      'Accept-Language': String(navigator.language||'zh-CN'),
      'Content-Type':'application/json',
      'Origin': String(window.location.origin||'https://www.codebuddy.ai'),
      'Referer': requestReferrer,
      'User-Agent': String(navigator.userAgent||''),
      'Sec-Fetch-Site':'same-origin',
      'Sec-Fetch-Mode':'cors',
      'Sec-Fetch-Dest':'empty'
    }};
    var payload={{
      PageNumber:1,PageSize:100,
      ProductCode:'p_tcaca',Status:[0,3],
      PackageEndTimeRangeBegin:begin,
      PackageEndTimeRangeEnd:end
    }};
    return {{
      begin:begin,
      end:end,
      payload:payload,
      requestUrl:String(window.location.origin||'https://www.codebuddy.ai')+'/billing/meter/get-user-resource',
      requestMethod:'POST',
      requestReferrer:requestReferrer,
      requestHeaders:requestHeaders
    }};
  }}

  function inspectAuthorize(){{
    var href=String(window.location.href||'').toLowerCase();
    if(/accounts\.google\.com|\/login|platform=ide|state=|authorize|oauth/.test(href)) {{
      setStatus(String(OAUTH_STATUS_LOGIN_CONFIRM||''), '#0f172a');
      return;
    }}
    if(/no-permission/.test(href)){{
      setStatus(String(OAUTH_STATUS_LOGIN_CONFIRM||''), '#b91c1c');
      return;
    }}
    if(/profile|workspace|dashboard|code=|callback|success/.test(href)) {{
      setStatus(String(OAUTH_STEP_AUTHORIZE||''), '#0f172a');
      return;
    }}
    setStatus(String(OAUTH_STEP_AUTHORIZE||''), '#0f172a');
  }}

  function scheduleQuotaAfterAuthorize(){{
    if(quotaStarted||done) return;
    quotaStarted=true;
    setSteps('success','success','running','pending','pending');
    setStatus(String(OAUTH_STEP_BIND||''), '#0f172a');
    sessionStorage.setItem(PHASE_KEY,'bind');
  }}

  function canStartQuotaByUrl(){{
    try {{
      var href=String(window.location.href||'');
      if(!href) return false;
      var url=new URL(href);
      var host=String(url.hostname||'').toLowerCase();
      if(host!=='www.codebuddy.ai'&&host!=='codebuddy.ai') return false;
      var path=String(url.pathname||'').toLowerCase();
      if(path.indexOf('/no-permission')>=0) return false;
      if(path.indexOf('/started')>=0) return true;
      if(path.indexOf('/agents')>=0) return true;
      if(path.indexOf('/profile/usage')>=0) return true;
      if(path.indexOf('/workspace')>=0) return true;
      if(path.indexOf('/dashboard')>=0) return true;
      return false;
    }} catch(_e) {{
      return false;
    }}
  }}

  async function probeAuthorize(){{
    if(done||quotaStarted||authProbeRunning) return;
    authProbeRunning=true;
    try{{
      setStep('prepare','success');
      setStep('authorize','running');
      inspectAuthorize();
      if(canStartQuotaByUrl()) {{
        scheduleQuotaAfterAuthorize();
      }}
    }}catch(_e){{}}
    finally {{
      authProbeRunning=false;
    }}
  }}

  async function runQuotaFetch(){{
    if(done) return false;
    try{{
      var quotaReq=buildQuotaBody();
      showQuotaLoadingDialog();
      setStep('quota','running');
      setStatus(String(OAUTH_STEP_QUOTA||''), '#0f172a');
      var cookieHeader=String(document.cookie||'').trim();
      if(!cookieHeader){{
        removeQuotaLoadingDialog();
        dbg('quota_cookie_missing', {{cookieLen:0}});
        setStep('quota','error');
        setStatus(String(QUOTA_FAIL_TITLE||''), '#b91c1c');
        return false;
      }}

      var binding={{
        cookieHeader:cookieHeader,
        productCode:'p_tcaca',
        status:[0,3],
        packageEndTimeRangeBegin:quotaReq.begin,
        packageEndTimeRangeEnd:quotaReq.end,
        pageNumber:1,
        pageSize:100,
        requestUrl:quotaReq.requestUrl,
        requestMethod:quotaReq.requestMethod,
        requestReferrer:quotaReq.requestReferrer,
        requestHeaders:quotaReq.requestHeaders,
        requestBody:quotaReq.payload
      }};
      var payload=JSON.stringify({{binding:binding}});
      dbg('quota_signal_prepare', {{
        cookieLen:cookieHeader.length,
        payloadLen:payload.length,
        encodedLen:encodeURIComponent(payload).length,
        signalPath:String(SIGNAL||'')
      }});

      done=true;
      sessionStorage.setItem(PHASE_KEY,'quota');
      setSteps('success','success','success','running','pending');
      setStatus(String(OAUTH_STEP_QUOTA_COMPLETE||''), '#0f172a');
      await submitQuotaSnapshotToHost(payload);
      dbg('quota_snapshot_submitted', {{
        quotaOnly:Boolean(QUOTA_ONLY_MODE),
        payloadLen:payload.length
      }});
      if(QUOTA_ONLY_MODE){{
        setStatus(String(OAUTH_STEP_QUOTA_COMPLETE||''), '#15803d');
      }} else {{
        setStatus(String(OAUTH_STEP_QUOTA_COMPLETE||''), '#0f172a');
      }}
      return true;
    }}catch(_e){{
      removeQuotaLoadingDialog();
      var errText='';
      try{{ errText=String((_e&&_e.stack)||_e||''); }}catch(_e2){{ errText='[unreadable]'; }}
      dbg('runQuotaFetch_exception', {{error:errText}});
      done=false;
      setStep('quota','error');
      setStatus(String(QUOTA_FAIL_TITLE||''), '#b91c1c');
      return false;
    }}
  }}

  window.__agtools_cb_oauth_mark_account_bound=function(){{
    dbg('mark_account_bound_called', {{done:Boolean(done), quotaStarted:Boolean(quotaStarted)}});
    if(done) return;
    setSteps('success','success','success','running','pending');
    setStatus(String(OAUTH_STEP_QUOTA||''), '#0f172a');
    sessionStorage.setItem(PHASE_KEY,'quota');
    try{{ window.location.href=OAUTH_USAGE_URL; }}catch(_e){{}}
    setTimeout(function(){{ runQuotaFetch(); }},300);
  }};

  function requestCloseOauthWebview(){{
    sendActionToHost('close_webview').catch(function(){{
      try{{ window.close(); }}catch(_e2){{}}
    }});
  }}

  window.__agtools_cb_oauth_mark_success=function(){{
    dbg('mark_success_called', {{
      hasPanel:!!document.getElementById(PANEL_ID),
      hasMsg:!!document.getElementById(MSG_ID)
    }});
    removeQuotaLoadingDialog();
    done=true;
    if(authInterval) clearInterval(authInterval);
    if(authTimeout) clearTimeout(authTimeout);
    setSteps('success','success','success','success','success');
    promptSuccessCloseDialog().then(function(shouldClose){{
      if(shouldClose){{
        setStatus(String(OAUTH_SUCCESS_CLOSE_NOW_STATUS||''), '#15803d');
        setTimeout(function(){{ requestCloseOauthWebview(); }},20);
        return;
      }}
      setStatus(String(OAUTH_SUCCESS_CLOSE_LATER_STATUS||''), '#15803d');
      var panel=document.getElementById(PANEL_ID);
      if(panel) panel.style.borderBottomColor='#86efac';
    }}).catch(function(){{
      setStatus(String(OAUTH_SUCCESS_CLOSE_LATER_STATUS||''), '#15803d');
      var panel=document.getElementById(PANEL_ID);
      if(panel) panel.style.borderBottomColor='#86efac';
    }});
  }};

  ensurePanel();
  dbg('script_initialized', {{quotaOnly:Boolean(QUOTA_ONLY_MODE), forceAuthorize:Boolean(FORCE_AUTHORIZE)}});
  var phase=sessionStorage.getItem(PHASE_KEY);
  if(FORCE_AUTHORIZE){{
    phase='authorize';
    sessionStorage.setItem(PHASE_KEY,'authorize');
    try{{ sessionStorage.removeItem(AUTH_BOOTSTRAP_KEY); }}catch(_e){{}}
  }}
  if(isAuthorizeEntryUrl()){{
    phase='authorize';
    done=false;
    quotaStarted=false;
    authProbeRunning=false;
    try{{ sessionStorage.setItem(PHASE_KEY,'authorize'); }}catch(_e){{}}
  }}
  if(QUOTA_ONLY_MODE){{
    setStep('prepare','success');
    setStep('bind','success');
    setStep('complete','pending');
    if(canStartQuotaByUrl()){{
      quotaStarted=true;
      setStep('authorize','success');
      setStep('quota','running');
      setStatus(String(OAUTH_STEP_QUOTA||''), '#0f172a');
      runQuotaFetch();
      return;
    }}
    setStep('authorize','running');
    setStep('quota','pending');
    setStatus(String(OAUTH_STATUS_LOGIN_CONFIRM||''), '#0f172a');
    var quotaOnlyInterval=setInterval(function(){{
      if(done){{ clearInterval(quotaOnlyInterval); return; }}
      if(!canStartQuotaByUrl()) return;
      clearInterval(quotaOnlyInterval);
      quotaStarted=true;
      setStep('authorize','success');
      setStep('quota','running');
      setStatus(String(OAUTH_STEP_QUOTA||''), '#0f172a');
      runQuotaFetch();
    }},2000);
    setTimeout(function(){{
      if(done||quotaStarted) return;
      clearInterval(quotaOnlyInterval);
      setStep('authorize','error');
      setStatus(String(QUOTA_FAIL_TITLE||''), '#b91c1c');
    }},600000);
    return;
  }}
  if(phase==='quota'){{
    quotaStarted=true;
    setSteps('success','success','success','running','pending');
    setStatus(String(OAUTH_STEP_QUOTA||''), '#0f172a');
    runQuotaFetch();
    return;
  }}
  if(phase==='bind'){{
    setSteps('success','success','running','pending','pending');
    setStatus(String(OAUTH_STEP_BIND||''), '#0f172a');
    return;
  }}

  setSteps('success','running','pending','pending','pending');
  setStatus(String(OAUTH_STATUS_LOGIN_CONFIRM||''), '#0f172a');
  ensureAuthorizeBootstrap();
  probeAuthorize();
  authInterval=setInterval(function(){{ probeAuthorize(); }},3000);
  authTimeout=setTimeout(function(){{
    if(done||quotaStarted) return;
    if(authInterval) clearInterval(authInterval);
    setStep('authorize','error');
    setStatus(String(QUOTA_FAIL_TITLE||''), '#b91c1c');
  }},600000);

  setTimeout(function(){{
    if(done) return;
    if(!quotaStarted && stepState.authorize!=='success'){{
      inspectAuthorize();
    }}
  }},1200);
}})();"#,
        signal_path,
        action_signal_path,
        auth_url_literal,
        quota_account_id_literal,
        force_authorize_literal,
        quota_only_mode_literal,
        manual_url_placeholder_literal,
        manual_url_go_literal,
        manual_url_invalid_literal,
        quota_failure_prompt_literal,
        quota_failure_title_literal,
        quota_failure_retry_label_literal,
        quota_failure_skip_label_literal,
        oauth_success_close_prompt_literal,
        oauth_success_close_title_literal,
        oauth_success_close_now_label_literal,
        oauth_success_close_later_label_literal,
        oauth_success_close_now_status_literal,
        oauth_success_close_later_status_literal,
        oauth_step_quota_authorize_literal,
        oauth_step_quota_bind_literal,
        oauth_step_quota_complete_literal,
        oauth_step_prepare_literal,
        oauth_step_authorize_literal,
        oauth_step_bind_literal,
        oauth_step_quota_literal,
        oauth_step_complete_literal,
        oauth_status_login_confirm_literal,
        oauth_usage_url_literal,
        debug_trace_id_literal
    )
}

fn build_session_json(account: &CodebuddyAccount) -> String {
    let uid = account.uid.as_deref().unwrap_or("");
    let nickname = account.nickname.as_deref().unwrap_or("");
    let enterprise_id = account.enterprise_id.as_deref().unwrap_or("");
    let enterprise_name = account.enterprise_name.as_deref().unwrap_or("");
    let domain = account.domain.as_deref().unwrap_or("");
    let refresh_token = account.refresh_token.as_deref().unwrap_or("");
    let expires_at = account.expires_at.unwrap_or(0);

    let session = serde_json::json!({
        "id": "Tencent-Cloud.genie-ide",
        "token": account.access_token,
        "refreshToken": refresh_token,
        "expiresAt": expires_at,
        "domain": domain,
        "accessToken": format!("{}+{}", uid, account.access_token),
        "converted": true,
        "account": {
            "id": uid,
            "uid": uid,
            "label": nickname,
            "nickname": nickname,
            "enterpriseId": enterprise_id,
            "enterpriseName": enterprise_name,
            "pluginEnabled": true,
            "lastLogin": true,
        },
        "auth": {
            "accessToken": account.access_token,
            "refreshToken": refresh_token,
            "tokenType": account.token_type.as_deref().unwrap_or("Bearer"),
            "domain": domain,
            "expiresAt": expires_at,
            "expiresIn": expires_at,
            "refreshExpiresIn": 0,
            "refreshExpiresAt": 0,
            "lastRefreshTime": chrono::Utc::now().timestamp_millis(),
        }
    });

    session.to_string()
}
