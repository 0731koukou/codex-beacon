use super::{
    backup_existing_file_once, codex_config_with_hooks_enabled, codex_hook_is_verified,
    codex_hooks_are_installed, current_unix_millis, install_codex_hooks_for_paths,
    install_codex_status_hooks, is_valid_codex_thread_id, migrate_legacy_app_data_for_paths,
    normalize_codex_sessions, parse_rollout_activity, CodexSession, CODEX_EVENT_SCRIPT_FILE_NAME,
    CODEX_STATUS_FILE_NAME, CODEX_VERIFICATION_FILE_NAME,
};
use serde_json::{json, Value};
use std::fs;

#[test]
fn preserves_the_first_codex_config_backup() {
    let directory = std::env::temp_dir().join(format!(
        "codex-beacon-backup-test-{}-{}",
        std::process::id(),
        current_unix_millis()
    ));
    let config_path = directory.join("hooks.json");
    fs::create_dir_all(&directory).expect("create test directory");
    fs::write(&config_path, "original").expect("write original config");

    let backup_path = backup_existing_file_once(&config_path)
        .expect("create backup")
        .expect("backup path");
    fs::write(&config_path, "changed").expect("change config");
    backup_existing_file_once(&config_path).expect("reuse backup");

    assert_eq!(
        fs::read_to_string(backup_path).expect("read backup"),
        "original"
    );
    fs::remove_dir_all(directory).expect("remove test directory");
}

#[test]
fn codex_hooks_install_is_idempotent_and_preserves_other_hooks() {
    let directory = std::env::temp_dir().join(format!(
        "codex-beacon-hooks-test-{}-{}",
        std::process::id(),
        current_unix_millis()
    ));
    let hooks_path = directory.join("hooks.json");
    let script_path = directory.join(CODEX_EVENT_SCRIPT_FILE_NAME);
    fs::create_dir_all(&directory).expect("create test directory");
    fs::write(&script_path, "test").expect("write script");
    fs::write(
        &hooks_path,
        serde_json::to_string_pretty(&json!({
            "custom": "preserved",
            "hooks": {
                "UserPromptSubmit": [
                    {"hooks": [{"type": "command", "command": "other-tool"}]},
                    {"hooks": [{"type": "command", "command": "focusd-codex-old.ps1"}]}
                ]
            }
        }))
        .expect("serialize hooks"),
    )
    .expect("write hooks");

    install_codex_status_hooks(&hooks_path, &script_path).expect("install hooks");
    install_codex_status_hooks(&hooks_path, &script_path).expect("reinstall hooks");

    let installed: Value =
        serde_json::from_str(&fs::read_to_string(&hooks_path).expect("read installed hooks"))
            .expect("parse installed hooks");
    assert_eq!(installed["custom"], "preserved");
    assert_eq!(
        installed["hooks"]["UserPromptSubmit"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(installed["hooks"]["Stop"].as_array().unwrap().len(), 1);
    assert!(codex_hooks_are_installed(&hooks_path));

    fs::remove_dir_all(directory).expect("remove test directory");
}

#[test]
fn codex_hooks_feature_is_enabled_without_changing_other_settings() {
    let config = r#"model = "gpt-5.6"

[features]
js_repl = false
hooks = false
memories = true

[windows]
sandbox = "elevated"
"#;

    let enabled = codex_config_with_hooks_enabled(config);

    assert!(enabled.contains("js_repl = false\nhooks = true\nmemories = true"));
    assert!(enabled.contains("js_repl = false"));
    assert!(enabled.contains("memories = true"));
    assert!(enabled.contains("[windows]\nsandbox = \"elevated\""));
    assert_eq!(enabled.matches("hooks = true").count(), 1);
}

#[test]
fn codex_hooks_feature_section_is_added_once() {
    let config = "model = \"gpt-5.6\"\n";
    let enabled = codex_config_with_hooks_enabled(config);
    let enabled_again = codex_config_with_hooks_enabled(&enabled);

    assert_eq!(enabled, enabled_again);
    assert_eq!(enabled.matches("[features]").count(), 1);
    assert_eq!(enabled.matches("hooks = true").count(), 1);
}

#[test]
fn codex_hook_verification_requires_a_valid_timestamp() {
    let directory = std::env::temp_dir().join(format!(
        "codex-beacon-verification-test-{}-{}",
        std::process::id(),
        current_unix_millis()
    ));
    let verification_path = directory.join("codex-hook-verification.json");
    fs::create_dir_all(&directory).expect("create test directory");

    fs::write(&verification_path, r#"{"verifiedAt":0}"#).expect("write invalid marker");
    assert!(!codex_hook_is_verified(&verification_path));

    fs::write(&verification_path, r#"{"verifiedAt":1784778346765}"#).expect("write valid marker");
    assert!(codex_hook_is_verified(&verification_path));

    fs::remove_dir_all(directory).expect("remove test directory");
}

#[test]
fn reinstall_resets_hook_verification() {
    let directory = std::env::temp_dir().join(format!(
        "codex-beacon-reinstall-test-{}-{}",
        std::process::id(),
        current_unix_millis()
    ));
    let app_dir = directory.join("app-data");
    let home_dir = directory.join("home");
    fs::create_dir_all(&app_dir).expect("create app data directory");
    fs::write(
        app_dir.join(CODEX_VERIFICATION_FILE_NAME),
        r#"{"verifiedAt":1784778346765}"#,
    )
    .expect("write verification marker");

    let result = install_codex_hooks_for_paths(&app_dir, &home_dir).expect("install Codex hooks");

    assert!(!result.verified);
    assert!(!app_dir.join(CODEX_VERIFICATION_FILE_NAME).exists());
    fs::remove_dir_all(directory).expect("remove test directory");
}

#[test]
fn legacy_app_data_is_migrated_without_preserving_trust() {
    let directory = std::env::temp_dir().join(format!(
        "codex-beacon-migration-test-{}-{}",
        std::process::id(),
        current_unix_millis()
    ));
    let app_dir = directory.join("app.codexbeacon.desktop");
    let legacy_dir = directory.join("com.focusd.island");
    fs::create_dir_all(&legacy_dir).expect("create legacy directory");
    fs::write(legacy_dir.join(CODEX_STATUS_FILE_NAME), "status").expect("write legacy status");
    fs::write(
        legacy_dir.join(CODEX_EVENT_SCRIPT_FILE_NAME),
        "legacy script",
    )
    .expect("write legacy script");
    fs::write(
        legacy_dir.join(CODEX_VERIFICATION_FILE_NAME),
        r#"{"verifiedAt":1784778346765}"#,
    )
    .expect("write legacy verification");

    migrate_legacy_app_data_for_paths(&app_dir, &legacy_dir).expect("migrate legacy data");

    assert_eq!(
        fs::read_to_string(app_dir.join(CODEX_STATUS_FILE_NAME)).expect("read migrated status"),
        "status"
    );
    assert!(!app_dir.join(CODEX_VERIFICATION_FILE_NAME).exists());
    assert!(!legacy_dir.exists());
    fs::remove_dir_all(directory).expect("remove test directory");
}

#[test]
fn rollout_activity_tracks_plan_and_pending_approval() {
    let plan_arguments = serde_json::to_string(&json!({
        "plan": [
            {"step": "检查接口", "status": "completed"},
            {"step": "构建 Windows 应用", "status": "in_progress"},
            {"step": "验证安装包", "status": "pending"}
        ]
    }))
    .expect("serialize plan");
    let approval_arguments = serde_json::to_string(&json!({
        "cmd": "pnpm tauri build",
        "sandbox_permissions": "require_escalated"
    }))
    .expect("serialize approval");
    let content = [
        json!({
            "type": "response_item",
            "payload": {
                "type": "function_call",
                "name": "update_plan",
                "arguments": plan_arguments,
                "call_id": "plan"
            }
        }),
        json!({
            "type": "response_item",
            "payload": {
                "type": "function_call_output",
                "call_id": "plan",
                "output": "ok"
            }
        }),
        json!({
            "type": "response_item",
            "payload": {
                "type": "function_call",
                "name": "exec_command",
                "arguments": approval_arguments,
                "call_id": "approval"
            }
        }),
    ]
    .into_iter()
    .map(|record| record.to_string())
    .collect::<Vec<_>>()
    .join("\n");

    let activity = parse_rollout_activity(&content);

    assert_eq!(activity.plan_completed, 1);
    assert_eq!(activity.plan_total, 3);
    assert_eq!(activity.current_step, "构建 Windows 应用");
    assert_eq!(activity.attention, "approval");
    assert_eq!(activity.activity, "等待你批准 Codex 操作");
}

#[test]
fn rollout_activity_clears_attention_after_tool_output() {
    let content = [
        json!({
            "type": "response_item",
            "payload": {
                "type": "function_call",
                "name": "exec_command",
                "arguments": "{\"sandbox_permissions\":\"require_escalated\"}",
                "call_id": "approval"
            }
        }),
        json!({
            "type": "response_item",
            "payload": {
                "type": "function_call_output",
                "call_id": "approval",
                "output": "done"
            }
        }),
    ]
    .into_iter()
    .map(|record| record.to_string())
    .collect::<Vec<_>>()
    .join("\n");

    assert!(parse_rollout_activity(&content).attention.is_empty());
}

#[test]
fn rollout_activity_ignores_attention_from_an_older_turn() {
    let content = [
        json!({
            "type": "response_item",
            "payload": {
                "type": "function_call",
                "name": "exec_command",
                "arguments": "{\"sandbox_permissions\":\"require_escalated\"}",
                "call_id": "old-approval"
            }
        }),
        json!({
            "type": "event_msg",
            "payload": {
                "type": "task_started",
                "turn_id": "new-turn"
            }
        }),
        json!({
            "type": "event_msg",
            "payload": {
                "type": "agent_reasoning",
                "text": "working"
            }
        }),
    ]
    .into_iter()
    .map(|record| record.to_string())
    .collect::<Vec<_>>()
    .join("\n");

    let activity = parse_rollout_activity(&content);

    assert!(activity.attention.is_empty());
    assert_eq!(activity.activity, "正在分析下一步");
}

#[test]
fn rollout_activity_marks_interrupted_turn_failed() {
    let content = [
        json!({
            "type": "event_msg",
            "payload": {
                "type": "task_started",
                "turn_id": "stopped-turn"
            }
        }),
        json!({
            "type": "event_msg",
            "payload": {
                "type": "turn_aborted",
                "turn_id": "stopped-turn",
                "reason": "interrupted",
                "completed_at": 1_784_790_288
            }
        }),
    ]
    .into_iter()
    .map(|record| record.to_string())
    .collect::<Vec<_>>()
    .join("\n");

    let activity = parse_rollout_activity(&content);
    let terminal = activity
        .terminal_turns
        .get("stopped-turn")
        .expect("interrupted turn should be terminal");

    assert_eq!(terminal.phase, "failed");
    assert_eq!(terminal.updated_at, 1_784_790_288_000);
}

#[test]
fn only_latest_turn_in_a_thread_stays_running() {
    let now = current_unix_millis();
    let mut sessions = serde_json::from_value::<Vec<CodexSession>>(json!([
        {
            "sessionId": "thread",
            "turnId": "stopped-turn",
            "phase": "running",
            "startedAt": now - 2_000,
            "updatedAt": now - 2_000
        },
        {
            "sessionId": "thread",
            "turnId": "current-turn",
            "phase": "running",
            "startedAt": now,
            "updatedAt": now
        }
    ]))
    .expect("deserialize sessions");

    normalize_codex_sessions(&mut sessions);

    assert_eq!(
        sessions
            .iter()
            .find(|session| session.turn_id == "stopped-turn")
            .expect("stopped turn")
            .phase,
        "failed"
    );
    assert_eq!(
        sessions
            .iter()
            .find(|session| session.turn_id == "current-turn")
            .expect("current turn")
            .phase,
        "running"
    );
}

#[test]
fn codex_thread_links_only_accept_uuid_ids() {
    assert!(is_valid_codex_thread_id(
        "019f8ca3-5c75-7c71-8c3d-59eacd1f284a"
    ));
    assert!(!is_valid_codex_thread_id(
        "019f8ca3-5c75-7c71-8c3d-59eacd1f284a?unsafe=true"
    ));
    assert!(!is_valid_codex_thread_id("codex://threads/example"));
}
