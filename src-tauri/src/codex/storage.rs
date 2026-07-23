use super::{
    APP_DATA_DIRECTORY_NAME, CODEX_EVENT_SCRIPT_FILE_NAME, CODEX_STATUS_FILE_NAME,
    CODEX_VERIFICATION_FILE_NAME, LEGACY_APP_DATA_DIRECTORY_NAME,
    LEGACY_CODEX_EVENT_SCRIPT_FILE_NAME,
};
use std::{
    env, fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};

pub(super) fn app_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let app_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Failed to resolve app data directory: {error}"))?;
    fs::create_dir_all(&app_dir)
        .map_err(|error| format!("Failed to create app data directory: {error}"))?;
    migrate_legacy_app_data(&app_dir)?;
    Ok(app_dir)
}

pub(super) fn current_app_data_dir() -> Result<PathBuf, String> {
    Ok(windows_app_data_dir()?.join(APP_DATA_DIRECTORY_NAME))
}

pub(super) fn migrate_legacy_app_data(app_dir: &Path) -> Result<(), String> {
    let legacy_dir = windows_app_data_dir()?.join(LEGACY_APP_DATA_DIRECTORY_NAME);
    migrate_legacy_app_data_for_paths(app_dir, &legacy_dir)
}

pub(super) fn migrate_legacy_app_data_for_paths(
    app_dir: &Path,
    legacy_dir: &Path,
) -> Result<(), String> {
    if app_dir == legacy_dir || !legacy_dir.is_dir() {
        return Ok(());
    }

    fs::create_dir_all(app_dir)
        .map_err(|error| format!("Failed to create app data directory: {error}"))?;

    let legacy_status_path = legacy_dir.join(CODEX_STATUS_FILE_NAME);
    let status_path = app_dir.join(CODEX_STATUS_FILE_NAME);
    if legacy_status_path.is_file() && !status_path.exists() {
        fs::rename(&legacy_status_path, &status_path)
            .or_else(|_| {
                fs::copy(&legacy_status_path, &status_path)?;
                fs::remove_file(&legacy_status_path)
            })
            .map_err(|error| format!("Failed to migrate Codex task status: {error}"))?;
    }

    for file_name in [
        CODEX_EVENT_SCRIPT_FILE_NAME,
        LEGACY_CODEX_EVENT_SCRIPT_FILE_NAME,
        CODEX_VERIFICATION_FILE_NAME,
    ] {
        let path = legacy_dir.join(file_name);
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "Failed to remove legacy app data file {}: {error}",
                    path.display()
                ));
            }
        }
    }

    if fs::read_dir(legacy_dir)
        .map(|mut entries| entries.next().is_none())
        .unwrap_or(false)
    {
        fs::remove_dir(legacy_dir).map_err(|error| {
            format!(
                "Failed to remove legacy app data directory {}: {error}",
                legacy_dir.display()
            )
        })?;
    }

    Ok(())
}

fn windows_app_data_dir() -> Result<PathBuf, String> {
    env::var_os("APPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| "Failed to resolve Windows APPDATA directory.".to_string())
}

pub(super) fn windows_home_dir() -> Result<PathBuf, String> {
    if let Ok(user_profile) = env::var("USERPROFILE") {
        let user_profile = user_profile.trim();
        if !user_profile.is_empty() {
            return Ok(PathBuf::from(user_profile));
        }
    }

    match (env::var("HOMEDRIVE"), env::var("HOMEPATH")) {
        (Ok(home_drive), Ok(home_path)) if !home_drive.is_empty() && !home_path.is_empty() => {
            Ok(PathBuf::from(format!("{home_drive}{home_path}")))
        }
        _ => Err("Failed to resolve the Windows user profile directory.".to_string()),
    }
}

pub(super) fn write_text_file(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create {}: {error}", parent.display()))?;
    }

    let temporary_path = path.with_extension("tmp");
    fs::write(&temporary_path, content)
        .map_err(|error| format!("Failed to write {}: {error}", temporary_path.display()))?;
    fs::rename(&temporary_path, path)
        .or_else(|_| {
            fs::remove_file(path).ok();
            fs::rename(&temporary_path, path)
        })
        .map_err(|error| format!("Failed to replace {}: {error}", path.display()))
}

pub(super) fn backup_existing_file_once(path: &Path) -> Result<Option<PathBuf>, String> {
    if !path.is_file() {
        return Ok(None);
    }

    let file_name = path
        .file_name()
        .ok_or_else(|| format!("Failed to resolve backup name for {}", path.display()))?;
    let backup_path = path.with_file_name(format!(
        "{}.codex-beacon-backup",
        file_name.to_string_lossy()
    ));

    if !backup_path.exists() {
        fs::copy(path, &backup_path).map_err(|error| {
            format!(
                "Failed to back up {} to {}: {error}",
                path.display(),
                backup_path.display()
            )
        })?;
    }

    Ok(Some(backup_path))
}

pub(super) fn normalize_windows_line_endings(content: &str) -> String {
    content.replace("\r\n", "\n").replace('\n', "\r\n")
}

pub(super) fn current_unix_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}
