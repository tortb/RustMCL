import { create } from "zustand";
import { dbHealth, getAppInfo } from "../lib/api";
import type { AppInfo } from "../lib/types";

interface AppStore {
  appInfo: AppInfo | null;
  dbTables: string[];
  loading: boolean;
  error: string | null;
  init: () => Promise<void>;
}

export const useAppStore = create<AppStore>((set) => ({
  appInfo: null,
  dbTables: [],
  loading: true,
  error: null,
  init: async () => {
    set({ loading: true, error: null });
    try {
      const [info, tables] = await Promise.all([getAppInfo(), dbHealth()]);
      set({ appInfo: info, dbTables: tables, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },
}));
