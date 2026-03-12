use std::collections::HashMap;

use tauri::AppHandle;

use crate::auth::current_auth_account_id;
use crate::auth::extract_auth;
use crate::auth::normalize_imported_auth_json;
use crate::auth::read_current_codex_auth;
use crate::auth::read_current_codex_auth_optional;
use crate::auth::refresh_chatgpt_auth_tokens;
use crate::models::AccountSummary;
use crate::models::AccountsStore;
use crate::models::AuthJsonImportInput;
use crate::models::ImportAccountFailure;
use crate::models::ImportAccountsResult;
use crate::models::StoredAccount;
use crate::models::UsageSnapshot;
use crate::state::AppState;
use crate::store::load_store;
use crate::store::save_store;
use crate::usage::fetch_usage_snapshot;
use crate::utils::now_unix_seconds;
use crate::utils::short_account;

const DEACTIVATED_WORKSPACE_NOTICE: &str = "该账号已被踢出 team 组织，请重新授权后再刷新。";
const DEACTIVATED_ACCOUNT_NOTICE: &str = "账号被封禁，请检查邮箱";
const AUTH_EXPIRED_NOTICE: &str = "授权过期，请重新登录授权。";

struct PreparedImport {
    auth_json: serde_json::Value,
    account_id: String,
    email: Option<String>,
    plan_type: Option<String>,
    usage: Option<UsageSnapshot>,
    label: Option<String>,
}

pub(crate) async fn list_accounts_internal(
    app: &AppHandle,
    state: &AppState,
) -> Result<Vec<AccountSummary>, String> {
    let _guard = state.store_lock.lock().await;
    let store = load_store(app)?;
    let current_account_id = current_auth_account_id();
    Ok(store
        .accounts
        .iter()
        .map(|account| account.to_summary(current_account_id.as_deref()))
        .collect())
}

pub(crate) async fn import_current_auth_account_internal(
    app: &AppHandle,
    state: &AppState,
    label: Option<String>,
) -> Result<AccountSummary, String> {
    let auth_json = read_current_codex_auth()?;
    let prepared = prepare_auth_json_import(auth_json, label).await?;
    commit_prepared_import(app, state, prepared).await
}

pub(crate) async fn import_auth_json_accounts_internal(
    app: &AppHandle,
    state: &AppState,
    items: Vec<AuthJsonImportInput>,
) -> Result<ImportAccountsResult, String> {
    if items.is_empty() {
        return Err("请至少提供一个 JSON 文件或 JSON 文本".to_string());
    }

    let total_count = items.len();
    let mut prepared_imports = Vec::with_capacity(total_count);
    let mut failures = Vec::new();

    for item in items {
        let source = normalize_import_source(&item.source);
        let auth_json = match parse_auth_json_content(&item.content) {
            Ok(value) => value,
            Err(error) => {
                failures.push(ImportAccountFailure { source, error });
                continue;
            }
        };

        match prepare_auth_json_import(auth_json, item.label).await {
            Ok(prepared) => prepared_imports.push(prepared),
            Err(error) => failures.push(ImportAccountFailure { source, error }),
        }
    }

    if prepared_imports.is_empty() {
        return Ok(ImportAccountsResult {
            total_count,
            imported_count: 0,
            updated_count: 0,
            failures,
        });
    }

    let current_account_id = current_auth_account_id();
    let (imported_count, updated_count) = {
        let mut _guard = state.store_lock.lock().await;
        let mut store = load_store(app)?;
        let mut imported_count = 0usize;
        let mut updated_count = 0usize;

        for prepared in prepared_imports {
            let (_, updated_existing) =
                upsert_prepared_import(&mut store, prepared, current_account_id.as_deref());
            if updated_existing {
                updated_count += 1;
            } else {
                imported_count += 1;
            }
        }

        save_store(app, &store)?;
        (imported_count, updated_count)
    };

    Ok(ImportAccountsResult {
        total_count,
        imported_count,
        updated_count,
        failures,
    })
}

pub(crate) async fn delete_account_internal(
    app: &AppHandle,
    state: &AppState,
    id: &str,
) -> Result<(), String> {
    let mut _guard = state.store_lock.lock().await;
    let mut store = load_store(app)?;
    let original_len = store.accounts.len();
    store.accounts.retain(|account| account.id != id);

    if original_len == store.accounts.len() {
        return Err("未找到要删除的账号".to_string());
    }

    save_store(app, &store)?;
    Ok(())
}

/// 拉取并刷新所有账号用量，返回可直接用于前端/状态栏显示的摘要。
///
/// 为避免“后台刷新覆盖新增账号”的竞态：
/// 1) 先拿快照用于网络请求；
/// 2) 请求完成后重新加载最新 store 并按 account_id 合并写回。
pub(crate) async fn refresh_all_usage_internal(
    app: &AppHandle,
    state: &AppState,
    force_auth_refresh: bool,
) -> Result<Vec<AccountSummary>, String> {
    #[derive(Debug)]
    struct RefreshTarget {
        account_id: String,
        auth_json: serde_json::Value,
        auth_is_current: bool,
    }

    let current_auth_override: Option<(String, serde_json::Value)> =
        read_current_codex_auth_optional()
            .ok()
            .flatten()
            .and_then(|auth_json| {
                extract_auth(&auth_json)
                    .ok()
                    .map(|extracted| (extracted.account_id, auth_json))
            });

    let refresh_targets: Vec<RefreshTarget> = {
        let _guard = state.store_lock.lock().await;
        let store = load_store(app)?;
        store
            .accounts
            .into_iter()
            .map(|account| {
                let (auth_json, auth_is_current) = current_auth_override
                    .as_ref()
                    .and_then(|(account_id, auth_json)| {
                        if account_id == &account.account_id {
                            Some((auth_json.clone(), true))
                        } else {
                            None
                        }
                    })
                    .unwrap_or((account.auth_json, false));

                RefreshTarget {
                    account_id: account.account_id,
                    auth_json,
                    auth_is_current,
                }
            })
            .collect()
    };

    #[derive(Debug)]
    struct RefreshOutcome {
        usage: Option<crate::models::UsageSnapshot>,
        usage_error: Option<String>,
        updated_at: i64,
        auth_plan_type: Option<String>,
        auth_email: Option<String>,
        auth_json: serde_json::Value,
        auth_is_current: bool,
        auth_refreshed: bool,
    }

    let mut outcomes: HashMap<String, RefreshOutcome> = HashMap::new();
    let mut handles = Vec::with_capacity(refresh_targets.len());
    for target in refresh_targets {
        handles.push(tauri::async_runtime::spawn(async move {
            let mut working_auth_json = target.auth_json;
            let mut refresh_error: Option<String> = None;
            let mut auth_refreshed = false;

            if force_auth_refresh {
                match refresh_chatgpt_auth_tokens(&working_auth_json).await {
                    Ok(refreshed) => {
                        working_auth_json = refreshed;
                        auth_refreshed = true;
                    }
                    Err(err) => {
                        refresh_error = Some(err);
                    }
                }
            }

            let mut extracted = extract_auth(&working_auth_json);
            let mut fetch_result = match &extracted {
                Ok(auth) => fetch_usage_snapshot(&auth.access_token, &auth.account_id).await,
                Err(err) => Err(err.clone()),
            };

            if !force_auth_refresh && should_retry_with_token_refresh(&fetch_result) {
                match refresh_chatgpt_auth_tokens(&working_auth_json).await {
                    Ok(refreshed) => {
                        working_auth_json = refreshed;
                        auth_refreshed = true;
                        extracted = extract_auth(&working_auth_json);
                        fetch_result = match &extracted {
                            Ok(auth) => {
                                fetch_usage_snapshot(&auth.access_token, &auth.account_id).await
                            }
                            Err(err) => Err(err.clone()),
                        };
                    }
                    Err(err) => {
                        refresh_error = Some(err);
                    }
                }
            }

            let (auth_plan_type, auth_email) = match &extracted {
                Ok(auth) => (auth.plan_type.clone(), auth.email.clone()),
                Err(_) => (None, None),
            };

            let updated_at = now_unix_seconds();
            let outcome = match fetch_result {
                Ok(snapshot) => RefreshOutcome {
                    usage: Some(snapshot),
                    usage_error: None,
                    updated_at,
                    auth_plan_type,
                    auth_email,
                    auth_json: working_auth_json,
                    auth_is_current: target.auth_is_current,
                    auth_refreshed,
                },
                Err(err) => {
                    let combined_error = if let Some(refresh_err) = refresh_error {
                        format!("{err} | 令牌刷新失败: {refresh_err}")
                    } else {
                        err
                    };
                    let display_error = normalize_usage_error_message(&combined_error);
                    RefreshOutcome {
                        usage: None,
                        usage_error: Some(display_error),
                        updated_at,
                        auth_plan_type,
                        auth_email,
                        auth_json: working_auth_json,
                        auth_is_current: target.auth_is_current,
                        auth_refreshed,
                    }
                }
            };
            (target.account_id, outcome)
        }));
    }

    for handle in handles {
        match handle.await {
            Ok((account_id, outcome)) => {
                outcomes.insert(account_id, outcome);
            }
            Err(err) => {
                log::warn!("并行刷新账号用量任务异常: {err}");
            }
        }
    }

    let store = {
        let _guard = state.store_lock.lock().await;
        let mut latest_store = load_store(app)?;

        for account in &mut latest_store.accounts {
            let Some(outcome) = outcomes.get(&account.account_id) else {
                continue;
            };

            account.updated_at = outcome.updated_at;
            account.auth_json = outcome.auth_json.clone();
            account.email = outcome.auth_email.clone().or(account.email.clone());
            let trusted_auth_plan_type = if outcome.auth_is_current || outcome.auth_refreshed {
                outcome.auth_plan_type.clone()
            } else {
                None
            };
            if let Some(snapshot) = outcome.usage.clone() {
                let mut resolved_snapshot = snapshot;
                let resolved_plan_type = trusted_auth_plan_type
                    .clone()
                    .or(resolved_snapshot.plan_type.clone())
                    .or(account.plan_type.clone());
                resolved_snapshot.plan_type = resolved_plan_type.clone();
                account.plan_type = resolved_plan_type;
                account.usage = Some(resolved_snapshot);
                account.usage_error = None;
            } else if let Some(err) = outcome.usage_error.clone() {
                if trusted_auth_plan_type.is_some() {
                    account.plan_type = trusted_auth_plan_type;
                }
                account.usage_error = Some(err);
            }
        }

        save_store(app, &latest_store)?;
        latest_store
    };

    // 与当前 auth 文件重新对齐，确保 current 标签准确。
    let current_account_id = current_auth_account_id();
    let summaries: Vec<AccountSummary> = store
        .accounts
        .iter()
        .map(|account| account.to_summary(current_account_id.as_deref()))
        .collect();

    Ok(summaries)
}

fn should_retry_with_token_refresh(
    fetch_result: &Result<crate::models::UsageSnapshot, String>,
) -> bool {
    match fetch_result {
        Ok(snapshot) => snapshot.plan_type.is_none(),
        Err(err) => {
            let normalized = err.to_ascii_lowercase();
            normalized.contains("401")
                || normalized.contains("unauthorized")
                || normalized.contains("invalid_token")
                || normalized.contains("deactivated_workspace")
        }
    }
}

fn normalize_usage_error_message(raw_error: &str) -> String {
    let normalized = raw_error.to_ascii_lowercase();
    if normalized.contains("deactivated_workspace") {
        return DEACTIVATED_WORKSPACE_NOTICE.to_string();
    }
    if normalized.contains("your openai account has been deactivated")
        || normalized.contains("account has been deactivated")
        || normalized.contains("account deactivated")
        || normalized.contains("deactivated_user")
        || (normalized.contains("deactivated") && normalized.contains("check your email"))
    {
        return DEACTIVATED_ACCOUNT_NOTICE.to_string();
    }
    if normalized.contains("provided authentication token is expired")
        || normalized
            .contains("your refresh token has already been used to generate a new access token")
        || normalized.contains("please try signing in again")
        || normalized.contains("token is expired")
    {
        return AUTH_EXPIRED_NOTICE.to_string();
    }
    raw_error.to_string()
}

async fn prepare_auth_json_import(
    auth_json: serde_json::Value,
    label: Option<String>,
) -> Result<PreparedImport, String> {
    let extracted = extract_auth(&auth_json)?;

    // 用量拉取失败不阻断导入流程，避免账号无法入库。
    let usage = fetch_usage_snapshot(&extracted.access_token, &extracted.account_id)
        .await
        .ok();

    Ok(PreparedImport {
        auth_json,
        account_id: extracted.account_id,
        email: extracted.email,
        plan_type: extracted.plan_type,
        usage,
        label,
    })
}

async fn commit_prepared_import(
    app: &AppHandle,
    state: &AppState,
    prepared: PreparedImport,
) -> Result<AccountSummary, String> {
    let current_account_id = current_auth_account_id();
    let summary = {
        let mut _guard = state.store_lock.lock().await;
        let mut store = load_store(app)?;
        let (summary, _) =
            upsert_prepared_import(&mut store, prepared, current_account_id.as_deref());
        save_store(app, &store)?;
        summary
    };

    Ok(summary)
}

fn upsert_prepared_import(
    store: &mut AccountsStore,
    prepared: PreparedImport,
    current_account_id: Option<&str>,
) -> (AccountSummary, bool) {
    let PreparedImport {
        auth_json,
        account_id,
        email,
        plan_type,
        usage,
        label,
    } = prepared;

    let now = now_unix_seconds();
    let resolved_label = normalize_custom_label(label)
        .unwrap_or_else(|| fallback_account_label(email.as_deref(), &account_id));

    if let Some(existing) = store
        .accounts
        .iter_mut()
        .find(|account| account.account_id == account_id)
    {
        existing.label = resolved_label;
        existing.email = email;
        existing.plan_type = usage
            .as_ref()
            .and_then(|snapshot| snapshot.plan_type.clone())
            .or(plan_type)
            .or(existing.plan_type.clone());
        existing.auth_json = auth_json;
        existing.updated_at = now;
        existing.usage = usage;
        existing.usage_error = None;
        (existing.to_summary(current_account_id), true)
    } else {
        let stored = StoredAccount {
            id: uuid::Uuid::new_v4().to_string(),
            label: resolved_label,
            email,
            account_id,
            plan_type: usage
                .as_ref()
                .and_then(|snapshot| snapshot.plan_type.clone())
                .or(plan_type),
            auth_json,
            added_at: now,
            updated_at: now,
            usage,
            usage_error: None,
        };
        let summary = stored.to_summary(current_account_id);
        store.accounts.push(stored);
        (summary, false)
    }
}

fn parse_auth_json_content(raw: &str) -> Result<serde_json::Value, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("JSON 内容为空".to_string());
    }

    let normalized = trimmed.strip_prefix('\u{feff}').unwrap_or(trimmed);
    let parsed =
        serde_json::from_str(normalized).map_err(|error| format!("JSON 格式无效: {error}"))?;
    Ok(normalize_imported_auth_json(parsed))
}

fn normalize_custom_label(label: Option<String>) -> Option<String> {
    label.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn fallback_account_label(email: Option<&str>, account_id: &str) -> String {
    email
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("Codex {}", short_account(account_id)))
}

fn normalize_import_source(source: &str) -> String {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        "未命名 JSON".to_string()
    } else {
        trimmed.to_string()
    }
}
