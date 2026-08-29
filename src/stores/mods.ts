import { create } from "zustand";
import {
  deleteMod,
  getInstance,
  getModVersions,
  installMod,
  listInstanceMods,
  listInstances,
  searchMods,
  setModEnabled,
} from "../lib/api";
import type {
  InstanceDetail,
  ModEntry,
  ModrinthHit,
  ModrinthVersion,
} from "../lib/types";

interface ModsStore {
  instances: InstanceDetail[];
  selectedInstanceId: string;

  installed: ModEntry[];
  loadingInstalled: boolean;

  query: string;
  results: ModrinthHit[];
  searching: boolean;
  searched: boolean;

  // 版本选择弹窗
  versionModalProject: ModrinthHit | null;
  versions: ModrinthVersion[];
  loadingVersions: boolean;
  installing: boolean;

  loadInstances: () => Promise<void>;
  selectInstance: (id: string) => Promise<void>;
  loadInstalled: (id: string) => Promise<void>;

  setQuery: (q: string) => void;
  search: () => Promise<void>;

  openVersions: (hit: ModrinthHit) => Promise<void>;
  closeVersions: () => void;
  install: (version: ModrinthVersion) => Promise<void>;
  toggle: (mod: ModEntry, enabled: boolean) => Promise<void>;
  remove: (mod: ModEntry) => Promise<void>;
}

export const useModsStore = create<ModsStore>((set, get) => ({
  instances: [],
  selectedInstanceId: "",

  installed: [],
  loadingInstalled: false,

  query: "",
  results: [],
  searching: false,
  searched: false,

  versionModalProject: null,
  versions: [],
  loadingVersions: false,
  installing: false,

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

  setQuery: (q) => set({ query: q }),

  search: async () => {
    const q = get().query.trim();
    if (!q) return;
    set({ searching: true });
    try {
      const results = await searchMods(q);
      set({ results, searching: false, searched: true });
    } catch {
      set({ results: [], searching: false, searched: true });
    }
  },

  openVersions: async (hit) => {
    const id = get().selectedInstanceId;
    if (!id) return;
    set({ versionModalProject: hit, versions: [], loadingVersions: true });
    try {
      const versions = await getModVersions(hit.project_id, id);
      set({ versions, loadingVersions: false });
    } catch {
      set({ versions: [], loadingVersions: false });
    }
  },

  closeVersions: () =>
    set({ versionModalProject: null, versions: [], loadingVersions: false }),

  install: async (version) => {
    const id = get().selectedInstanceId;
    if (!id) return;
    set({ installing: true });
    try {
      await installMod(id, version.id);
      await get().loadInstalled(id);
      set({ versionModalProject: null, versions: [], installing: false });
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
