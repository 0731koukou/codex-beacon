import type { CodexPhase } from "../codex/types";
import codexBeaconIcon from "../../src-tauri/icons/128x128.png";

export function CodexMark({
  phase,
  compact = false,
}: {
  phase: CodexPhase;
  compact?: boolean;
}) {
  return (
    <span className={`codex-mark phase-${phase} ${compact ? "is-compact" : ""}`}>
      <img src={codexBeaconIcon} alt="" aria-hidden="true" />
    </span>
  );
}
