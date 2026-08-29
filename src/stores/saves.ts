import { create } from "zustand";
import {
  backupSave,
  deleteSave,
  deleteScreenshot,
  listBackups,
  listSaves,
  listScreenshots,
  restoreBackup,
} from "../lib/api";
import type { BackupInfo, SaveInfo, ScreenshotInfo } from "../lib/types";

interface SavesStore {
  instanceId: string | null;
  saves: SaveInfo[];
  backups: BackupInfo[];
  screenshots: ScreenshotInfo[];
  loading: boolean;
  message: string;
  error: string;

  load: (instanceId: string) => Promise<void>;
  backup: (saveName: string) => Promise<void>;
  restore: (backupName: string, targetName: string) => Promise<void>;
  removeSave: (saveName: string) => Promise<void>;
  removeScreenshot: (name: string) => Promise<void>;
  clearMessage: () => void;
}

export const useSavesStore = create<SavesStore>((set, get) => ({
  instanceId: null,
  saves: [],
  backups: [],
  screenshots: [],
  loading: false,
  message: "",
  error: "",

  load: async (instanceId) => {
    set({ instanceId, loading: true, error: "" });
    try {
      const [saves, backups, screenshots] = await Promise.all([
        listSaves(instanceId),
        listBackups(instanceId),
        listScreenshots(instanceId),
      ]);
      set({ saves, backups, screenshots, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  backup: async (saveName) => {
    try {
      await backupSave(get().instanceId!, saveName);
      set({ message: `已备份 ${saveName}` });
      await get().load(get().instanceId!);
    } catch (e) {
      set({ error: String(e) });
    }
  },

  restore: async (backupName, targetName) => {
    try {
      await restoreBackup(get().instanceId!, backupName, targetName);
      set({ message: `已恢复为「${targetName}」` });
      await get().load(get().instanceId!);
    } catch (e) {
      set({ error: String(e) });
    }
  },

  removeSave: async (saveName) => {
    await deleteSave(get().instanceId!, saveName);
    await get().load(get().instanceId!);
  },

  removeScreenshot: async (name) => {
    await deleteScreenshot(get().instanceId!, name);
    await get().load(get().instanceId!);
  },

  clearMessage: () => set({ message: "", error: "" }),
}));
