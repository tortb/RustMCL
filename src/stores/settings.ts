import { create } from "zustand";
import i18n from "../i18n";
import { detectJava, getAppConfig, updateAppConfig } from "../lib/api";
import type { AppConfig } from "../lib/types";

/// 把主题应用到根元素(无深色变量时移除 dark),供 load/save 与启动时调用
function applyTheme(theme: string) {
  document.documentElement.classList.toggle("dark", theme === "dark");
}

/// 应用界面语言(i18n 切换);供 load/save 与启动时调用
function applyLanguage(lang: string) {
  if (lang === "en-US") {
    void i18n.changeLanguage("en-US");
  } else {
    void i18n.changeLanguage("zh-CN");
  }
}

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
      applyTheme(config.general.theme);
      applyLanguage(config.general.language);
    } catch (e) {
      set({ error: String(e), loaded: true });
    }
  },

  save: async (config) => {
    set({ saving: true, error: "" });
    try {
      const saved = await updateAppConfig(config);
      set({ config: saved, saving: false });
      applyTheme(saved.general.theme);
      applyLanguage(saved.general.language);
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
