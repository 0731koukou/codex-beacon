import {
  ArrowUpRight,
  Check,
  CircleAlert,
  Clock3,
  Folder,
  LoaderCircle,
  Sparkles,
} from "lucide-react";
import {
  compactText,
  formatRelativeTime,
  PHASE_HINTS,
  PHASE_LABELS,
  projectName,
  sessionDuration,
  sessionKey,
  taskTitle,
} from "../codex/presentation";
import type { CodexSession } from "../codex/types";

export function SessionWorkspace({
  sessions,
  currentSession,
  now,
  onSelect,
  onOpen,
}: {
  sessions: CodexSession[];
  currentSession: CodexSession;
  now: number;
  onSelect: (key: string) => void;
  onOpen: (session: CodexSession) => void;
}) {
  const response = compactText(currentSession.lastAssistantMessage, 260);
  const isRunning = currentSession.phase === "running";
  const needsAttention = currentSession.attention !== "";
  const activity =
    compactText(currentSession.activity, 180) ||
    compactText(currentSession.currentStep, 180) ||
    "Codex 正在处理任务";
  const badgeLabel =
    currentSession.attention === "approval"
      ? "待批准"
      : currentSession.attention === "input"
        ? "待回复"
        : PHASE_LABELS[currentSession.phase];

  return (
    <div className="session-workspace">
      <article className="current-session">
        <div className="session-eyebrow">
          <span
            className={`phase-badge phase-${currentSession.phase} attention-${currentSession.attention || "none"}`}
          >
            {currentSession.phase === "running" && !needsAttention && (
              <LoaderCircle className="spin" aria-hidden="true" size={12} />
            )}
            {needsAttention && (
              <CircleAlert aria-hidden="true" size={12} />
            )}
            {currentSession.phase === "completed" && (
              <Check aria-hidden="true" size={12} />
            )}
            {(currentSession.phase === "failed" ||
              currentSession.phase === "stale") && (
              <CircleAlert aria-hidden="true" size={12} />
            )}
            {badgeLabel}
          </span>
          <span className="session-age">
            {formatRelativeTime(currentSession.updatedAt, now)}
          </span>
        </div>

        <h1>{taskTitle(currentSession)}</h1>

        <div className="session-meta">
          <span title={currentSession.cwd || "本地项目"}>
            <Folder aria-hidden="true" size={13} />
            {projectName(currentSession.cwd)}
          </span>
          <span>
            <Clock3 aria-hidden="true" size={13} />
            {sessionDuration(currentSession, now)}
          </span>
          {currentSession.model && <span>{currentSession.model}</span>}
        </div>

        <div
          className={`response-surface ${isRunning ? "is-running" : ""} ${needsAttention ? "needs-attention" : ""}`}
        >
          {isRunning ? (
            <>
              <div className="activity-heading">
                <div className="thinking-row">
                  {needsAttention ? (
                    <CircleAlert aria-hidden="true" size={15} />
                  ) : (
                    <Sparkles aria-hidden="true" size={15} />
                  )}
                  <span>
                    {currentSession.attention === "approval"
                      ? "操作需要批准"
                      : currentSession.attention === "input"
                        ? "Codex 等待回复"
                        : "实时任务活动"}
                  </span>
                </div>
                <button
                  className={`thread-action ${needsAttention ? "is-attention" : ""}`}
                  type="button"
                  onClick={() => onOpen(currentSession)}
                >
                  {currentSession.attention === "approval"
                    ? "前往批准"
                    : "回到对话"}
                  <ArrowUpRight aria-hidden="true" size={12} />
                </button>
              </div>
              {currentSession.planTotal > 0 ? (
                <div className="plan-progress">
                  <span className="plan-track">
                    <span
                      style={{
                        width: `${
                          (currentSession.planCompleted /
                            currentSession.planTotal) *
                          100
                        }%`,
                      }}
                    />
                  </span>
                  <span>
                    计划 {currentSession.planCompleted}/
                    {currentSession.planTotal}
                  </span>
                </div>
              ) : (
                <div className="activity-track">
                  <span />
                </div>
              )}
              <p>{activity}</p>
            </>
          ) : response ? (
            <>
              <div className="response-heading">
                <span className="response-label">最后回复</span>
                <ThreadAction onClick={() => onOpen(currentSession)} />
              </div>
              <p>{response}</p>
            </>
          ) : (
            <>
              <div className="response-heading">
                <span className="response-label">任务状态</span>
                <ThreadAction onClick={() => onOpen(currentSession)} />
              </div>
              <p>{PHASE_HINTS[currentSession.phase]}</p>
            </>
          )}
        </div>
      </article>

      <aside className="session-list" aria-label="最近 Codex 任务">
        <div className="session-list-heading">
          <span>最近任务</span>
          <span>{sessions.length}</span>
        </div>
        <div className="session-list-items">
          {sessions.slice(0, 4).map((session) => {
            const active = sessionKey(session) === sessionKey(currentSession);
            return (
              <button
                className={`session-list-item ${active ? "is-selected" : ""}`}
                type="button"
                key={sessionKey(session)}
                onClick={() => onSelect(sessionKey(session))}
              >
                <span
                  className={`mini-state phase-${session.phase} attention-${session.attention || "none"}`}
                />
                <span className="list-item-copy">
                  <strong>{taskTitle(session)}</strong>
                  <span>
                    {projectName(session.cwd)} ·{" "}
                    {formatRelativeTime(session.updatedAt, now)}
                  </span>
                </span>
              </button>
            );
          })}
        </div>
      </aside>
    </div>
  );
}

function ThreadAction({ onClick }: { onClick: () => void }) {
  return (
    <button className="thread-action" type="button" onClick={onClick}>
      回到对话
      <ArrowUpRight aria-hidden="true" size={12} />
    </button>
  );
}
