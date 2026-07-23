use super::status::is_valid_codex_thread_id;
use super::{
    CachedRolloutActivity, CodexSession, PendingCall, RolloutActivity, RolloutTerminal,
    MAX_ROLLOUT_TAIL_BYTES,
};
use serde_json::{Map, Value};
use std::{
    collections::HashMap,
    fs,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

static ROLLOUT_PATHS: OnceLock<Mutex<HashMap<String, PathBuf>>> = OnceLock::new();
static ROLLOUT_ACTIVITY: OnceLock<Mutex<HashMap<String, CachedRolloutActivity>>> = OnceLock::new();

pub(super) fn enrich_codex_sessions(sessions: &mut [CodexSession], home_dir: &Path) {
    let sessions_dir = home_dir.join(".codex").join("sessions");
    if !sessions_dir.is_dir() {
        return;
    }

    for session in sessions
        .iter_mut()
        .filter(|session| session.phase == "running" || session.phase == "stale")
    {
        let Some(path) = rollout_path_for_session(&sessions_dir, &session.session_id) else {
            continue;
        };
        let Ok(activity) = cached_rollout_activity(&session.session_id, &path) else {
            continue;
        };

        if let Some(terminal) = activity.terminal_turns.get(&session.turn_id) {
            session.phase = terminal.phase.clone();
            session.activity = if terminal.phase == "completed" {
                "任务已完成".to_string()
            } else {
                "任务已停止".to_string()
            };
            session.attention.clear();
            session.updated_at = session.updated_at.max(terminal.updated_at);
            continue;
        }

        session.activity = activity.activity;
        session.attention = activity.attention;
        session.plan_completed = activity.plan_completed;
        session.plan_total = activity.plan_total;
        session.current_step = activity.current_step;
        session.updated_at = session.updated_at.max(activity.updated_at);
    }
}

fn rollout_path_for_session(sessions_dir: &Path, session_id: &str) -> Option<PathBuf> {
    if !is_valid_codex_thread_id(session_id) {
        return None;
    }

    if let Some(path) = rollout_paths()
        .lock()
        .expect("rollout path cache poisoned")
        .get(session_id)
        .filter(|path| path.is_file())
        .cloned()
    {
        return Some(path);
    }

    let file_suffix = format!("-{session_id}.jsonl");
    let mut directories = vec![sessions_dir.to_path_buf()];
    while let Some(directory) = directories.pop() {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                directories.push(entry.path());
                continue;
            }
            if file_type.is_file() && entry.file_name().to_string_lossy().ends_with(&file_suffix) {
                let path = entry.path();
                rollout_paths()
                    .lock()
                    .expect("rollout path cache poisoned")
                    .insert(session_id.to_string(), path.clone());
                return Some(path);
            }
        }
    }

    None
}

fn cached_rollout_activity(session_id: &str, path: &Path) -> Result<RolloutActivity, String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("Failed to read Codex task metadata: {error}"))?;
    let file_length = metadata.len();
    let modified_at = system_time_to_unix_millis(metadata.modified().ok());

    let cached = rollout_activity_cache()
        .lock()
        .expect("rollout activity cache poisoned")
        .get(session_id)
        .cloned();
    if let Some(cached) = cached
        .as_ref()
        .filter(|cached| cached.file_length == file_length && cached.modified_at == modified_at)
    {
        return Ok(cached.activity.clone());
    }

    let mut activity = read_rollout_activity(path)?;
    if let Some(cached) = cached.filter(|cached| cached.file_length <= file_length) {
        let latest_terminals = std::mem::take(&mut activity.terminal_turns);
        activity.terminal_turns = cached.activity.terminal_turns;
        activity.terminal_turns.extend(latest_terminals);
    }
    activity.updated_at = modified_at;
    rollout_activity_cache()
        .lock()
        .expect("rollout activity cache poisoned")
        .insert(
            session_id.to_string(),
            CachedRolloutActivity {
                file_length,
                modified_at,
                activity: activity.clone(),
            },
        );
    Ok(activity)
}

fn read_rollout_activity(path: &Path) -> Result<RolloutActivity, String> {
    let mut file =
        fs::File::open(path).map_err(|error| format!("Failed to read Codex task: {error}"))?;
    let file_length = file
        .metadata()
        .map_err(|error| format!("Failed to read Codex task metadata: {error}"))?
        .len();
    let start = file_length.saturating_sub(MAX_ROLLOUT_TAIL_BYTES);
    file.seek(SeekFrom::Start(start))
        .map_err(|error| format!("Failed to seek Codex task: {error}"))?;

    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|error| format!("Failed to read Codex task: {error}"))?;
    let mut content = String::from_utf8_lossy(&bytes).into_owned();
    if start > 0 {
        if let Some(first_newline) = content.find('\n') {
            content.drain(..=first_newline);
        }
    }

    Ok(parse_rollout_activity(&content))
}

pub(super) fn parse_rollout_activity(content: &str) -> RolloutActivity {
    let mut activity = RolloutActivity {
        activity: "正在分析任务".to_string(),
        ..RolloutActivity::default()
    };
    let mut pending_calls = HashMap::<String, PendingCall>::new();

    for line in content.lines() {
        let Ok(record) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let Some(payload) = record.get("payload").and_then(Value::as_object) else {
            continue;
        };
        let payload_type = payload.get("type").and_then(Value::as_str).unwrap_or("");

        match (record.get("type").and_then(Value::as_str), payload_type) {
            (Some("event_msg"), "task_started") => {
                let terminal_turns = std::mem::take(&mut activity.terminal_turns);
                activity = RolloutActivity {
                    activity: "正在分析任务".to_string(),
                    terminal_turns,
                    ..RolloutActivity::default()
                };
                pending_calls.clear();
            }
            (Some("event_msg"), "task_complete") => {
                record_rollout_terminal(&mut activity, payload, "completed");
                activity.activity = "任务已完成".to_string();
                activity.attention.clear();
                pending_calls.clear();
            }
            (Some("event_msg"), "turn_aborted") => {
                record_rollout_terminal(&mut activity, payload, "failed");
                activity.activity = "任务已停止".to_string();
                activity.attention.clear();
                pending_calls.clear();
            }
            (Some("event_msg"), "agent_message") => {
                if payload.get("phase").and_then(Value::as_str) == Some("commentary") {
                    if let Some(message) = payload.get("message").and_then(Value::as_str) {
                        activity.activity = compact_activity_text(message);
                    }
                }
            }
            (Some("event_msg"), "agent_reasoning") | (Some("response_item"), "reasoning") => {
                activity.activity = if activity.current_step.is_empty() {
                    "正在分析下一步".to_string()
                } else {
                    activity.current_step.clone()
                };
            }
            (Some("response_item"), "function_call")
            | (Some("response_item"), "custom_tool_call") => {
                let name = payload
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let input = payload
                    .get("arguments")
                    .or_else(|| payload.get("input"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let call_id = payload
                    .get("call_id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();

                if name == "update_plan" {
                    apply_plan_update(&mut activity, &input);
                } else {
                    activity.activity = activity_label_for_call(&name, &input);
                }
                if !call_id.is_empty() {
                    pending_calls.insert(call_id, PendingCall { name, input });
                }
            }
            (Some("response_item"), "function_call_output")
            | (Some("response_item"), "custom_tool_call_output") => {
                if let Some(call_id) = payload.get("call_id").and_then(Value::as_str) {
                    pending_calls.remove(call_id);
                }
            }
            _ => {}
        }
    }

    if pending_calls
        .values()
        .any(|pending| pending.name == "request_user_input")
    {
        activity.attention = "input".to_string();
        activity.activity = "等待你回复 Codex".to_string();
    } else if pending_calls
        .values()
        .any(|pending| call_requires_approval(&pending.input))
    {
        activity.attention = "approval".to_string();
        activity.activity = "等待你批准 Codex 操作".to_string();
    }

    activity
}

fn record_rollout_terminal(
    activity: &mut RolloutActivity,
    payload: &Map<String, Value>,
    phase: &str,
) {
    let Some(turn_id) = payload
        .get("turn_id")
        .and_then(Value::as_str)
        .filter(|turn_id| !turn_id.is_empty())
    else {
        return;
    };
    let updated_at = payload
        .get("completed_at")
        .and_then(Value::as_i64)
        .unwrap_or_default()
        .saturating_mul(1000);
    activity.terminal_turns.insert(
        turn_id.to_string(),
        RolloutTerminal {
            phase: phase.to_string(),
            updated_at,
        },
    );
}

fn apply_plan_update(activity: &mut RolloutActivity, input: &str) {
    let Ok(arguments) = serde_json::from_str::<Value>(input) else {
        return;
    };
    let Some(plan) = arguments.get("plan").and_then(Value::as_array) else {
        return;
    };

    activity.plan_total = plan.len();
    activity.plan_completed = plan
        .iter()
        .filter(|step| step.get("status").and_then(Value::as_str) == Some("completed"))
        .count();
    activity.current_step = plan
        .iter()
        .find(|step| step.get("status").and_then(Value::as_str) == Some("in_progress"))
        .and_then(|step| step.get("step"))
        .and_then(Value::as_str)
        .map(compact_activity_text)
        .unwrap_or_default();
    if !activity.current_step.is_empty() {
        activity.activity = activity.current_step.clone();
    }
}

fn activity_label_for_call(name: &str, input: &str) -> String {
    let lower = input.to_ascii_lowercase();
    match name {
        "apply_patch" => "正在修改项目文件",
        "view_image" => "正在检查图像",
        "imagegen" | "image_gen" => "正在生成图像",
        "run" => "正在查询资料",
        "wait" | "write_stdin" => "正在等待任务结果",
        "exec" | "exec_command" if lower.contains("pnpm") && lower.contains("build") => {
            "正在构建并验证"
        }
        "exec" | "exec_command"
            if lower.contains("cargo test")
                || lower.contains("pnpm test")
                || lower.contains("npm test") =>
        {
            "正在运行测试"
        }
        "exec" | "exec_command"
            if lower.contains("get-content")
                || lower.contains("rg ")
                || lower.contains("git status") =>
        {
            "正在检查项目文件"
        }
        "exec" | "exec_command" => "正在执行命令",
        "request_user_input" => "等待你回复 Codex",
        _ => "正在调用工具",
    }
    .to_string()
}

fn call_requires_approval(input: &str) -> bool {
    input.contains("require_escalated")
        || input.contains("\"sandbox_permissions\":\"require_escalated\"")
}

fn compact_activity_text(value: &str) -> String {
    let compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut characters = compact.chars();
    let shortened = characters.by_ref().take(120).collect::<String>();
    if characters.next().is_some() {
        format!("{shortened}…")
    } else {
        shortened
    }
}

fn rollout_paths() -> &'static Mutex<HashMap<String, PathBuf>> {
    ROLLOUT_PATHS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn rollout_activity_cache() -> &'static Mutex<HashMap<String, CachedRolloutActivity>> {
    ROLLOUT_ACTIVITY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn system_time_to_unix_millis(time: Option<SystemTime>) -> i64 {
    time.and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}
