use std::fs;
use std::path::PathBuf;

use tauri::AppHandle;
use tauri::Manager;
use uuid::Uuid;

use crate::auth::extract_auth;
use crate::auth::read_current_codex_auth_optional;
use crate::models::AccountsStore;
use crate::models::StoredAccount;
use crate::utils::now_unix_seconds;
use crate::utils::set_private_permissions;
use crate::utils::short_account;

pub(crate) fn load_store(app: &AppHandle) -> Result<AccountsStore, String> {
    let path = account_store_path(app)?;
    if !path.exists() {
        return Ok(AccountsStore::default());
    }

    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("读取账号存储文件失败 {}: {e}", path.display()))?;

    match serde_json::from_str::<AccountsStore>(&raw) {
        Ok(store) => Ok(store),
        Err(primary_err) => {
            // 兼容“合法 JSON 后拼接了额外字符/内容”的场景，尝试恢复首个对象。
            let mut stream = serde_json::Deserializer::from_str(&raw).into_iter::<AccountsStore>();
            if let Some(Ok(recovered)) = stream.next() {
                log::warn!(
                    "账号存储文件存在尾随内容，已自动恢复首个 JSON 对象 {}: {}",
                    path.display(),
                    primary_err
                );
                if let Err(repair_err) = write_store_file(&path, &recovered) {
                    log::warn!(
                        "恢复后重写账号存储文件失败 {}: {}",
                        path.display(),
                        repair_err
                    );
                }
                return Ok(recovered);
            }

            // 无法恢复时，备份损坏文件并重建默认文件，避免启动阶段崩溃。
            if let Err(backup_err) = backup_corrupted_store_file(&path, &raw) {
                log::warn!(
                    "账号存储文件损坏，备份失败 {}: {}",
                    path.display(),
                    backup_err
                );
            }

            let fallback = AccountsStore::default();
            if let Err(repair_err) = write_store_file(&path, &fallback) {
                return Err(format!(
                    "账号存储文件格式无效且修复失败 {}: {}; {}",
                    path.display(),
                    primary_err,
                    repair_err
                ));
            }

            log::warn!(
                "账号存储文件格式无效，已重建默认存储 {}: {}",
                path.display(),
                primary_err
            );
            Ok(fallback)
        }
    }
}

pub(crate) fn save_store(app: &AppHandle, store: &AccountsStore) -> Result<(), String> {
    let path = account_store_path(app)?;
    write_store_file(&path, store)
}

/// 启动时自动同步当前登录账号：
/// 若本机已有 `~/.codex/auth.json` 且账号不在列表中，则自动写入存储。
pub(crate) fn sync_current_auth_account_on_startup(app: &AppHandle) -> Result<(), String> {
    let auth_json = match read_current_codex_auth_optional()? {
        Some(value) => value,
        None => return Ok(()),
    };

    let extracted = match extract_auth(&auth_json) {
        Ok(value) => value,
        Err(err) => {
            log::warn!("跳过启动自动导入当前账号: {err}");
            return Ok(());
        }
    };

    let mut store = load_store(app)?;
    let already_exists = store
        .accounts
        .iter()
        .any(|account| account.account_id == extracted.account_id);
    if already_exists {
        return Ok(());
    }

    let now = now_unix_seconds();
    let label = extracted
        .email
        .clone()
        .unwrap_or_else(|| format!("Codex {}", short_account(&extracted.account_id)));

    let stored = StoredAccount {
        id: Uuid::new_v4().to_string(),
        label,
        email: extracted.email,
        account_id: extracted.account_id,
        plan_type: extracted.plan_type,
        auth_json,
        added_at: now,
        updated_at: now,
        usage: None,
        usage_error: None,
    };
    store.accounts.push(stored);
    save_store(app, &store)?;
    Ok(())
}

fn account_store_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("无法获取应用数据目录: {e}"))?;
    Ok(dir.join("accounts.json"))
}

fn write_store_file(path: &PathBuf, store: &AccountsStore) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("无法解析存储目录 {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|e| format!("创建存储目录失败 {}: {e}", parent.display()))?;

    let serialized =
        serde_json::to_string_pretty(store).map_err(|e| format!("序列化账号存储失败: {e}"))?;
    fs::write(path, serialized)
        .map_err(|e| format!("写入账号存储文件失败 {}: {e}", path.display()))?;
    set_private_permissions(path);
    Ok(())
}

fn backup_corrupted_store_file(path: &PathBuf, raw: &str) -> Result<PathBuf, String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("无法解析存储目录 {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|e| format!("创建存储目录失败 {}: {e}", parent.display()))?;

    let backup_path = parent.join(format!("accounts.corrupt-{}.json", now_unix_seconds()));
    fs::write(&backup_path, raw)
        .map_err(|e| format!("写入损坏备份文件失败 {}: {e}", backup_path.display()))?;
    set_private_permissions(&backup_path);
    Ok(backup_path)
}
