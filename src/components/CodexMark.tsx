import { Bot } from "lucide-react";
import type { CodexPhase } from "../codex/types";

export function CodexMark({
  phase,
  compact = false,
}: {
  phase: CodexPhase;
  compact?: boolean;
}) {
  return (
    <span className={`codex-mark phase-${phase} ${compact ? "is-compact" : ""}`}>
      <Bot aria-hidden="true" size={compact ? 17 : 21} strokeWidth={1.8} />
      <span className="mark-pulse" />
    </span>
  );
}
