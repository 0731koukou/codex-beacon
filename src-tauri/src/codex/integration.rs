use super::storage::{
    app_data_dir, backup_existing_file_once, current_app_data_dir, current_unix_millis,
    migrate_legacy_app_data, normalize_windows_line_endings, windows_home_dir, write_text_file,
};
use super::{
    CodexHooksInstallResult, CodexIntegrationStatus, CODEX_EVENT_SCRIPT,
    CODEX_EVENT_SCRIPT_FILE_NAME, CODEX_STATUS_FILE_NAME, CODEX_VERIFICATION_FILE_NAME,
    LEGACY_CODEX_EVENT_SCRIPT_FILE_NAME, MANAGED_HOOK_SIGNATURES,
};
use serde_json::{json, Map, Value};
use std::{fs, path::Path};
use tauri::AppHandle;

#[tauri::command]
pub(crate) fn get_codex_integration_status() -> Result<CodexIntegrationStatus, String> {
    let home_dir = windows_home_dir()?;
    let app_dir = current_app_data_dir()?;
    migrate_legacy_app_data(&app_dir)?;
    let hooks_path = home_dir.join(".codex").join("hooks.json");
    let script_path = app_dir.join(CODEX_EVENT_SCRIPT_FILE_NAME);
    let verification_path = app_dir.join(CODEX_VERIFICATION_FILE_NAME);
    let configured = script_path.is_file() && codex_hooks_are_installed(&hooks_path);

    Ok(CodexIntegrationStatus {
        configured,
        verified: configured && codex_hook_is_verified(&verification_path),
        hooks_path: hooks_path.to_string_lossy().to_string(),
        script_path: script_path.to_string_lossy().to_string(),
        verification_path: verification_path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub(crate) fn install_codex_hooks(app: AppHandle) -> Result<CodexHooksInstallResult, String> {
    let app_dir = app_data_dir(&app)?;
    let home_dir = windows_home_dir()?;
    install_codex_hooks_for_paths(&app_dir, &home_dir)
}

pub fn install_codex_hooks_for_current_user() -> Result<(), String> {
    let app_dir = current_app_data_dir()?;
    migrate_legacy_app_data(&app_dir)?;
    let home_dir = windows_home_dir()?;
    install_codex_hooks_for_paths(&app_dir, &home_dir).map(|_| ())
}

pub(super) fn install_codex_hooks_for_paths(
    app_dir: &Path,
    home_dir: &Path,
) -> Result<CodexHooksInstallResult, String> {
    fs::create_dir_all(app_dir)
        .map_err(|error| format!("Failed to create app data directory: {error}"))?;

    let script_path = app_dir.join(CODEX_EVENT_SCRIPT_FILE_NAME);
    write_text_file(
        &script_path,
        &normalize_windows_line_endings(CODEX_EVENT_SCRIPT),
    )?;
    let legacy_script_path = app_dir.join(LEGACY_CODEX_EVENT_SCRIPT_FILE_NAME);
    if legacy_script_path.is_file() {
        fs::remove_file(&legacy_script_path).map_err(|error| {
            format!(
                "Failed to remove legacy event script {}: {error}",
                legacy_script_path.display()
            )
        })?;
    }

    let codex_directory = home_dir.join(".codex");
    let hooks_path = codex_directory.join("hooks.json");
    let backup_path = backup_existing_file_once(&hooks_path)?;
    install_codex_status_hooks(&hooks_path, &script_path)?;
    let config_path = codex_directory.join("config.toml");
    enable_codex_hooks_feature(&config_path)?;
    let verification_path = app_dir.join(CODEX_VERIFICATION_FILE_NAME);
    match fs::remove_file(&verification_path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "Failed to reset Codex hook verification {}: {error}",
                verification_path.display()
            ));
        }
    }

    Ok(CodexHooksInstallResult {
        configured: true,
        verified: false,
        status_path: app_dir
            .join(CODEX_STATUS_FILE_NAME)
            .to_string_lossy()
            .to_string(),
        hooks_path: hooks_path.to_string_lossy().to_string(),
        script_path: script_path.to_string_lossy().to_string(),
        verification_path: verification_path.to_string_lossy().to_string(),
        backup_path: backup_path.map(|path| path.to_string_lossy().to_string()),
        installed_at: current_unix_millis(),
    })
}

pub(super) fn install_codex_status_hooks(
    hooks_path: &Path,
    script_path: &Path,
) -> Result<(), String> {
    let mut config = match fs::read_to_string(hooks_path) {
        Ok(content) if !content.trim().is_empty() => serde_json::from_str::<Value>(&content)
            .map_err(|error| format!("Failed to parse Codex hooks.json: {error}"))?,
        _ => json!({}),
    };

    let Some(root) = config.as_object_mut() else {
        return Err("Codex hooks.json must contain a JSON object.".to_string());
    };
    let hooks = root
        .entry("hooks")
        .or_insert_with(|| Value::Object(Map::new()));
    if !hooks.is_object() {
        *hooks = Value::Object(Map::new());
    }
    let hooks = hooks
        .as_object_mut()
        .ok_or_else(|| "Failed to prepare Codex hooks object.".to_string())?;

    install_codex_hook_event(
        hooks,
        "UserPromptSubmit",
        codex_event_hook_entry(script_path, "running"),
    );
    install_codex_hook_event(
        hooks,
        "Stop",
        codex_event_hook_entry(script_path, "completed"),
    );

    let content = serde_json::to_string_pretty(&config)
        .map_err(|error| format!("Failed to serialize Codex hooks.json: {error}"))?;
    write_text_file(hooks_path, &content)
}

fn install_codex_hook_event(hooks: &mut Map<String, Value>, event_name: &str, entry: Value) {
    let mut entries = hooks
        .remove(event_name)
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default()
        .into_iter()
        .filter_map(remove_managed_hooks)
        .collect::<Vec<_>>();

    entries.push(entry);
    hooks.insert(event_name.to_string(), Value::Array(entries));
}

fn codex_event_hook_entry(script_path: &Path, phase: &str) -> Value {
    let command = format!(
        "powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File \"{}\" {}",
        script_path.to_string_lossy(),
        phase
    );

    json!({
        "hooks": [
            {
                "type": "command",
                "command": command,
                "timeout": 5,
                "statusMessage": "Syncing Codex task to Codex Beacon"
            }
        ]
    })
}

pub(super) fn codex_hooks_are_installed(hooks_path: &Path) -> bool {
    let Ok(content) = fs::read_to_string(hooks_path) else {
        return false;
    };
    let Ok(config) = serde_json::from_str::<Value>(&content) else {
        return false;
    };
    let Some(hooks) = config.get("hooks").and_then(Value::as_object) else {
        return false;
    };

    ["UserPromptSubmit", "Stop"].iter().all(|event_name| {
        hooks
            .get(*event_name)
            .is_some_and(|entry| value_contains_signature(entry, CODEX_EVENT_SCRIPT_FILE_NAME))
    })
}

pub(super) fn codex_hook_is_verified(verification_path: &Path) -> bool {
    fs::read_to_string(verification_path)
        .ok()
        .and_then(|content| serde_json::from_str::<Value>(&content).ok())
        .and_then(|verification| {
            verification
                .get("verifiedAt")
                .and_then(Value::as_i64)
                .filter(|verified_at| *verified_at > 0)
        })
        .is_some()
}

fn remove_managed_hooks(mut entry: Value) -> Option<Value> {
    let Value::Object(entry_object) = &mut entry else {
        return Some(entry);
    };
    let Some(Value::Array(hooks)) = entry_object.get_mut("hooks") else {
        return Some(entry);
    };

    hooks.retain(|hook| !value_contains_managed_hook_signature(hook));
    if hooks.is_empty() {
        None
    } else {
        Some(entry)
    }
}

fn value_contains_managed_hook_signature(value: &Value) -> bool {
    MANAGED_HOOK_SIGNATURES
        .iter()
        .any(|signature| value_contains_signature(value, signature))
}

fn value_contains_signature(value: &Value, signature: &str) -> bool {
    match value {
        Value::String(text) => text.contains(signature),
        Value::Array(values) => values
            .iter()
            .any(|value| value_contains_signature(value, signature)),
        Value::Object(values) => values
            .values()
            .any(|value| value_contains_signature(value, signature)),
        _ => false,
    }
}

fn enable_codex_hooks_feature(config_path: &Path) -> Result<(), String> {
    let content = fs::read_to_string(config_path).unwrap_or_default();
    let next_content = codex_config_with_hooks_enabled(&content);
    if next_content == content {
        return Ok(());
    }

    backup_existing_file_once(config_path)?;
    write_text_file(config_path, &next_content)
}

pub(super) fn codex_config_with_hooks_enabled(content: &str) -> String {
    let had_trailing_newline = content.ends_with('\n');
    let mut lines = content.lines().map(str::to_string).collect::<Vec<_>>();
    let feature_header = lines.iter().position(|line| line.trim() == "[features]");

    if let Some(header_index) = feature_header {
        let section_end = lines
            .iter()
            .enumerate()
            .skip(header_index + 1)
            .find_map(|(index, line)| line.trim().starts_with('[').then_some(index))
            .unwrap_or(lines.len());
        let hooks_line = lines
            .iter()
            .enumerate()
            .take(section_end)
            .skip(header_index + 1)
            .find_map(|(index, line)| {
                line.split_once('=')
                    .filter(|(key, _)| key.trim() == "hooks")
                    .map(|_| index)
            });

        if let Some(hooks_index) = hooks_line {
            lines[hooks_index] = "hooks = true".to_string();
        } else {
            lines.insert(header_index + 1, "hooks = true".to_string());
        }
    } else {
        if lines.last().is_some_and(|line| !line.trim().is_empty()) {
            lines.push(String::new());
        }
        lines.push("[features]".to_string());
        lines.push("hooks = true".to_string());
    }

    let mut next_content = lines.join("\n");
    if had_trailing_newline || content.is_empty() {
        next_content.push('\n');
    }
    next_content
}
