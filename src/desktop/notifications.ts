import {
  isPermissionGranted,
  onAction,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { openCodexThread } from "../codex/api";
import type { CodexSession } from "../codex/types";

export type SessionNotificationState =
  | ""
  | "approval"
  | "input"
  | "completed"
  | "failed";

export function sessionNotificationState(
  session: CodexSession,
): SessionNotificationState {
  if (session.attention === "approval" || session.attention === "input") {
    return session.attention;
  }
  if (session.phase === "completed") {
    return "completed";
  }
  if (session.phase === "failed" || session.phase === "stale") {
    return "failed";
  }
  return "";
}

export async function listenForNotificationActions() {
  return onAction((notification) => {
    const sessionId = notification.extra?.sessionId;
    if (typeof sessionId === "string") {
      void openCodexThread(sessionId).catch(() => undefined);
    }
  });
}

export async function notifySession(
  session: CodexSession,
  state: Exclude<SessionNotificationState, "">,
) {
  const granted =
    (await isPermissionGranted()) ||
    (await requestPermission()) === "granted";
  if (!granted) {
    return;
  }

  const messages = {
    approval: ["Codex 等待批准", "需要你在 Codex 中批准操作"],
    input: ["Codex 等待回复", "需要你回复后任务才能继续"],
    completed: ["Codex 任务已完成", "点击查看 Codex 的最终回复"],
    failed: ["Codex 任务未完成", "任务失败、停止或已与 Codex 失联"],
  } as const;
  const [title, detail] = messages[state];

  sendNotification({
    title,
    body: detail,
    extra: { sessionId: session.sessionId },
    autoCancel: true,
  });
}
