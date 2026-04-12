use crate::models::codex::CodexAccount;
use crate::modules::{atomic_write, logger};
use chrono::{SecondsFormat, Utc};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const HERMES_PROVIDER_KEY: &str = "openai-codex";
const HERMES_DEFAULT_BASE_URL: &str = "https://chatgpt.com/backend-api/codex";

fn get_hermes_auth_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("无法获取用户主目录")?;
    Ok(home.join(".hermes").join("auth.json"))
}

fn ensure_object<'a>(
    value: &'a mut Value,
    field_name: &str,
) -> Result<&'a mut Map<String, Value>, String> {
    if value.is_null() {
        *value = Value::Object(Map::new());
    }
    value
        .as_object_mut()
        .ok_or_else(|| format!("Hermes auth.json 字段不是对象: {}", field_name))
}

fn ensure_array<'a>(value: &'a mut Value, field_name: &str) -> Result<&'a mut Vec<Value>, String> {
    if value.is_null() {
        *value = Value::Array(Vec::new());
    }
    value
        .as_array_mut()
        .ok_or_else(|| format!("Hermes auth.json 字段不是数组: {}", field_name))
}

fn build_provider_tokens(account: &CodexAccount) -> Value {
    json!({
        "id_token": account.tokens.id_token,
        "access_token": account.tokens.access_token,
        "refresh_token": account.tokens.refresh_token,
        "account_id": account.account_id,
    })
}

fn build_pool_entry(existing: Option<&Value>, account: &CodexAccount, now_rfc3339: &str) -> Value {
    let mut entry = existing
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();

    entry.insert(
        "id".to_string(),
        Value::String(account.id.chars().take(6).collect::<String>()),
    );
    entry.insert(
        "label".to_string(),
        entry
            .get("label")
            .cloned()
            .unwrap_or_else(|| Value::String("device_code".to_string())),
    );
    entry.insert("auth_type".to_string(), Value::String("oauth".to_string()));
    entry.insert(
        "priority".to_string(),
        entry.get("priority").cloned().unwrap_or_else(|| json!(0)),
    );
    entry.insert(
        "source".to_string(),
        Value::String("codex_switch".to_string()),
    );
    entry.insert(
        "access_token".to_string(),
        Value::String(account.tokens.access_token.clone()),
    );
    entry.insert(
        "refresh_token".to_string(),
        account
            .tokens
            .refresh_token
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    entry.insert("last_status".to_string(), Value::Null);
    entry.insert("last_status_at".to_string(), Value::Null);
    entry.insert("last_error_code".to_string(), Value::Null);
    entry.insert("last_error_reason".to_string(), Value::Null);
    entry.insert("last_error_message".to_string(), Value::Null);
    entry.insert("last_error_reset_at".to_string(), Value::Null);
    entry.insert(
        "base_url".to_string(),
        entry
            .get("base_url")
            .cloned()
            .unwrap_or_else(|| Value::String(HERMES_DEFAULT_BASE_URL.to_string())),
    );
    entry.insert(
        "last_refresh".to_string(),
        Value::String(now_rfc3339.to_string()),
    );
    entry.insert("request_count".to_string(), json!(0));

    Value::Object(entry)
}

pub fn replace_openai_codex_entry_from_codex(account: &CodexAccount) -> Result<(), String> {
    if account.is_api_key_auth() {
        return Err("Hermes 同步仅支持 Codex OAuth 账号".to_string());
    }

    let auth_path = get_hermes_auth_path()?;
    let now_rfc3339 = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);

    logger::log_info(&format!(
        "[HermesAuth] 准备同步 Hermes openai-codex 登录信息: account_id={}, email={}, path={}",
        account.id,
        account.email,
        auth_path.display()
    ));

    let mut root = if auth_path.exists() {
        let content = fs::read_to_string(&auth_path)
            .map_err(|e| format!("读取 Hermes auth.json 失败: {}", e))?;
        atomic_write::parse_json_with_auto_restore::<Value>(&auth_path, &content)
            .map_err(|e| format!("解析 Hermes auth.json 失败: {}", e))?
    } else {
        json!({})
    };

    let root_obj = ensure_object(&mut root, "root")?;
    if !root_obj.contains_key("version") {
        root_obj.insert("version".to_string(), json!(1));
    }

    {
        let providers = ensure_object(
            root_obj
                .entry("providers".to_string())
                .or_insert_with(|| Value::Object(Map::new())),
            "providers",
        )?;
        let provider = ensure_object(
            providers
                .entry(HERMES_PROVIDER_KEY.to_string())
                .or_insert_with(|| Value::Object(Map::new())),
            "providers.openai-codex",
        )?;
        provider.insert("tokens".to_string(), build_provider_tokens(account));
        provider.insert(
            "last_refresh".to_string(),
            Value::String(now_rfc3339.clone()),
        );
        provider.insert(
            "auth_mode".to_string(),
            Value::String("chatgpt".to_string()),
        );
    }

    {
        let credential_pool = ensure_object(
            root_obj
                .entry("credential_pool".to_string())
                .or_insert_with(|| Value::Object(Map::new())),
            "credential_pool",
        )?;
        let pool = ensure_array(
            credential_pool
                .entry(HERMES_PROVIDER_KEY.to_string())
                .or_insert_with(|| Value::Array(Vec::new())),
            "credential_pool.openai-codex",
        )?;
        let next_entry = build_pool_entry(pool.first(), account, &now_rfc3339);
        pool.clear();
        pool.push(next_entry);
    }

    root_obj.insert("updated_at".to_string(), Value::String(now_rfc3339.clone()));

    let content = serde_json::to_string_pretty(&root)
        .map_err(|e| format!("序列化 Hermes auth.json 失败: {}", e))?;
    atomic_write::write_string_atomic(&auth_path, &content)?;

    logger::log_info(&format!(
        "[HermesAuth] 已同步 Hermes openai-codex 登录信息: account_id={}, path={}",
        account.id,
        auth_path.display()
    ));

    Ok(())
}

pub fn restart_gateway() -> Result<(), String> {
    #[derive(Clone)]
    struct RestartCommand {
        program: String,
        args: Vec<String>,
        label: String,
    }

    fn push_restart_command(commands: &mut Vec<RestartCommand>, command: RestartCommand) {
        if commands
            .iter()
            .any(|item| item.program == command.program && item.args == command.args)
        {
            return;
        }
        commands.push(command);
    }

    let mut commands: Vec<RestartCommand> = Vec::new();
    if let Ok(cli_path) = std::env::var("HERMES_CLI_PATH") {
        let trimmed = cli_path.trim();
        if !trimmed.is_empty() {
            push_restart_command(
                &mut commands,
                RestartCommand {
                    program: trimmed.to_string(),
                    args: vec!["gateway".to_string(), "restart".to_string()],
                    label: format!("HERMES_CLI_PATH ({}) gateway restart", trimmed),
                },
            );
        }
    }
    push_restart_command(
        &mut commands,
        RestartCommand {
            program: "hermes".to_string(),
            args: vec!["gateway".to_string(), "restart".to_string()],
            label: "hermes gateway restart".to_string(),
        },
    );

    let mut not_found_labels: Vec<String> = Vec::new();
    let mut failed_outputs: Vec<String> = Vec::new();

    for restart_command in commands {
        let mut command = Command::new(&restart_command.program);
        command.args(&restart_command.args);

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }

        match command.output() {
            Ok(output) => {
                if output.status.success() {
                    logger::log_info(&format!(
                        "[HermesAuth] gateway restart 已触发（{}）",
                        restart_command.label
                    ));
                    return Ok(());
                }
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                failed_outputs.push(format!(
                    "{} status={} stderr={} stdout={}",
                    restart_command.label,
                    output.status,
                    if stderr.is_empty() {
                        "<empty>"
                    } else {
                        &stderr
                    },
                    if stdout.is_empty() {
                        "<empty>"
                    } else {
                        &stdout
                    }
                ));
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                not_found_labels.push(restart_command.label);
            }
            Err(err) => {
                failed_outputs.push(format!("{} error={}", restart_command.label, err));
            }
        }
    }

    if !failed_outputs.is_empty() {
        return Err(format!(
            "执行 Hermes gateway restart 失败: {}",
            failed_outputs.join(" | ")
        ));
    }
    if !not_found_labels.is_empty() {
        return Err(format!(
            "未找到可用的 Hermes CLI: {}",
            not_found_labels.join(", ")
        ));
    }

    Err("未能触发 Hermes gateway restart".to_string())
}
