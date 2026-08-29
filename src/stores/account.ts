import { create } from "zustand";
import {
  getActiveAccount,
  listAccounts,
  logoutAccount,
  startMicrosoftLogin,
  cancelMicrosoftLogin,
  createOfflineAccount,
} from "../lib/api";
import type { Account, MsDeviceInfo, MsLoginFinished, MsLoginStatus } from "../lib/types";

export type LoginStage =
  | "idle"
  | "device"
  | "waiting"
  | "exchanging"
  | "saving"
  | "error";

interface AccountStore {
  accounts: Account[];
  active: Account | null;
  loaded: boolean;

  // 登录弹窗状态
  loginOpen: boolean;
  device: MsDeviceInfo | null;
  statusMsg: string;
  stage: LoginStage;
  loginError: string;

  loadAccounts: () => Promise<void>;
  openLogin: () => void;
  closeLogin: () => void;
  startLogin: () => Promise<void>;
  cancelLogin: () => Promise<void>;
  /** 创建离线账号:入参为已在前端校验过的合法用户名 */
  createOffline: (username: string) => Promise<void>;
  logout: (id: string) => Promise<void>;

  // ms-login-* 事件回调(由 LoginModal 注册监听)
  onDevice: (d: MsDeviceInfo) => void;
  onStatus: (s: MsLoginStatus) => void;
  onFinished: (f: MsLoginFinished) => void;
}

export const useAccountStore = create<AccountStore>((set, get) => ({
  accounts: [],
  active: null,
  loaded: false,

  loginOpen: false,
  device: null,
  statusMsg: "",
  stage: "idle",
  loginError: "",

  loadAccounts: async () => {
    try {
      const [accounts, active] = await Promise.all([listAccounts(), getActiveAccount()]);
      set({ accounts, active, loaded: true });
    } catch {
      set({ loaded: true });
    }
  },

  openLogin: () => set({ loginOpen: true, stage: "idle", device: null, loginError: "" }),
  closeLogin: () => set({ loginOpen: false, stage: "idle", device: null, loginError: "" }),

  startLogin: async () => {
    set({ stage: "device", loginError: "", device: null });
    try {
      await startMicrosoftLogin();
    } catch (e) {
      set({ stage: "error", loginError: String(e) });
    }
  },

  cancelLogin: async () => {
    try {
      await cancelMicrosoftLogin();
    } finally {
      set({ loginOpen: false, stage: "idle", device: null });
    }
  },

  createOffline: async (username) => {
    try {
      await createOfflineAccount(username);
      set({ loginOpen: false, stage: "idle", device: null, loginError: "" });
      await get().loadAccounts();
    } catch (e) {
      set({ stage: "error", loginError: String(e) });
    }
  },

  logout: async (id) => {
    try {
      await logoutAccount(id);
    } finally {
      await get().loadAccounts();
    }
  },

  onDevice: (d) => set({ device: d, stage: "waiting", statusMsg: "" }),
  onStatus: (s) =>
    set({
      stage: s.stage === "exchange" ? "exchanging" : s.stage === "save" ? "saving" : "waiting",
      statusMsg: s.message,
    }),
  onFinished: (f) => {
    if (f.ok) {
      set({ loginOpen: false, stage: "idle", device: null, loginError: "" });
      get().loadAccounts();
    } else {
      set({ stage: "error", loginError: f.error });
    }
  },
}));
