import { Bot, LoaderCircle, Wrench } from "lucide-react";
import type { CodexIntegrationStatus } from "../codex/types";

export function EmptyState({
  integration,
  busy,
  notice,
  onInstall,
}: {
  integration: CodexIntegrationStatus;
  busy: boolean;
  notice: string;
  onInstall: () => void;
}) {
  const needsReview = integration.configured && !integration.verified;

  return (
    <div className={`empty-state ${needsReview ? "is-review" : ""}`}>
      <div className="empty-visual">
        <Bot aria-hidden="true" size={28} strokeWidth={1.6} />
        <span />
      </div>
      <div className="empty-copy">
        <span className="empty-kicker">
          {integration.verified
            ? "CODEX READY"
            : needsReview
              ? "HOOK REVIEW REQUIRED"
              : "ONE-TIME SETUP"}
        </span>
        <h1>
          {integration.verified
            ? "等待下一条 Codex 任务"
            : needsReview
              ? "还差一步：在 Codex 中信任 Hook"
              : "先连接 Codex"}
        </h1>
        <p>
          {integration.verified
            ? "在 Codex 中开始任务后，标题、项目、耗时和完成回复会自动同步到这里。"
            : needsReview
              ? "完全退出并重新打开 Codex，进入“设置 → Hooks”，审核并信任 Codex Beacon；然后发送一条新任务完成验证。"
              : "安装本地 Hook 后，灵动岛才能识别真实 Codex 会话。不会上传任务内容。"}
        </p>
        {!integration.configured && (
          <button
            className="primary-action"
            type="button"
            disabled={busy}
            onClick={onInstall}
          >
            {busy ? (
              <LoaderCircle className="spin" aria-hidden="true" size={15} />
            ) : (
              <Wrench aria-hidden="true" size={15} />
            )}
            {busy ? "正在连接" : "连接 Codex"}
          </button>
        )}
        {notice && <span className="inline-notice">{notice}</span>}
      </div>
    </div>
  );
}
