import { create } from "zustand";
import {
  addServer,
  joinServer,
  listServers,
  pingServer,
  removeServer,
  updateServer,
} from "../lib/api";
import type { ServerEntry, ServerStatus } from "../lib/types";

interface ServerStore {
  servers: ServerEntry[];
  loading: boolean;
  pingingId: string | null;
  error: string;

  load: () => Promise<void>;
  add: (name: string, address: string, port: number, favorite?: boolean) => Promise<void>;
  remove: (id: string) => Promise<void>;
  toggleFavorite: (id: string, favorite: boolean) => Promise<void>;
  reorder: (servers: ServerEntry[]) => Promise<void>;
  ping: (id: string) => Promise<ServerStatus | null>;
  pingAll: () => Promise<void>;
  join: (serverId: string, instanceId: string) => Promise<void>;
  clearError: () => void;
}

export const useServerStore = create<ServerStore>((set, get) => ({
  servers: [],
  loading: false,
  pingingId: null,
  error: "",

  load: async () => {
    set({ loading: true });
    try {
      const servers = await listServers();
      set({ servers, loading: false });
    } catch {
      set({ loading: false });
    }
  },

  add: async (name, address, port, favorite) => {
    try {
      await addServer(name, address, port, favorite ?? false);
      await get().load();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  remove: async (id) => {
    await removeServer(id);
    await get().load();
  },

  toggleFavorite: async (id, favorite) => {
    await updateServer(id, undefined, favorite);
    await get().load();
  },

  reorder: async (servers) => {
    // 以新顺序重建列表(保留原对象引用,避免拖拽后字段丢失)
    const current = get().servers;
    const byId = new Map(current.map((s) => [s.id, s]));
    const next = servers
      .map((s) => byId.get(s.id))
      .filter((s): s is ServerEntry => s !== undefined);
    set({ servers: next });
    // 持久化 sort_order
    try {
      await Promise.all(next.map((s, idx) => updateServer(s.id, undefined, undefined, idx)));
    } catch (e) {
      set({ error: String(e) });
    }
  },

  ping: async (id) => {
    set({ pingingId: id });
    try {
      const st = await pingServer(id);
      // 把延迟写回本地卡片
      set((s) => ({
        servers: s.servers.map((sv) =>
          sv.id === id ? { ...sv, last_ping_ms: st.latency_ms } : sv,
        ),
      }));
      return st;
    } catch (e) {
      set((s) => ({
        servers: s.servers.map((sv) =>
          sv.id === id ? { ...sv, last_ping_ms: null } : sv,
        ),
      }));
      return null;
    } finally {
      set({ pingingId: null });
    }
  },

  pingAll: async () => {
    const ids = get().servers.map((s) => s.id);
    for (const id of ids) {
      await get().ping(id);
    }
  },

  join: async (serverId, instanceId) => {
    try {
      await joinServer(serverId, instanceId);
    } catch (e) {
      set({ error: String(e) });
    }
  },

  clearError: () => set({ error: "" }),
}));
