import { create } from "zustand";
import {
  checkModDependencies,
  deleteMod,
  getCurseforgeFileVersions,
  getInstance,
  getModVersions,
  installCurseforgeFile,
  installMod,
  listInstanceMods,
  listInstances,
  searchCurseforgeMods,
  searchMods,
  setModEnabled,
} from "../lib/api";
import type {
  CurseForgeFile,
  DepCheckResult,
  InstanceDetail,
  ModEntry,
  ModrinthVersion,
  ModSearchResult,
  ModSourceType,
} from "../lib/types";

interface ModsStore {
  instances: InstanceDetail[];
  selectedInstanceId: string;
  source: ModSourceType;

  installed: ModEntry[];
  loadingInstalled: boolean;

  query: string;
  results: ModSearchResult[];
  searching: boolean;
  searched: boolean;
  sourceError: string;

  // 版本选择弹窗
  versionModalProject: ModSearchResult | null;
  versions: ModrinthVersion[];
  cfFiles: CurseForgeFile[];
  loadingVersions: boolean;
  installing: boolean;
  depResult: DepCheckResult | null;
  depLoading: boolean;

  loadInstances: () => Promise<void>;
  selectInstance: (id: string) => Promise<void>;
  loadInstalled: (id: string) => Promise<void>;

  setSource: (s: ModSourceType) => void;
  setQuery: (q: string) => void;
  search: () => Promise<void>;

  openVersions: (hit: ModSearchResult) => Promise<void>;
  closeVersions: () => void;
  checkDeps: (versionId: string) => Promise<void>;
  install: (version: ModrinthVersion) => Promise<void>;
  installDep: (versionId: string) => Promise<void>;
  installCfFile: (file: CurseForgeFile) => Promise<void>;
  toggle: (mod: ModEntry, enabled: boolean) => Promise<void>;
  remove: (mod: ModEntry) => Promise<void>;
}

export const useModsStore = create<ModsStore>((set, get) => ({
  instances: [],
  selectedInstanceId: "",
  source: "modrinth",

  installed: [],
  loadingInstalled: false,

  query: "",
  results: [],
  searching: false,
  searched: false,
  sourceError: "",

  versionModalProject: null,
  versions: [],
  cfFiles: [],
  loadingVersions: false,
  installing: false,
  depResult: null,
  depLoading: false,

  loadInstances: async () => {
    try {
      const list = await listInstances();
      let selected = get().selectedInstanceId;
      const details = await Promise.all(list.map((i) => getInstance(i.id)));
      const instances = details.filter((d): d is InstanceDetail => d !== null);
      set({ instances });
      if (!selected && instances.length > 0) {
        selected = instances[0].id;
        set({ selectedInstanceId: selected });
        await get().loadInstalled(selected);
      }
    } catch {
      set({ instances: [] });
    }
  },

  selectInstance: async (id) => {
    set({ selectedInstanceId: id, results: [], searched: false });
    await get().loadInstalled(id);
  },

  loadInstalled: async (id) => {
    if (!id) {
      set({ installed: [] });
      return;
    }
    set({ loadingInstalled: true });
    try {
      const installed = await listInstanceMods(id);
      set({ installed, loadingInstalled: false });
    } catch {
      set({ installed: [], loadingInstalled: false });
    }
  },

  setSource: (source) => set({ source, results: [], searched: false, sourceError: "" }),

  setQuery: (q) => set({ query: q }),

  search: async () => {
    const q = get().query.trim();
    const source = get().source;
    if (!q) return;
    const inst = get().instances.find((i) => i.id === get().selectedInstanceId);
    const mc = inst?.config.meta.mc_version ?? "";
    const loader = inst?.config.meta.loader ?? "vanilla";
    set({ searching: true, sourceError: "" });
    try {
      if (source === "curseforge") {
        const hits = await searchCurseforgeMods(q, mc, loader);
        const results: ModSearchResult[] = hits.map((h) => ({ ...h, source: "curseforge" as const }));
        set({ results, searching: false, searched: true });
      } else {
        const hits = await searchMods(q);
        const results: ModSearchResult[] = hits.map((h) => ({ ...h, source: "modrinth" as const }));
        set({ results, searching: false, searched: true });
      }
    } catch (e) {
      set({ results: [], searching: false, searched: true, sourceError: String(e) });
    }
  },

  openVersions: async (hit) => {
    const id = get().selectedInstanceId;
    if (!id) return;
    set({
      versionModalProject: hit,
      versions: [],
      cfFiles: [],
      loadingVersions: true,
      depResult: null,
    });
    try {
      if (hit.source === "curseforge") {
        const cfFiles = await getCurseforgeFileVersions(hit.project_id, id);
        set({ cfFiles, loadingVersions: false });
      } else {
        const versions = await getModVersions(hit.project_id, id);
        set({ versions, loadingVersions: false });
      }
    } catch {
      set({ versions: [], cfFiles: [], loadingVersions: false });
    }
  },

  closeVersions: () =>
    set({
      versionModalProject: null,
      versions: [],
      cfFiles: [],
      loadingVersions: false,
      depResult: null,
    }),

  checkDeps: async (versionId) => {
    const id = get().selectedInstanceId;
    if (!id) return;
    set({ depLoading: true });
    try {
      const r = await checkModDependencies(id, versionId);
      set({ depResult: r, depLoading: false });
    } catch {
      set({ depResult: null, depLoading: false });
    }
  },

  install: async (version) => {
    const id = get().selectedInstanceId;
    if (!id) return;
    set({ installing: true });
    try {
      await installMod(id, version.id);
      await get().loadInstalled(id);
      set({ versionModalProject: null, versions: [], installing: false, depResult: null });
    } catch {
      set({ installing: false });
    }
  },

  installDep: async (versionId) => {
    const id = get().selectedInstanceId;
    if (!id) return;
    set({ installing: true });
    try {
      await installMod(id, versionId);
      await get().loadInstalled(id);
      set({ installing: false });
    } catch {
      set({ installing: false });
    }
  },

  installCfFile: async (file) => {
    const id = get().selectedInstanceId;
    const projectId = get().versionModalProject?.project_id;
    if (!id || !projectId) return;
    set({ installing: true });
    try {
      await installCurseforgeFile(id, projectId, file);
      await get().loadInstalled(id);
      set({ versionModalProject: null, cfFiles: [], installing: false });
    } catch {
      set({ installing: false });
    }
  },

  toggle: async (mod, enabled) => {
    await setModEnabled(mod.id, enabled);
    await get().loadInstalled(get().selectedInstanceId);
  },

  remove: async (mod) => {
    await deleteMod(mod.id);
    await get().loadInstalled(get().selectedInstanceId);
  },
}));
