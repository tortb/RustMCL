import { create } from "zustand";
import {
  createInstance,
  deleteInstance,
  getInstance,
  getLatestLoaderVersion,
  installLoader,
  launchInstance,
  listInstances,
  listVersions,
  updateInstance,
} from "../lib/api";
import type {
  DownloadProgress,
  InstanceDetail,
  InstanceInput,
  VersionInfo,
} from "../lib/types";

interface InstanceStore {
  instances: InstanceDetail[];
  loading: boolean;
  versions: VersionInfo[];
  versionsLoading: boolean;

  // 创建/编辑弹窗
  modalOpen: boolean;
  editing: InstanceDetail | null;

  // 启动状态
  runningId: string | null;
  logs: string[];

  // 加载器安装状态
  installingId: string | null;
  installProgress: DownloadProgress | null;
  installError: string;

  loadInstances: () => Promise<void>;
  loadVersions: () => Promise<void>;
  openCreate: () => void;
  openEdit: (inst: InstanceDetail) => void;
  closeModal: () => void;
  save: (input: InstanceInput) => Promise<void>;
  remove: (id: string) => Promise<void>;
  launch: (id: string) => Promise<void>;
  appendLog: (line: string) => void;
  setRunning: (id: string | null) => void;
  setInstalling: (id: string | null) => void;
  setInstallProgress: (p: DownloadProgress | null) => void;
  setInstallError: (e: string) => void;
}

export const useInstanceStore = create<InstanceStore>((set, get) => ({
  instances: [],
  loading: false,
  versions: [],
  versionsLoading: false,

  modalOpen: false,
  editing: null,

  runningId: null,
  logs: [],

  installingId: null,
  installProgress: null,
  installError: "",

  loadInstances: async () => {
    set({ loading: true });
    try {
      const list = await listInstances();
      const details = await Promise.all(list.map((i) => getInstance(i.id)));
      set({ instances: details.filter((d): d is InstanceDetail => d !== null), loading: false });
    } catch {
      set({ loading: false });
    }
  },

  loadVersions: async () => {
    if (get().versions.length > 0) return;
    set({ versionsLoading: true });
    try {
      const versions = await listVersions("release");
      set({ versions: versions.slice(0, 40), versionsLoading: false });
    } catch {
      set({ versionsLoading: false });
    }
  },

  openCreate: () => set({ modalOpen: true, editing: null }),
  openEdit: (inst) => set({ modalOpen: true, editing: inst }),
  closeModal: () => set({ modalOpen: false, editing: null }),

  save: async (input) => {
    const editing = get().editing;
    let detail: InstanceDetail;
    if (editing) {
      detail = await updateInstance(editing.id, input);
    } else {
      // 创建:loader 非 vanilla 且未指定版本时自动解析最新加载器版本
      const isModded = input.loader !== undefined && input.loader !== "vanilla";
      const loaderVersion =
        isModded && !input.loader_version && input.mc_version
          ? await getLatestLoaderVersion(input.mc_version, input.loader!)
          : (input.loader_version ?? "");
      detail = await createInstance({
        ...input,
        loader_version: loaderVersion || undefined,
      });
    }
    await get().loadInstances();
    set({ modalOpen: false, editing: null });

    // 非原版实例:后台安装加载器并展示进度
    const meta = detail.config.meta;
    if (meta.loader !== "vanilla") {
      set({ installingId: detail.id, installProgress: null, installError: "" });
      try {
        await installLoader(meta.mc_version, meta.loader, meta.loader_version || "");
      } catch (e) {
        set({ installError: String(e) });
        set({ installingId: null, installProgress: null });
      }
    }
  },

  remove: async (id) => {
    await deleteInstance(id);
    await get().loadInstances();
  },

  launch: async (id) => {
    if (get().runningId) return;
    set({ runningId: id, logs: [] });
    get().appendLog(`[Runa] 正在启动实例 ${id.slice(0, 8)} ...`);
    try {
      await launchInstance(id);
    } catch (e) {
      get().appendLog(`[Runa] 启动失败: ${e}`);
      get().setRunning(null);
    }
  },

  appendLog: (line) => set((s) => ({ logs: [...s.logs.slice(-499), line] })),
  setRunning: (id) => set({ runningId: id }),
  setInstalling: (id) => set({ installingId: id }),
  setInstallProgress: (p) => set({ installProgress: p }),
  setInstallError: (e) => set({ installError: e }),
}));
