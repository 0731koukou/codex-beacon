pub(super) mod integration;
mod rollout;
pub(super) mod status;
mod storage;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use integration::install_codex_hooks_for_current_user;

const APP_DATA_DIRECTORY_NAME: &str = "app.codexbeacon.desktop";
const LEGACY_APP_DATA_DIRECTORY_NAME: &str = "com.focusd.island";
const CODEX_STATUS_FILE_NAME: &str = "codex-status.json";
const CODEX_VERIFICATION_FILE_NAME: &str = "codex-hook-verification.json";
const CODEX_EVENT_SCRIPT_FILE_NAME: &str = "codex-beacon-event.ps1";
const LEGACY_CODEX_EVENT_SCRIPT_FILE_NAME: &str = "focusd-codex-event.ps1";
const CODEX_RUNNING_STALE_AFTER_MS: i64 = 2 * 60 * 60 * 1000;
const MAX_ROLLOUT_TAIL_BYTES: u64 = 2 * 1024 * 1024;
const MANAGED_HOOK_SIGNATURES: [&str; 2] = ["codex-beacon-", "focusd-codex-"];
const CODEX_EVENT_SCRIPT: &str = include_str!("../../scripts/codex-beacon-event.ps1");
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CodexSession {
    #[serde(default)]
    session_id: String,
    #[serde(default)]
    turn_id: String,
    #[serde(default = "default_codex_phase")]
    phase: String,
    #[serde(default)]
    prompt: String,
    #[serde(default)]
    cwd: String,
    #[serde(default)]
    model: String,
    #[serde(default)]
    last_assistant_message: String,
    #[serde(default)]
    started_at: i64,
    #[serde(default)]
    updated_at: i64,
    #[serde(default)]
    activity: String,
    #[serde(default)]
    attention: String,
    #[serde(default)]
    plan_completed: usize,
    #[serde(default)]
    plan_total: usize,
    #[serde(default)]
    current_step: String,
}

#[derive(Debug, Clone, Default)]
struct RolloutActivity {
    activity: String,
    attention: String,
    plan_completed: usize,
    plan_total: usize,
    current_step: String,
    updated_at: i64,
    terminal_turns: HashMap<String, RolloutTerminal>,
}

#[derive(Debug, Clone)]
struct RolloutTerminal {
    phase: String,
    updated_at: i64,
}

#[derive(Debug, Clone)]
struct CachedRolloutActivity {
    file_length: u64,
    modified_at: i64,
    activity: RolloutActivity,
}

#[derive(Debug)]
struct PendingCall {
    name: String,
    input: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PersistedCodexStatus {
    #[serde(default)]
    sessions: Vec<CodexSession>,
    #[serde(default)]
    updated_at: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexStatusSnapshot {
    sessions: Vec<CodexSession>,
    updated_at: i64,
    status_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexIntegrationStatus {
    configured: bool,
    verified: bool,
    hooks_path: String,
    script_path: String,
    verification_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexHooksInstallResult {
    configured: bool,
    verified: bool,
    status_path: String,
    hooks_path: String,
    script_path: String,
    verification_path: String,
    backup_path: Option<String>,
    installed_at: i64,
}

fn default_codex_phase() -> String {
    "idle".to_string()
}

#[cfg(test)]
use integration::{
    codex_config_with_hooks_enabled, codex_hook_is_verified, codex_hooks_are_installed,
    install_codex_hooks_for_paths, install_codex_status_hooks,
};
#[cfg(test)]
use rollout::parse_rollout_activity;
#[cfg(test)]
use status::{is_valid_codex_thread_id, normalize_codex_sessions};
#[cfg(test)]
use storage::{backup_existing_file_once, current_unix_millis, migrate_legacy_app_data_for_paths};

#[cfg(test)]
mod tests;
