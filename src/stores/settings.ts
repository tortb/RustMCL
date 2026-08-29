import { create } from "zustand";
import { detectJava, getAppConfig, updateAppConfig } from "../lib/api";
import type { AppConfig } from "../lib/types";

interface SettingsStore {
  config: AppConfig | null;
  loaded: boolean;
  saving: boolean;
  javaVersion: string | null;
  detectingJava: boolean;
  error: string;

  load: () => Promise<void>;
  save: (config: AppConfig) => Promise<void>;
  detect: () => Promise<void>;
  clearError: () => void;
}

export const useSettingsStore = create<SettingsStore>((set) => ({
  config: null,
  loaded: false,
  saving: false,
  javaVersion: null,
  detectingJava: false,
  error: "",

  load: async () => {
    try {
      const config = await getAppConfig();
      set({ config, loaded: true });
    } catch (e) {
      set({ error: String(e), loaded: true });
    }
  },

  save: async (config) => {
    set({ saving: true, error: "" });
    try {
      const saved = await updateAppConfig(config);
      set({ config: saved, saving: false });
    } catch (e) {
      set({ error: String(e), saving: false });
    }
  },

  detect: async () => {
    set({ detectingJava: true });
    try {
      const v = await detectJava();
      set({ javaVersion: v, detectingJava: false });
    } catch {
      set({ javaVersion: null, detectingJava: false });
    }
  },

  clearError: () => set({ error: "" }),
}));
