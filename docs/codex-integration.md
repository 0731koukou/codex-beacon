# Codex Integration

Codex Beacon uses local Codex lifecycle hooks and read-only local rollout enrichment. It does not infer activity from a running process, CPU use, window titles, or timers.

## Installed files

The in-app “连接 Codex” action writes:

```text
%APPDATA%\app.codexbeacon.desktop\codex-beacon-event.ps1
%USERPROFILE%\.codex\hooks.json
%USERPROFILE%\.codex\config.toml
```

It installs two managed hooks:

- `UserPromptSubmit` calls the script with `running`.
- `Stop` calls the script with `completed`.

The installer is idempotent. It replaces only hook commands containing the
the current `codex-beacon-` signature or the previous product signature and
preserves every unrelated Codex hook.

The first existing `hooks.json` is copied once to:

```text
%USERPROFILE%\.codex\hooks.json.codex-beacon-backup
```

The installer also sets `hooks = true` in the existing `[features]` section of
`config.toml` without changing other settings.

## Trust and verification

Writing `hooks.json` only means the integration is configured. Codex still
reviews command hooks as a security boundary, and a running Codex process may
need to be restarted before it loads a newly written hook.

After installation:

1. Fully exit and reopen Codex.
2. Open `Settings → Hooks` (or use `/hooks` in the CLI).
3. Review the command and trust it only when it points to
   `%APPDATA%\app.codexbeacon.desktop\codex-beacon-event.ps1`.
4. Send a new Codex task.

Codex Beacon reports three distinct states:

- `未连接`: the managed files are missing.
- `等待审核`: files are configured, but no real Codex hook event has arrived.
- `已验证`: Codex executed the hook at least once.

The verification marker is written only by the event script:

```text
%APPDATA%\app.codexbeacon.desktop\codex-hook-verification.json
```

Installing or reinstalling the hook clears the previous marker so a changed
command cannot inherit a stale verified state.

## Status contract

The PowerShell script reads the Codex hook JSON payload from standard input and writes:

```text
%APPDATA%\app.codexbeacon.desktop\codex-status.json
```

Example:

```json
{
  "sessions": [
    {
      "sessionId": "019f...",
      "turnId": "019f...",
      "phase": "completed",
      "prompt": "Polish the Codex task island",
      "cwd": "D:\\Projects\\Codex-Beacon",
      "model": "gpt-5.6",
      "lastAssistantMessage": "The task is complete.",
      "startedAt": 1784776200000,
      "updatedAt": 1784776235000
    }
  ],
  "updatedAt": 1784776235000
}
```

Sessions are keyed by `sessionId + turnId`, ordered by update time, and capped at 6 entries. Writes use a named mutex and atomic file replacement so concurrent Codex tasks do not corrupt the state file.

Supported phases:

- `running`
- `completed`
- `failed`
- `stale`

The current Codex hook set does not emit a separate failure event. Codex Beacon retains `failed` for script tests and future event support. A `running` item becomes `stale` after 2 hours without an update.

## Live activity

For a running session, Codex Beacon locates the matching file under:

```text
%USERPROFILE%\.codex\sessions
```

It reads only the tail of that rollout and caches unchanged results. The UI can
then show:

- the latest user-facing Codex activity;
- command, file-edit, test, build, web and image activity labels;
- completed and total plan-step counts when Codex published a plan;
- a pending user-input or elevated-operation signal.

The plan-step count is not presented as an overall completion percentage.
Codex Beacon never writes to rollout files.

## Approval and conversation navigation

Codex Desktop owns the approval request/response channel. Its running App Server
is a private child process connected to the Desktop app over standard input and
output, so Codex Beacon does not attach to it or submit approval decisions.

When the rollout shows an unresolved elevated tool call, the island displays a
local “待批准” state. Clicking “前往批准” opens:

```text
codex://threads/{sessionId}
```

The same deep link powers “回到对话” for running and completed tasks. The task ID
is validated as a UUID before Windows is asked to open the link.

## Manual install

Close Codex Beacon, then run:

```powershell
codex-beacon.exe --install-codex-hooks
```

If Codex asks to review or trust the hook, verify that the command points to `%APPDATA%\app.codexbeacon.desktop\codex-beacon-event.ps1`.
Restart Codex after manual installation so the new hook configuration is loaded.

## Manual smoke test

```powershell
$script = Join-Path $env:APPDATA 'app.codexbeacon.desktop\codex-beacon-event.ps1'
$status = Join-Path $env:APPDATA 'app.codexbeacon.desktop\codex-status.json'

'{"session_id":"smoke-session","turn_id":"smoke-turn","cwd":"D:\\Demo","model":"gpt-5.6","prompt":"Verify the island"}' |
  powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File $script running

'{"session_id":"smoke-session","turn_id":"smoke-turn","cwd":"D:\\Demo","model":"gpt-5.6","last_assistant_message":"Verification complete."}' |
  powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File $script completed

Get-Content -LiteralPath $status -Raw
```

Expected result:

1. The collapsed island shows “Verify the island” as running.
2. The expanded island shows project `Demo`, model `gpt-5.6`, and elapsed time.
3. After the second command, the task changes to completed and shows “Verification complete.”
