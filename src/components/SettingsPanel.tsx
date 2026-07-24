import { Download, Power, RotateCw, Trash2 } from "lucide-react";
import { compactText } from "../codex/presentation";
import type { CodexIntegrationStatus } from "../codex/types";

export function SettingsPanel({
  integration,
  launchAtStartup,
  busyAction,
  notice,
  currentVersion,
  latestVersion,
  updateChecked,
  onInstall,
  onToggleStartup,
  onClear,
  onOpenUpdate,
}: {
  integration: CodexIntegrationStatus;
  launchAtStartup: boolean;
  busyAction: string;
  notice: string;
  currentVersion: string;
  latestVersion: string;
  updateChecked: boolean;
  onInstall: () => void;
  onToggleStartup: () => void;
  onClear: () => void;
  onOpenUpdate: () => void;
}) {
  return (
    <div className="settings-panel">
      <div className="settings-heading">
        <div>
          <span>系统设置</span>
          <h1>只保留必要开关</h1>
        </div>
        <span
          className={`connection-chip ${
            integration.verified
              ? "is-ok"
              : integration.configured
                ? "is-review"
                : ""
          }`}
        >
          <span />
          {integration.verified
            ? "Codex 已验证"
            : integration.configured
              ? "等待 Codex 审核"
              : "Codex 未连接"}
        </span>
      </div>

      <div className="settings-rows">
        <div className="settings-row">
          <span className="settings-row-icon">
            <Power aria-hidden="true" size={16} />
          </span>
          <div className="settings-row-copy">
            <strong>开机启动</strong>
            <span>登录 Windows 后自动显示灵动岛</span>
          </div>
          <button
            className={`switch ${launchAtStartup ? "is-on" : ""}`}
            type="button"
            role="switch"
            aria-checked={launchAtStartup}
            aria-label="切换开机启动"
            disabled={busyAction === "startup"}
            onClick={onToggleStartup}
          >
            <span />
          </button>
        </div>

        {latestVersion && (
          <div className="settings-row update-row">
            <span className="settings-row-icon update">
              <Download aria-hidden="true" size={16} />
            </span>
            <div className="settings-row-copy">
              <strong>发现 Codex Beacon v{latestVersion}</strong>
              <span>已发布新版本，点击前往 GitHub 下载</span>
            </div>
            <button
              className="secondary-action update"
              type="button"
              onClick={onOpenUpdate}
            >
              查看
            </button>
          </div>
        )}

        <div className="settings-row">
          <span className="settings-row-icon">
            <RotateCw aria-hidden="true" size={16} />
          </span>
          <div className="settings-row-copy">
            <strong>
              {integration.configured ? "重装 Codex Hook" : "安装 Codex Hook"}
            </strong>
            <span>
              {integration.configured && !integration.verified
                ? "安装已完成，仍需在 Codex 中审核信任"
                : "只更新 Codex Beacon Hook，不改动其他 Hook"}
            </span>
          </div>
          <button
            className="secondary-action"
            type="button"
            disabled={busyAction === "install"}
            onClick={onInstall}
          >
            {busyAction === "install" ? "处理中" : "执行"}
          </button>
        </div>

        <div className="settings-row">
          <span className="settings-row-icon danger">
            <Trash2 aria-hidden="true" size={16} />
          </span>
          <div className="settings-row-copy">
            <strong>清除最近任务</strong>
            <span>只删除本地状态，不影响 Codex 会话</span>
          </div>
          <button
            className="secondary-action danger"
            type="button"
            disabled={busyAction === "clear"}
            onClick={onClear}
          >
            {busyAction === "clear" ? "清除中" : "清除"}
          </button>
        </div>
      </div>

      <div className="settings-footnote">
        <span>
          {notice ||
            (currentVersion
              ? `Codex Beacon v${currentVersion} · ${
                  updateChecked ? "已检查更新" : "正在检查更新"
                }`
              : "任务数据仅保存在这台 Windows 设备上")}
        </span>
        {integration.hooksPath && (
          <span title={integration.hooksPath}>
            {compactText(integration.hooksPath, 58)}
          </span>
        )}
      </div>
    </div>
  );
}
