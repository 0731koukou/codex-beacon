import { invoke } from "@tauri-apps/api/core";
import type { IslandMode } from "../codex/types";

export function setIslandInteraction(mode: IslandMode) {
  return invoke<void>("set_island_interaction", { mode });
}

export function showReadyIsland() {
  return invoke<void>("show_ready_island");
}

export function minimizeIsland() {
  return invoke<void>("minimize_island");
}

export function getLaunchAtStartup() {
  return invoke<boolean>("get_launch_at_startup");
}

export function setLaunchAtStartup(enabled: boolean) {
  return invoke<void>("set_launch_at_startup", { enabled });
}

export function openLatestRelease() {
  return invoke<void>("open_latest_release");
}
