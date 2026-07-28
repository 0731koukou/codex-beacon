import type { CodexPhase, CodexSession } from "./types";

export const PHASE_LABELS: Record<CodexPhase, string> = {
  idle: "待命",
  running: "进行中",
  completed: "已完成",
  failed: "需处理",
  stale: "连接中断",
};

export const PHASE_HINTS: Record<CodexPhase, string> = {
  idle: "等待 Codex 任务",
  running: "Codex 正在执行",
  completed: "Codex 已完成任务",
  failed: "任务未正常完成",
  stale: "长时间未收到 Codex 更新",
};

export function normalizeSession(session: CodexSession): CodexSession {
  return {
    ...session,
    phase: normalizePhase(session.phase),
    prompt: session.prompt ?? "",
    cwd: session.cwd ?? "",
    model: session.model ?? "",
    lastAssistantMessage: session.lastAssistantMessage ?? "",
    startedAt: Number(session.startedAt) || 0,
    updatedAt: Number(session.updatedAt) || 0,
    activity: session.activity ?? "",
    attention:
      session.attention === "approval" || session.attention === "input"
        ? session.attention
        : "",
    planCompleted: Number(session.planCompleted) || 0,
    planTotal: Number(session.planTotal) || 0,
    currentStep: session.currentStep ?? "",
  };
}

export function compactText(value: string, limit: number) {
  const compact = value.replace(/\s+/g, " ").trim();
  if (!compact) {
    return "";
  }
  return compact.length > limit ? `${compact.slice(0, limit).trim()}…` : compact;
}

export function taskTitle(session?: CodexSession) {
  if (!session) {
    return "等待 Codex 任务";
  }
  return compactText(session.prompt, 72) || "Codex 任务";
}

export function projectName(cwd: string) {
  const normalized = cwd.replace(/[\\/]+$/, "");
  const segments = normalized.split(/[\\/]/).filter(Boolean);
  return segments[segments.length - 1] || "本地项目";
}

export function shortSessionId(sessionId: string) {
  if (!sessionId) {
    return "本地会话";
  }
  return sessionId.length > 8 ? sessionId.slice(-8) : sessionId;
}

export function sessionDuration(session: CodexSession, now: number) {
  const end = session.phase === "running" ? now : session.updatedAt;
  if (session.startedAt <= 0 || end <= 0) {
    return "刚刚";
  }
  return formatDuration(end - session.startedAt);
}

export function formatRelativeTime(timestamp: number, now: number) {
  if (timestamp <= 0) {
    return "暂无更新";
  }

  const elapsed = Math.max(0, now - timestamp);
  if (elapsed < 60_000) {
    return "刚刚";
  }
  if (elapsed < 3_600_000) {
    return `${Math.floor(elapsed / 60_000)}分钟前`;
  }
  if (elapsed < 86_400_000) {
    return `${Math.floor(elapsed / 3_600_000)}小时前`;
  }
  return new Intl.DateTimeFormat("zh-CN", {
    month: "numeric",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(timestamp);
}

export function sessionKey(session: CodexSession) {
  return `${session.sessionId}:${session.turnId}`;
}

export function moveSessionIndex(
  current: number,
  total: number,
  direction = 1,
) {
  if (total <= 1) {
    return 0;
  }
  const step = direction < 0 ? -1 : 1;
  return (current + step + total) % total;
}

export function shouldCompactIsland(
  sessions: CodexSession[],
  now: number,
  lingerMs = 6_000,
) {
  if (sessions.length === 0) {
    return true;
  }
  if (
    sessions.some(
      (session) =>
        session.phase !== "completed" || Boolean(session.attention),
    )
  ) {
    return false;
  }
  return (
    now - Math.max(...sessions.map((session) => session.updatedAt)) >=
    lingerMs
  );
}

function normalizePhase(phase: string): CodexPhase {
  if (
    phase === "running" ||
    phase === "completed" ||
    phase === "failed" ||
    phase === "stale"
  ) {
    return phase;
  }
  return "idle";
}

function formatDuration(milliseconds: number) {
  const totalSeconds = Math.max(0, Math.floor(milliseconds / 1000));
  if (totalSeconds < 60) {
    return `${totalSeconds}秒`;
  }

  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  if (minutes < 60) {
    return seconds > 0 ? `${minutes}分 ${seconds}秒` : `${minutes}分钟`;
  }

  const hours = Math.floor(minutes / 60);
  const remainingMinutes = minutes % 60;
  return remainingMinutes > 0 ? `${hours}时 ${remainingMinutes}分` : `${hours}小时`;
}
