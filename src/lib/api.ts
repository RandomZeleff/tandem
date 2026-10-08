import { invoke } from "@tauri-apps/api/core";

export interface AppInfo {
  version: string;
  dataDir: string;
}

export type LogLevel = "TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR";

export interface LogEntry {
  seq: number;
  timestampMs: number;
  level: LogLevel;
  target: string;
  message: string;
}

export const LOG_EVENT = "log://entry";

export const api = {
  appInfo: () => invoke<AppInfo>("app_info"),
  getLogs: () => invoke<LogEntry[]>("get_logs"),
  getSetting: <T>(key: string) => invoke<T | null>("get_setting", { key }),
  setSetting: (key: string, value: unknown) => invoke<void>("set_setting", { key, value }),
};
