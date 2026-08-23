import { create } from "zustand";
import {
  createInstance,
  deleteInstance,
  getInstance,
  launchInstance,
  listInstances,
  listVersions,
  updateInstance,
} from "../lib/api";
import type { InstanceDetail, InstanceInput, VersionInfo } from "../lib/types";

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
    if (editing) {
      await updateInstance(editing.id, input);
    } else {
      await createInstance(input);
    }
    await get().loadInstances();
    set({ modalOpen: false, editing: null });
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
}));
