import { create } from "zustand";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  cancelInstanceDownload,
  createInstance,
  deleteInstance,
  exportModpack,
  getInstance,
  getLatestLoaderVersion,
  importModpack,
  installForge,
  installLoader,
  launchInstance,
  listForgeVersions,
  listInstances,
  listVersions,
  prepareInstance,
  updateInstance,
} from "../lib/api";
import type {
  DownloadFinished,
  DownloadProgress,
  ForgeVersionInfo,
  InstanceDetail,
  InstanceInput,
  ModpackFinished,
  ModpackProgress,
  VersionInfo,
} from "../lib/types";
import { useAccountStore } from "./account";

interface InstanceStore {
  instances: InstanceDetail[];
  loading: boolean;
  /**
   * 实例加载失败时的错误信息(用于展示错误态 + 重试);
   * 为空表示无错误。
   */
  error: string;
  versions: VersionInfo[];
  versionsLoading: boolean;

  // Forge 版本(创建弹窗用)
  forgeVersions: ForgeVersionInfo[];
  forgeVersionsLoading: boolean;

  // 创建/编辑弹窗
  modalOpen: boolean;
  editing: InstanceDetail | null;

  // 启动状态
  runningId: string | null;
  /** 启动时自动下载资源的阶段进度(用于实例卡片进度条) */
  launchProgress: DownloadProgress | null;
  logs: string[];

  // 加载器安装状态
  installingId: string | null;
  installProgress: DownloadProgress | null;
  installError: string;

  // 创建实例时的资源预下载(创建弹窗内进度)
  creatingId: string | null;
  createProgress: DownloadProgress | null;
  createError: string;

  // 整合包导入/导出
  modpackImportingId: string | null;
  modpackProgress: ModpackProgress | null;
  modpackResult: ModpackFinished | null;

  loadInstances: () => Promise<void>;
  loadVersions: () => Promise<void>;
  loadForgeVersions: (mcVersion: string) => Promise<void>;
  openCreate: () => void;
  openEdit: (inst: InstanceDetail) => void;
  closeModal: () => void;
  save: (input: InstanceInput) => Promise<void>;
  createAndPrepare: (input: InstanceInput) => Promise<void>;
  cancelCreate: (id: string) => Promise<void>;
  handleCreateFinished: (e: DownloadFinished) => Promise<void>;
  remove: (id: string) => Promise<void>;
  launch: (id: string) => Promise<void>;
  appendLog: (line: string) => void;
  setRunning: (id: string | null) => void;
  setLaunchProgress: (p: DownloadProgress | null) => void;
  setCreateProgress: (p: DownloadProgress | null) => void;
  setInstalling: (id: string | null) => void;
  setInstallProgress: (p: DownloadProgress | null) => void;
  setInstallError: (e: string) => void;
  importPack: (id: string) => Promise<void>;
  exportPack: (id: string) => Promise<void>;
  setModpackProgress: (p: ModpackProgress | null) => void;
  setModpackResult: (r: ModpackFinished | null) => void;
}

export const useInstanceStore = create<InstanceStore>((set, get) => ({
  instances: [],
  loading: false,
  error: "",
  versions: [],
  versionsLoading: false,

  forgeVersions: [],
  forgeVersionsLoading: false,

  modalOpen: false,
  editing: null,

  runningId: null,
  launchProgress: null,
  logs: [],

  installingId: null,
  installProgress: null,
  installError: "",

  creatingId: null,
  createProgress: null,
  createError: "",

  modpackImportingId: null,
  modpackProgress: null,
  modpackResult: null,

  loadInstances: async () => {
    set({ loading: true, error: "" });
    try {
      const list = await listInstances();
      const details = await Promise.all(list.map((i) => getInstance(i.id)));
      set({ instances: details.filter((d): d is InstanceDetail => d !== null), loading: false });
    } catch (e) {
      set({ loading: false, error: String(e) });
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

  loadForgeVersions: async (mcVersion) => {
    if (!mcVersion) {
      set({ forgeVersions: [], forgeVersionsLoading: false });
      return;
    }
    set({ forgeVersionsLoading: true });
    try {
      const list = await listForgeVersions(mcVersion);
      set({ forgeVersions: list, forgeVersionsLoading: false });
    } catch {
      set({ forgeVersions: [], forgeVersionsLoading: false });
    }
  },

  openCreate: () => set({ modalOpen: true, editing: null }),
  openEdit: (inst) => set({ modalOpen: true, editing: inst }),
  closeModal: () => set({ modalOpen: false, editing: null }),

  save: async (input) => {
    const editing = get().editing;
    if (editing) {
      await updateInstance(editing.id, input);
      await get().loadInstances();
      set({ modalOpen: false, editing: null });
      return;
    }
    // 创建:走"创建 + 资源预下载"流程,弹窗经 progress 事件驱动,由 download-finished 收尾
    await get().createAndPrepare(input);
  },

  createAndPrepare: async (input) => {
    // loader 非 vanilla 且未指定版本时自动解析最新加载器版本
    const isModded = input.loader !== undefined && input.loader !== "vanilla";
    const loaderVersion =
      isModded && !input.loader_version && input.mc_version
        ? await getLatestLoaderVersion(input.mc_version, input.loader!)
        : (input.loader_version ?? "");
    const detail = await createInstance({
      ...input,
      loader_version: loaderVersion || undefined,
    });
    await get().loadInstances();
    set({ creatingId: detail.id, createProgress: null, createError: "" });

    // 非原版实例:后台安装加载器并展示进度(卡片 installingId)
    const meta = detail.config.meta;
    if (meta.loader === "fabric" || meta.loader === "quilt") {
      set({ installingId: detail.id, installProgress: null, installError: "" });
      installLoader(meta.mc_version, meta.loader, meta.loader_version || "").catch((e) => {
        set({ installError: String(e) });
        set({ installingId: null, installProgress: null });
      });
    } else if (meta.loader === "forge") {
      set({ installingId: detail.id, installProgress: null, installError: "" });
      installForge(meta.mc_version, meta.loader_version || "").catch((e) => {
        set({ installError: String(e) });
        set({ installingId: null, installProgress: null });
      });
    }

    // 触发资源预下载(命令立即返回,后台执行,进度走 download-progress,结束走 download-finished)
    try {
      await prepareInstance(detail.id);
    } catch (e) {
      get().handleCreateFinished({ ok: false, error: String(e), cancelled: false });
    }
  },

  cancelCreate: async (id) => {
    try {
      await cancelInstanceDownload(id);
    } catch {
      // ignore
    }
    try {
      await deleteInstance(id);
    } catch {
      // ignore
    }
    await get().loadInstances();
    set({ creatingId: null, createProgress: null, createError: "", modalOpen: false, editing: null });
  },

  handleCreateFinished: async (e) => {
    const id = get().creatingId;
    if (!id) return;
    if (e.ok) {
      await get().loadInstances();
      set({ creatingId: null, createProgress: null, createError: "", modalOpen: false, editing: null });
    } else if (e.cancelled) {
      // 实例已在 cancelCreate 中删除,仅清状态
      set({ creatingId: null, createProgress: null, createError: "", modalOpen: false, editing: null });
    } else {
      // 失败:保留弹窗展示错误,用户可重试或关闭(关闭走 cancelCreate 清理)
      set({ createError: e.error });
    }
  },

  remove: async (id) => {
    await deleteInstance(id);
    await get().loadInstances();
  },

  launch: async (id) => {
    if (get().runningId) return;
    // 登录门禁:未登录时不发起实际启动,改为弹出登录引导(选择微软或离线账号)
    if (!useAccountStore.getState().active) {
      useAccountStore.getState().openLogin();
      return;
    }
    set({ runningId: id, logs: [], launchProgress: null });
    get().appendLog(`[RustMCL] 正在启动实例 ${id.slice(0, 8)} ...`);
    try {
      await launchInstance(id);
    } catch (e) {
      get().appendLog(`[RustMCL] 启动失败: ${e}`);
      get().setRunning(null);
    }
  },

  appendLog: (line) => set((s) => ({ logs: [...s.logs.slice(-499), line] })),
  setRunning: (id) => set({ runningId: id }),
  setLaunchProgress: (p) => set({ launchProgress: p }),
  setCreateProgress: (p) => set({ createProgress: p }),
  setInstalling: (id) => set({ installingId: id }),
  setInstallProgress: (p) => set({ installProgress: p }),
  setInstallError: (e) => set({ installError: e }),

  importPack: async (id) => {
    const selected = await open({
      multiple: false,
      filters: [{ name: "整合包", extensions: ["mrpack", "zip", "jar"] }],
    });
    if (!selected || Array.isArray(selected)) return;
    set({ modpackImportingId: id, modpackProgress: null, modpackResult: null });
    try {
      await importModpack(selected, id);
    } catch (e) {
      set({
        modpackResult: { ok: false, error: String(e), installed: [], failures: [], name: "" },
        modpackImportingId: null,
      });
    }
  },

  exportPack: async (id) => {
    const selected = await save({ filters: [{ name: "整合包", extensions: ["mrpack"] }] });
    if (!selected) return;
    try {
      await exportModpack(id, selected);
    } catch {
      // 导出失败时静默,用户可重试
    }
  },

  setModpackProgress: (p) => set({ modpackProgress: p }),
  setModpackResult: (r) => set({ modpackResult: r, modpackImportingId: null }),
}));
