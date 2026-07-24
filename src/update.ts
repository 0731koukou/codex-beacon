const LATEST_RELEASE_API =
  "https://api.github.com/repos/0731koukou/codex-beacon/releases/latest";

function versionParts(value: string) {
  const match = value.match(/^v?(\d+)\.(\d+)\.(\d+)(?:$|-)/);
  return match?.slice(1).map(Number);
}

export function isNewerVersion(latest: string, current: string) {
  const latestParts = versionParts(latest);
  const currentParts = versionParts(current);
  if (!latestParts || !currentParts) {
    return false;
  }
  return latestParts.some(
    (part, index) =>
      part > currentParts[index] &&
      latestParts.slice(0, index).every((value, i) => value === currentParts[i]),
  );
}

export async function checkForUpdate(currentVersion: string) {
  const response = await fetch(LATEST_RELEASE_API, {
    headers: { Accept: "application/vnd.github+json" },
  });
  if (!response.ok) {
    throw new Error(`GitHub Release check failed: ${response.status}`);
  }
  const release = (await response.json()) as { tag_name?: unknown };
  const latestVersion =
    typeof release.tag_name === "string" ? release.tag_name.replace(/^v/, "") : "";
  return isNewerVersion(latestVersion, currentVersion) ? latestVersion : "";
}
