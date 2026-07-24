use super::rollout::enrich_codex_sessions;
use super::storage::{app_data_dir, current_unix_millis, windows_home_dir};
use super::{
    default_codex_phase, CodexSession, CodexStatusSnapshot, PersistedCodexStatus,
    CODEX_RUNNING_STALE_AFTER_MS, CODEX_STATUS_FILE_NAME, CREATE_NO_WINDOW,
};
use std::{fs, os::windows::process::CommandExt, process::Command};
use tauri::AppHandle;

#[tauri::command]
pub(crate) fn get_codex_status(app: AppHandle) -> Result<CodexStatusSnapshot, String> {
    let app_dir = app_data_dir(&app)?;
    let status_path = app_dir.join(CODEX_STATUS_FILE_NAME);
    let status_path_display = status_path.to_string_lossy().to_string();
    let mut persisted = match fs::read_to_string(&status_path) {
        Ok(content) => serde_json::from_str::<PersistedCodexStatus>(&content).unwrap_or_default(),
        Err(_) => PersistedCodexStatus::default(),
    };

    if let Ok(home_dir) = windows_home_dir() {
        enrich_codex_sessions(&mut persisted.sessions, &home_dir);
    }
    normalize_codex_sessions(&mut persisted.sessions);
    let updated_at = if persisted.updated_at > 0 {
        persisted.updated_at
    } else {
        current_unix_millis()
    };

    Ok(CodexStatusSnapshot {
        sessions: persisted.sessions,
        updated_at,
        status_path: status_path_display,
    })
}

#[tauri::command]
pub(crate) fn open_codex_thread(session_id: String) -> Result<(), String> {
    if !is_valid_codex_thread_id(&session_id) {
        return Err("Codex task ID is invalid.".to_string());
    }

    Command::new("explorer.exe")
        .creation_flags(CREATE_NO_WINDOW)
        .arg(format!("codex://threads/{session_id}"))
        .spawn()
        .map_err(|error| format!("Failed to open the Codex task: {error}"))?;
    Ok(())
}

#[tauri::command]
pub(crate) fn clear_codex_status(app: AppHandle) -> Result<CodexStatusSnapshot, String> {
    let app_dir = app_data_dir(&app)?;
    let status_path = app_dir.join(CODEX_STATUS_FILE_NAME);
    match fs::remove_file(&status_path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "Failed to clear Codex status file {}: {error}",
                status_path.display()
            ));
        }
    }

    Ok(CodexStatusSnapshot {
        sessions: Vec::new(),
        updated_at: current_unix_millis(),
        status_path: status_path.to_string_lossy().to_string(),
    })
}

pub(super) fn is_valid_codex_thread_id(value: &str) -> bool {
    value.len() == 36
        && value
            .chars()
            .enumerate()
            .all(|(index, character)| match index {
                8 | 13 | 18 | 23 => character == '-',
                _ => character.is_ascii_hexdigit(),
            })
}

pub(super) fn normalize_codex_sessions(sessions: &mut Vec<CodexSession>) {
    let now = current_unix_millis();
    sessions.retain(|session| !session.session_id.trim().is_empty());

    for session in sessions.iter_mut() {
        if !matches!(
            session.phase.as_str(),
            "idle" | "running" | "completed" | "failed" | "stale"
        ) {
            session.phase = default_codex_phase();
        }

        if session.started_at <= 0 {
            session.started_at = session.updated_at.max(0);
        }
        if session.phase == "running"
            && session.attention.is_empty()
            && session.updated_at > 0
            && now.saturating_sub(session.updated_at) >= CODEX_RUNNING_STALE_AFTER_MS
        {
            session.phase = "stale".to_string();
        }
    }

    sessions.sort_by(|left, right| {
        left.session_id
            .cmp(&right.session_id)
            .then_with(|| right.started_at.cmp(&left.started_at))
            .then_with(|| right.updated_at.cmp(&left.updated_at))
    });
    sessions.dedup_by(|left, right| left.session_id == right.session_id);

    sessions.sort_by(|left, right| {
        let left_running = left.phase == "running";
        let right_running = right.phase == "running";
        right_running
            .cmp(&left_running)
            .then_with(|| right.updated_at.cmp(&left.updated_at))
    });
    sessions.truncate(6);
}
