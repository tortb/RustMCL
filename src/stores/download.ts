import { create } from "zustand";
import { downloadVersion, launchVersion, listVersions } from "../lib/api";
import type { VersionInfo } from "../lib/types";

export type DownloadState = "idle" | "downloading" | "done" | "error";
export type RunState = "idle" | "running" | "exited";

export interface MergedProgress {
  phase: "core" | "assets";
  current: number;
  total: number;
  file: string;
}

interface DownloadsStore {
  // 版本列表
  versions: VersionInfo[];
  versionsLoading: boolean;
  selected: string;
  setSelected: (id: string) => void;
  loadVersions: () => Promise<void>;

  // 下载
  dlState: DownloadState;
  dlError: string;
  progress: MergedProgress | null;
  setDownloading: () => void;
  setProgress: (p: MergedProgress) => void;
  setDownloadFinished: (ok: boolean, error: string) => void;
  startDownload: () => Promise<void>;

  // 启动
  runState: RunState;
  exitCode: number | null;
  logs: string[];
  appendLog: (line: string) => void;
  setRunState: (s: RunState, code?: number) => void;
  startLaunch: () => Promise<void>;
}

export const useDownloadsStore = create<DownloadsStore>((set, get) => ({
  versions: [],
  versionsLoading: false,
  selected: "",
  setSelected: (id) => set({ selected: id }),
  loadVersions: async () => {
    if (get().versions.length > 0) return;
    set({ versionsLoading: true });
    try {
      const versions = await listVersions("release");
      set({ versions: versions.slice(0, 40), versionsLoading: false });
      if (!get().selected && versions.length > 0) {
        set({ selected: versions[0].id });
      }
    } catch {
      set({ versionsLoading: false });
    }
  },

  dlState: "idle",
  dlError: "",
  progress: null,
  setDownloading: () => set({ dlState: "downloading", dlError: "", progress: null }),
  setProgress: (p) => set({ progress: p }),
  setDownloadFinished: (ok, error) =>
    set(ok ? { dlState: "done", progress: null } : { dlState: "error", dlError: error, progress: null }),
  startDownload: async () => {
    const id = get().selected;
    if (!id || get().dlState === "downloading") return;
    get().setDownloading();
    try {
      await downloadVersion(id);
    } catch (e) {
      get().setDownloadFinished(false, String(e));
    }
  },

  runState: "idle",
  exitCode: null,
  logs: [],
  appendLog: (line) =>
    set((s) => ({ logs: [...s.logs.slice(-499), line] })),
  setRunState: (s, code) => set({ runState: s, exitCode: code ?? null }),
  startLaunch: async () => {
    const id = get().selected;
    if (!id || get().runState === "running") return;
    set({ logs: [], runState: "running", exitCode: null });
    get().appendLog(`[RustMCL] 正在启动 ${id} ...`);
    try {
      // 不传用户名:Rust 侧优先使用已登录的微软账号,否则离线 Steve
      await launchVersion(id);
    } catch (e) {
      get().appendLog(`[RustMCL] 启动失败: ${e}`);
      get().setRunState("exited", -1);
    }
  },
}));
