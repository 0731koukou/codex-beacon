export type IslandMode = "collapsed" | "expanded";
export type CodexPhase = "idle" | "running" | "completed" | "failed" | "stale";

export type CodexSession = {
  sessionId: string;
  turnId: string;
  phase: CodexPhase;
  prompt: string;
  cwd: string;
  model: string;
  lastAssistantMessage: string;
  startedAt: number;
  updatedAt: number;
  activity: string;
  attention: "" | "approval" | "input";
  planCompleted: number;
  planTotal: number;
  currentStep: string;
};

export type CodexStatusSnapshot = {
  sessions: CodexSession[];
  updatedAt: number;
  statusPath: string;
};

export type CodexIntegrationStatus = {
  configured: boolean;
  verified: boolean;
  hooksPath: string;
  scriptPath: string;
  verificationPath: string;
};

export type CodexHooksInstallResult = CodexIntegrationStatus & {
  statusPath: string;
  backupPath?: string;
  installedAt: number;
};

export const EMPTY_STATUS: CodexStatusSnapshot = {
  sessions: [],
  updatedAt: 0,
  statusPath: "",
};

export const EMPTY_INTEGRATION: CodexIntegrationStatus = {
  configured: false,
  verified: false,
  hooksPath: "",
  scriptPath: "",
  verificationPath: "",
};
