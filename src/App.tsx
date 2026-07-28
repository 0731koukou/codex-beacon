import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
} from "react";
import { getVersion } from "@tauri-apps/api/app";
import { PhysicalPosition } from "@tauri-apps/api/dpi";
import {
  availableMonitors,
  getCurrentWindow,
} from "@tauri-apps/api/window";
import {
  ChevronDown,
  GripHorizontal,
  Minus,
  Settings2,
  X,
} from "lucide-react";
import {
  clearCodexStatus,
  getCodexIntegrationStatus,
  getCodexStatus,
  installCodexHooks,
  openCodexThread,
} from "./codex/api";
import {
  moveSessionIndex,
  normalizeSession,
  PHASE_HINTS,
  PHASE_LABELS,
  projectName,
  sessionDuration,
  sessionKey,
  shouldCompactIsland,
  shortSessionId,
  taskTitle,
} from "./codex/presentation";
import {
  EMPTY_INTEGRATION,
  EMPTY_STATUS,
  type CodexIntegrationStatus,
  type CodexSession,
  type CodexStatusSnapshot,
  type IslandMode,
} from "./codex/types";
import { CodexMark } from "./components/CodexMark";
import { EmptyState } from "./components/EmptyState";
import { SessionWorkspace } from "./components/SessionWorkspace";
import { SettingsPanel } from "./components/SettingsPanel";
import {
  getLaunchAtStartup,
  minimizeIsland,
  openLatestRelease,
  setIslandInteraction,
  setLaunchAtStartup,
  showReadyIsland,
  snapIslandToEdge,
} from "./desktop/api";
import {
  listenForNotificationActions,
  notifySession,
  sessionNotificationState,
  type SessionNotificationState,
} from "./desktop/notifications";
import {
  DEFAULT_BACKGROUND_OPACITY,
  isWindowPositionVisible,
  normalizeBackgroundOpacity,
  readBackgroundOpacity,
  readWindowPosition,
  saveBackgroundOpacity,
  saveWindowPosition,
  toScreenRect,
} from "./desktop/preferences";
import { checkForUpdate } from "./update";
import "./App.css";

function initialBackgroundOpacity() {
  try {
    return readBackgroundOpacity(window.localStorage);
  } catch {
    return DEFAULT_BACKGROUND_OPACITY;
  }
}

async function restoreWindowPosition() {
  if (!("__TAURI_INTERNALS__" in window)) {
    return;
  }

  let position;
  try {
    position = readWindowPosition(window.localStorage);
  } catch {
    return;
  }
  if (!position) {
    return;
  }

  const appWindow = getCurrentWindow();
  const [size, monitors] = await Promise.all([
    appWindow.outerSize(),
    availableMonitors(),
  ]);
  const workAreas = monitors.map((monitor) =>
    toScreenRect(monitor.workArea),
  );
  if (isWindowPositionVisible(position, size, workAreas)) {
    await appWindow.setPosition(
      new PhysicalPosition(position.x, position.y),
    );
  }
}

function App() {
  const [mode, setMode] = useState<IslandMode>("collapsed");
  const [status, setStatus] = useState<CodexStatusSnapshot>(EMPTY_STATUS);
  const [integration, setIntegration] =
    useState<CodexIntegrationStatus>(EMPTY_INTEGRATION);
  const [selectedKey, setSelectedKey] = useState("");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [launchAtStartup, setLaunchAtStartupState] = useState(false);
  const [busyAction, setBusyAction] = useState("");
  const [notice, setNotice] = useState("");
  const [now, setNow] = useState(Date.now());
  const [currentVersion, setCurrentVersion] = useState("");
  const [latestVersion, setLatestVersion] = useState("");
  const [updateChecked, setUpdateChecked] = useState(false);
  const [backgroundOpacity, setBackgroundOpacity] = useState(
    initialBackgroundOpacity,
  );
  const [collapsedSessionIndex, setCollapsedSessionIndex] = useState(0);
  const [titleRotationPaused, setTitleRotationPaused] = useState(false);
  const notificationStates = useRef(
    new Map<string, SessionNotificationState>(),
  );
  const notificationsReady = useRef(false);
  const lastWheelSwitchAt = useRef(0);

  const sessions = useMemo(
    () => status.sessions.map(normalizeSession),
    [status.sessions],
  );

  const currentSession = useMemo(() => {
    if (selectedKey) {
      const selected = sessions.find(
        (session) => sessionKey(session) === selectedKey,
      );
      if (selected) {
        return selected;
      }
    }
    return sessions[0];
  }, [selectedKey, sessions]);

  const isIdleCompact = shouldCompactIsland(sessions, now);
  const collapsedDisplayIndex =
    sessions.length > 0 ? collapsedSessionIndex % sessions.length : 0;
  const collapsedSession =
    !isIdleCompact && sessions.length > 0
      ? sessions[collapsedDisplayIndex]
      : undefined;

  const runningCount = sessions.filter(
    (session) => session.phase === "running",
  ).length;
  const currentPhase = currentSession?.phase ?? "idle";
  const currentAttention = currentSession?.attention ?? "";
  const collapsedPhase = collapsedSession?.phase ?? "idle";
  const collapsedAttention = collapsedSession?.attention ?? "";

  const refreshStatus = useCallback(async () => {
    try {
      const next = await getCodexStatus();
      setStatus(next);
    } catch {
      setStatus((previous) => previous);
    }
  }, []);

  const refreshIntegration = useCallback(async () => {
    try {
      const next = await getCodexIntegrationStatus();
      setIntegration(next);
    } catch {
      setIntegration((previous) => previous);
    }
  }, []);

  const changeMode = useCallback(async (nextMode: IslandMode) => {
    setMode(nextMode);
    setTitleRotationPaused(false);
    if (nextMode === "collapsed") {
      setSettingsOpen(false);
    }
  }, []);

  useEffect(() => {
    void setIslandInteraction(mode, !isIdleCompact).catch(
      () => undefined,
    );
  }, [isIdleCompact, mode]);

  useEffect(() => {
    setCollapsedSessionIndex((current) =>
      sessions.length > 0 ? current % sessions.length : 0,
    );
    if (
      mode !== "collapsed" ||
      isIdleCompact ||
      titleRotationPaused ||
      sessions.length < 2
    ) {
      return;
    }
    const timer = window.setInterval(() => {
      setCollapsedSessionIndex((current) =>
        moveSessionIndex(current, sessions.length),
      );
    }, 4_200);
    return () => window.clearInterval(timer);
  }, [isIdleCompact, mode, sessions.length, titleRotationPaused]);

  useEffect(() => {
    void Promise.all([
      refreshStatus(),
      refreshIntegration(),
      getLaunchAtStartup()
        .then(setLaunchAtStartupState)
        .catch(() => undefined),
      restoreWindowPosition().catch(() => undefined),
    ]).finally(() => {
      void showReadyIsland().catch(() => undefined);
    });

    const statusTimer = window.setInterval(() => {
      void refreshStatus();
    }, 900);
    const integrationTimer = window.setInterval(() => {
      void refreshIntegration();
    }, 1_500);
    const clockTimer = window.setInterval(() => setNow(Date.now()), 1000);

    return () => {
      window.clearInterval(statusTimer);
      window.clearInterval(integrationTimer);
      window.clearInterval(clockTimer);
    };
  }, [refreshIntegration, refreshStatus]);

  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window)) {
      return;
    }

    let active = true;
    let unlisten: (() => void) | undefined;
    let snapTimer: number | undefined;
    void getCurrentWindow()
      .onMoved(({ payload }) => {
        try {
          saveWindowPosition(window.localStorage, payload);
        } catch {
          // The window remains draggable when storage is unavailable.
        }
        window.clearTimeout(snapTimer);
        snapTimer = window.setTimeout(() => {
          void snapIslandToEdge().catch(() => undefined);
        }, 180);
      })
      .then((next) => {
        if (active) {
          unlisten = next;
        } else {
          next();
        }
      })
      .catch(() => undefined);

    return () => {
      active = false;
      window.clearTimeout(snapTimer);
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    void getVersion()
      .then((version) => {
        setCurrentVersion(version);
        return checkForUpdate(version);
      })
      .then(setLatestVersion)
      .catch(() => undefined)
      .finally(() => setUpdateChecked(true));
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") {
        return;
      }
      if (settingsOpen) {
        setSettingsOpen(false);
      } else if (mode === "expanded") {
        void changeMode("collapsed");
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [changeMode, mode, settingsOpen]);

  useEffect(() => {
    if (
      mode !== "expanded" ||
      !("__TAURI_INTERNALS__" in window)
    ) {
      return;
    }

    let active = true;
    let unlisten: (() => void) | undefined;
    let collapseTimer: number | undefined;
    void getCurrentWindow()
      .onFocusChanged(({ payload: focused }) => {
        window.clearTimeout(collapseTimer);
        if (!focused) {
          collapseTimer = window.setTimeout(() => {
            void changeMode("collapsed");
          }, 150);
        }
      })
      .then((next) => {
        if (active) {
          unlisten = next;
        } else {
          next();
        }
      })
      .catch(() => undefined);

    return () => {
      active = false;
      window.clearTimeout(collapseTimer);
      unlisten?.();
    };
  }, [changeMode, mode]);

  useEffect(() => {
    let active = true;
    let listener:
      | Awaited<ReturnType<typeof listenForNotificationActions>>
      | undefined;

    void listenForNotificationActions()
      .then((next) => {
        if (active) {
          listener = next;
        } else {
          void next.unregister();
        }
      })
      .catch(() => undefined);

    return () => {
      active = false;
      void listener?.unregister();
    };
  }, []);

  useEffect(() => {
    const nextStates = new Map<string, SessionNotificationState>();
    for (const session of sessions) {
      nextStates.set(session.sessionId, sessionNotificationState(session));
    }

    if (notificationsReady.current) {
      for (const session of sessions) {
        const next = nextStates.get(session.sessionId) ?? "";
        const previous = notificationStates.current.get(session.sessionId);
        if (next && next !== previous) {
          void notifySession(session, next).catch(() => undefined);
        }
      }
    } else {
      notificationsReady.current = true;
    }

    notificationStates.current = nextStates;
  }, [sessions]);

  const installHooks = async () => {
    setBusyAction("install");
    setNotice("");
    try {
      const result = await installCodexHooks();
      setIntegration({
        configured: result.configured,
        verified: result.verified,
        hooksPath: result.hooksPath,
        scriptPath: result.scriptPath,
        verificationPath: result.verificationPath,
      });
      setNotice(
        result.verified
          ? "Hook 已重新安装，Codex 连接保持有效"
          : "Hook 已安装；重启 Codex 并在 Hooks 中审核信任",
      );
      await refreshStatus();
    } catch (error) {
      setNotice(`连接失败：${String(error)}`);
    } finally {
      setBusyAction("");
    }
  };

  const clearStatus = async () => {
    setBusyAction("clear");
    setNotice("");
    try {
      const next = await clearCodexStatus();
      setStatus(next);
      setSelectedKey("");
      setNotice("最近任务已清除");
    } catch (error) {
      setNotice(`清除失败：${String(error)}`);
    } finally {
      setBusyAction("");
    }
  };

  const toggleLaunchAtStartup = async () => {
    const next = !launchAtStartup;
    setLaunchAtStartupState(next);
    setBusyAction("startup");
    setNotice("");
    try {
      await setLaunchAtStartup(next);
    } catch (error) {
      setLaunchAtStartupState(!next);
      setNotice(`开机启动设置失败：${String(error)}`);
    } finally {
      setBusyAction("");
    }
  };

  const openUpdate = async () => {
    try {
      await openLatestRelease();
    } catch (error) {
      setNotice(`无法打开下载页面：${String(error)}`);
    }
  };

  const openThread = async (session: CodexSession) => {
    try {
      await openCodexThread(session.sessionId);
      await changeMode("collapsed");
    } catch (error) {
      setNotice(`无法打开 Codex 对话：${String(error)}`);
    }
  };

  const startWindowDrag = (event: React.MouseEvent<HTMLElement>) => {
    if (
      event.button !== 0 ||
      !("__TAURI_INTERNALS__" in window) ||
      (event.target as HTMLElement).closest("button") !==
        event.currentTarget.closest("button")
    ) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    void getCurrentWindow().startDragging().catch(() => undefined);
  };

  const changeBackgroundOpacity = (value: number) => {
    const next = normalizeBackgroundOpacity(value);
    setBackgroundOpacity(next);
    try {
      saveBackgroundOpacity(window.localStorage, next);
    } catch {
      // The current setting still applies for this run.
    }
  };

  const switchCollapsedSession = (direction: number) => {
    setCollapsedSessionIndex((current) =>
      moveSessionIndex(current, sessions.length, direction),
    );
  };

  const switchCollapsedSessionByWheel = (
    event: React.WheelEvent<HTMLElement>,
  ) => {
    if (sessions.length < 2 || Math.abs(event.deltaY) < 4) {
      return;
    }
    const switchedAt = performance.now();
    if (switchedAt - lastWheelSwitchAt.current < 280) {
      return;
    }
    lastWheelSwitchAt.current = switchedAt;
    switchCollapsedSession(event.deltaY > 0 ? 1 : -1);
  };

  const collapsedSubtitle = collapsedSession
    ? projectName(collapsedSession.cwd)
    : integration.verified
      ? "Codex Hook 已验证"
      : integration.configured
        ? "Hook 已安装，等待 Codex 审核"
        : "尚未连接 Codex";
  const compactStateLabel =
    collapsedAttention === "approval"
      ? "等待批准"
      : collapsedAttention === "input"
        ? "等待回复"
        : collapsedPhase === "completed"
          ? "任务完成"
          : PHASE_LABELS[collapsedPhase];
  const integrationPhase = integration.verified
    ? "verified"
    : integration.configured
      ? "review"
      : "setup";
  const emptyTitle = integration.verified
    ? "等待 Codex 任务"
    : integration.configured
      ? "完成 Codex Hook 审核"
      : "连接 Codex";
  const headerLabel = currentSession
    ? currentAttention === "approval"
      ? "等待批准"
      : currentAttention === "input"
        ? "等待回复"
        : runningCount > 1
          ? `${runningCount} 个任务进行中`
          : PHASE_LABELS[currentPhase]
    : integration.verified
      ? "待命"
      : integration.configured
        ? "待审核"
        : "未连接";
  const footerHint = currentSession
    ? currentAttention === "approval"
      ? "需要你在 Codex 中批准操作"
      : currentAttention === "input"
        ? "需要你在 Codex 中回复"
        : PHASE_HINTS[currentPhase]
    : integration.verified
      ? "等待 Codex 任务"
      : integration.configured
        ? "重启 Codex 并信任 Codex Beacon Hook"
        : "先安装本地 Codex Hook";
  const displayPhase = mode === "collapsed" ? collapsedPhase : currentPhase;
  const displayAttention =
    mode === "collapsed" ? collapsedAttention : currentAttention;

  return (
    <main
      className={`island-stage mode-${mode} ${isIdleCompact ? "is-idle" : ""}`}
      data-phase={displayPhase}
      data-integration={integrationPhase}
      data-attention={displayAttention}
      aria-live="polite"
      style={
        {
          "--surface-opacity": backgroundOpacity / 100,
        } as CSSProperties
      }
    >
      <section
        className="collapsed-shell"
        aria-hidden={mode !== "collapsed"}
        onMouseEnter={() => setTitleRotationPaused(true)}
        onMouseLeave={() => setTitleRotationPaused(false)}
        onFocus={() => setTitleRotationPaused(true)}
        onBlur={(event) => {
          if (!event.currentTarget.contains(event.relatedTarget)) {
            setTitleRotationPaused(false);
          }
        }}
        onWheel={switchCollapsedSessionByWheel}
      >
        <button
          className="collapsed-drag-handle"
          type="button"
          title="拖动悬浮窗"
          aria-label="拖动悬浮窗"
          tabIndex={mode === "collapsed" ? 0 : -1}
          onMouseDown={startWindowDrag}
          onClick={(event) => event.stopPropagation()}
        >
          <GripHorizontal aria-hidden="true" size={16} />
        </button>
        <button
          className="collapsed-main"
          type="button"
          onClick={() => {
            if (collapsedSession) {
              setSelectedKey(sessionKey(collapsedSession));
            }
            void changeMode("expanded");
          }}
          onKeyDown={(event) => {
            if (sessions.length < 2) {
              return;
            }
            if (event.key === "ArrowUp" || event.key === "ArrowLeft") {
              event.preventDefault();
              switchCollapsedSession(-1);
            } else if (
              event.key === "ArrowDown" ||
              event.key === "ArrowRight"
            ) {
              event.preventDefault();
              switchCollapsedSession(1);
            }
          }}
          tabIndex={mode === "collapsed" ? 0 : -1}
          aria-label={
            sessions.length > 1
              ? `展开 Codex 灵动岛，当前第 ${collapsedDisplayIndex + 1} 个任务，共 ${sessions.length} 个，可用滚轮或方向键切换`
              : "展开 Codex 灵动岛"
          }
        >
          <CodexMark phase={collapsedPhase} />
          <span className="collapsed-copy">
            <span className="collapsed-title-row">
              {collapsedSession && <span className="collapsed-task-dot" />}
              <span className="collapsed-title">
                <span
                  className="collapsed-title-text"
                  key={
                    collapsedSession
                      ? sessionKey(collapsedSession)
                      : "idle"
                  }
                >
                  {collapsedSession ? taskTitle(collapsedSession) : emptyTitle}
                </span>
              </span>
            </span>
            <span
              className={`collapsed-subtitle-row ${collapsedSession ? "has-status" : ""}`}
            >
              <span className="collapsed-subtitle">{collapsedSubtitle}</span>
              {collapsedSession && (
                <span className="collapsed-time">
                  {sessionDuration(collapsedSession, now)}
                </span>
              )}
            </span>
          </span>
          <span className="collapsed-status-group">
            {collapsedSession && sessions.length > 1 && (
              <span
                className="collapsed-session-index"
                title="滚轮或方向键切换任务"
              >
                {collapsedDisplayIndex + 1}/{sessions.length}
              </span>
            )}
            {collapsedSession ? (
              <span
                className={`collapsed-status phase-${collapsedPhase} attention-${collapsedAttention || "none"}`}
              >
                {compactStateLabel}
              </span>
            ) : (
              <span className={`collapsed-ready state-${integrationPhase}`}>
                {integration.verified
                  ? "READY"
                  : integration.configured
                    ? "REVIEW"
                    : "SETUP"}
              </span>
            )}
            <span className="expand-affordance">
              <ChevronDown aria-hidden="true" size={17} />
            </span>
          </span>
        </button>

        <button
          className="collapsed-minimize icon-button"
          type="button"
          title="隐藏到系统托盘"
          aria-label="隐藏到系统托盘"
          tabIndex={mode === "collapsed" ? 0 : -1}
          onClick={() => void minimizeIsland().catch(() => undefined)}
        >
          <Minus aria-hidden="true" size={16} />
        </button>
      </section>

      <section className="expanded-shell" aria-hidden={mode !== "expanded"}>
        <header
          className="island-header"
          title="拖动悬浮窗"
          onMouseDown={startWindowDrag}
        >
          <div className="brand-lockup">
            <CodexMark phase={currentPhase} compact />
            <div>
              <strong>Codex</strong>
              <span>BEACON</span>
            </div>
          </div>

          <div
            className={`header-state phase-${currentPhase} integration-${integrationPhase} attention-${currentAttention || "none"}`}
          >
            <span className="state-dot" />
            {headerLabel}
          </div>

          <div className="header-actions">
            <button
              className={`icon-button ${settingsOpen ? "is-active" : ""}`}
              type="button"
              title="设置"
              aria-label="打开设置"
              tabIndex={mode === "expanded" ? 0 : -1}
              onClick={() => setSettingsOpen((open) => !open)}
            >
              {settingsOpen ? (
                <X aria-hidden="true" size={16} />
              ) : (
                <Settings2 aria-hidden="true" size={16} />
              )}
            </button>
            <button
              className="icon-button"
              type="button"
              title="收起"
              aria-label="收起灵动岛"
              tabIndex={mode === "expanded" ? 0 : -1}
              onClick={() => void changeMode("collapsed")}
            >
              <ChevronDown aria-hidden="true" size={17} />
            </button>
            <button
              className="icon-button"
              type="button"
              title="隐藏到系统托盘"
              aria-label="隐藏到系统托盘"
              tabIndex={mode === "expanded" ? 0 : -1}
              onClick={() =>
                void minimizeIsland().catch(() => undefined)
              }
            >
              <Minus aria-hidden="true" size={16} />
            </button>
          </div>
        </header>

        <div className="island-body">
          {settingsOpen ? (
            <SettingsPanel
              integration={integration}
              launchAtStartup={launchAtStartup}
              busyAction={busyAction}
              notice={notice}
              currentVersion={currentVersion}
              latestVersion={latestVersion}
              updateChecked={updateChecked}
              backgroundOpacity={backgroundOpacity}
              onInstall={() => void installHooks()}
              onToggleStartup={() => void toggleLaunchAtStartup()}
              onClear={() => void clearStatus()}
              onOpenUpdate={() => void openUpdate()}
              onBackgroundOpacityChange={changeBackgroundOpacity}
            />
          ) : currentSession ? (
            <SessionWorkspace
              sessions={sessions}
              currentSession={currentSession}
              now={now}
              onSelect={setSelectedKey}
              onOpen={(session) => void openThread(session)}
            />
          ) : (
            <EmptyState
              integration={integration}
              busy={busyAction === "install"}
              notice={notice}
              onInstall={() => void installHooks()}
            />
          )}
        </div>

        <footer className="island-footer">
          <span className="footer-signal">
            <span className={`signal-line phase-${currentPhase}`} />
            {footerHint}
          </span>
          <span>
            {currentSession
              ? `会话 ${shortSessionId(currentSession.sessionId)}`
              : "仅在本机读取 Codex Hook"}
          </span>
        </footer>
      </section>
    </main>
  );
}

export default App;
