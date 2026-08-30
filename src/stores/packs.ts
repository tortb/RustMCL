import { create } from "zustand";
import {
  checkShaderSupport,
  getInstance,
  getResourcePackVersions,
  installResourcePack,
  listInstances,
  removeResourcePack,
  scanResourcePacks,
  searchResourcePacks,
  setResourcePackEnabled,
} from "../lib/api";
import type {
  InstanceDetail,
  ModrinthHit,
  ModrinthVersion,
  ResourcePackEntry,
  ShaderSupportInfo,
} from "../lib/types";

export type PackType = "resourcepack" | "shaderpack";

interface PacksStore {
  instances: InstanceDetail[];
  selectedInstanceId: string;
  type: PackType;
  packs: ResourcePackEntry[];
  loading: boolean;

  query: string;
  results: ModrinthHit[];
  searching: boolean;
  shaderSupport: ShaderSupportInfo | null;

  // 版本选择弹窗(从搜索结果安装)
  versionModalProject: ModrinthHit | null;
  versions: ModrinthVersion[];
  loadingVersions: boolean;
  installing: boolean;
  installError: string;

  loadInstances: () => Promise<void>;
  setType: (t: PackType) => Promise<void>;
  setQuery: (q: string) => void;
  scan: () => Promise<void>;
  search: () => Promise<void>;
  openVersions: (hit: ModrinthHit) => Promise<void>;
  closeVersions: () => void;
  install: (version: ModrinthVersion) => Promise<void>;
  toggle: (pack: ResourcePackEntry) => Promise<void>;
  remove: (pack: ResourcePackEntry) => Promise<void>;
}

export const usePacksStore = create<PacksStore>((set, get) => ({
  instances: [],
  selectedInstanceId: "",
  type: "resourcepack",
  packs: [],
  loading: false,

  query: "",
  results: [],
  searching: false,
  shaderSupport: null,

  versionModalProject: null,
  versions: [],
  loadingVersions: false,
  installing: false,
  installError: "",

  loadInstances: async () => {
    try {
      const list = await listInstances();
      const details = await Promise.all(list.map((i) => getInstance(i.id)));
      const instances = details.filter((d): d is InstanceDetail => d !== null);
      set({ instances });
      if (!get().selectedInstanceId && instances.length > 0) {
        set({ selectedInstanceId: instances[0].id });
        await get().scan();
      }
    } catch {
      set({ instances: [] });
    }
  },

  setType: async (type) => {
    set({ type });
    await get().scan();
  },

  setQuery: (q) => set({ query: q }),

  scan: async () => {
    const id = get().selectedInstanceId;
    if (!id) {
      set({ packs: [], shaderSupport: null });
      return;
    }
    set({ loading: true });
    try {
      // 扫描全部类型,再按当前类型过滤(避免切类型时重复扫描)
      const all = await scanResourcePacks(id);
      let shaderSupport: ShaderSupportInfo | null = null;
      if (get().type === "shaderpack") {
        try {
          shaderSupport = await checkShaderSupport(id);
        } catch {
          shaderSupport = null;
        }
      }
      set({
        packs: all.filter((p) => p.type_kind === get().type),
        shaderSupport,
        loading: false,
      });
    } catch {
      set({ packs: [], shaderSupport: null, loading: false });
    }
  },

  search: async () => {
    const q = get().query.trim();
    if (!q) return;
    set({ searching: true });
    try {
      const results = await searchResourcePacks(q, get().type);
      set({ results, searching: false });
    } catch {
      set({ results: [], searching: false });
    }
  },

  openVersions: async (hit) => {
    const id = get().selectedInstanceId;
    if (!id) return;
    set({
      versionModalProject: hit,
      versions: [],
      loadingVersions: true,
      installing: false,
      installError: "",
    });
    try {
      const versions = await getResourcePackVersions(hit.project_id, id);
      set({ versions, loadingVersions: false });
    } catch {
      set({ versions: [], loadingVersions: false, installError: "获取版本失败" });
    }
  },

  closeVersions: () =>
    set({
      versionModalProject: null,
      versions: [],
      loadingVersions: false,
      installing: false,
      installError: "",
    }),

  install: async (version) => {
    const id = get().selectedInstanceId;
    const packType = get().type;
    if (!id) return;
    set({ installing: true, installError: "" });
    try {
      await installResourcePack(id, version.id, packType);
      await get().scan();
      set({ versionModalProject: null, versions: [], installing: false });
    } catch (e) {
      set({ installing: false, installError: String(e) });
    }
  },

  toggle: async (pack) => {
    await setResourcePackEnabled(pack.id, !pack.enabled);
    await get().scan();
  },

  remove: async (pack) => {
    await removeResourcePack(pack.id);
    await get().scan();
  },
}));

