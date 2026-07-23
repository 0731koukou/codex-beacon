import { invoke } from "@tauri-apps/api/core";
import type {
  CodexHooksInstallResult,
  CodexIntegrationStatus,
  CodexStatusSnapshot,
} from "./types";

export function getCodexStatus() {
  return invoke<CodexStatusSnapshot>("get_codex_status");
}

export function getCodexIntegrationStatus() {
  return invoke<CodexIntegrationStatus>("get_codex_integration_status");
}

export function installCodexHooks() {
  return invoke<CodexHooksInstallResult>("install_codex_hooks");
}

export function clearCodexStatus() {
  return invoke<CodexStatusSnapshot>("clear_codex_status");
}

export function openCodexThread(sessionId: string) {
  return invoke<void>("open_codex_thread", { sessionId });
}
