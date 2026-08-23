import { create } from "zustand";
import { listVersions } from "../lib/api";
import type { VersionFilter, VersionInfo } from "../lib/types";

interface VersionsStore {
  versions: VersionInfo[];
  loading: boolean;
  error: string | null;
  filter: VersionFilter;
  query: string;
  setFilter: (f: VersionFilter) => void;
  setQuery: (q: string) => void;
  load: (force?: boolean) => Promise<void>;
}

export const useVersionsStore = create<VersionsStore>((set, get) => ({
  versions: [],
  loading: false,
  error: null,
  filter: "release",
  query: "",
  setFilter: (f) => {
    set({ filter: f });
    get().load();
  },
  setQuery: (q) => set({ query: q }),
  load: async (force = false) => {
    set({ loading: true, error: null });
    try {
      const versions = await listVersions(get().filter, force);
      set({ versions, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },
}));
