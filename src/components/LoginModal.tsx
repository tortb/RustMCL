import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { AnimatePresence, motion } from "framer-motion";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { Copy, Check, Loader2, X, Wifi, User } from "lucide-react";
import { useAccountStore } from "../stores/account";
import type { MsDeviceInfo, MsLoginFinished, MsLoginStatus } from "../lib/types";

const ease = [0.32, 0.72, 0, 1] as const;
// MC 用户名规则:3-16 位字母数字下划线
const USERNAME_RE = /^[A-Za-z0-9_]{3,16}$/;

type LoginMode = "microsoft" | "offline";

export default function LoginModal() {
  const { t } = useTranslation();
  const s = useAccountStore();
  const [copied, setCopied] = useState(false);
  const [mode, setMode] = useState<LoginMode>("microsoft");

  // 弹窗打开时默认切到微软登录
  useEffect(() => {
    if (s.loginOpen) setMode("microsoft");
  }, [s.loginOpen]);

  // 常驻组件:始终注册事件监听,避免弹窗关闭期间丢失事件
  useEffect(() => {
    let unlisteners: UnlistenFn[] = [];
    let mounted = true;
    Promise.all([
      listen<MsDeviceInfo>("ms-login-device", (e) => s.onDevice(e.payload)),
      listen<MsLoginStatus>("ms-login-status", (e) => s.onStatus(e.payload)),
      listen<MsLoginFinished>("ms-login-finished", (e) => s.onFinished(e.payload)),
    ]).then((un) => {
      unlisteners = un;
      if (!mounted) un.forEach((u) => u());
    });
    return () => {
      mounted = false;
      unlisteners.forEach((u) => u());
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const copyCode = async () => {
    if (!s.device) return;
    try {
      await navigator.clipboard.writeText(s.device.user_code);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // 剪贴板不可用时忽略
    }
  };

  return (
    <AnimatePresence>
      {s.loginOpen && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.2 }}
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 backdrop-blur-sm"
        >
          <motion.div
            initial={{ opacity: 0, scale: 0.95, y: 8 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.95, y: 8 }}
            transition={{ duration: 0.28, ease }}
            className="w-[400px] rounded-[20px] bg-card p-7 shadow-[0_24px_64px_rgba(0,0,0,0.16)]"
          >
            <div className="flex items-center justify-between">
              <h2 className="text-[18px] font-bold tracking-tight text-ink">{t("login.title")}</h2>
              {s.stage === "waiting" && mode === "microsoft" && (
                <button
                  onClick={s.cancelLogin}
                  className="rounded-full p-1.5 text-ink-3 transition-colors hover:bg-hover"
                  aria-label="关闭"
                >
                  <X size={16} />
                </button>
              )}
            </div>

            {/* 登录方式切换 */}
            <div className="mt-4 flex rounded-[10px] bg-sidebar p-1 text-[13px] font-medium">
              {(
                [
                  { key: "microsoft", label: t("account.microsoft"), icon: Wifi },
                  { key: "offline", label: t("account.offline"), icon: User },
                ] as { key: LoginMode; label: string; icon: React.ElementType }[]
              ).map((tab) => {
                const Icon = tab.icon;
                const active = mode === tab.key;
                return (
                  <button
                    key={tab.key}
                    onClick={() => {
                      setMode(tab.key);
                      // 重置到初始状态,清除上次失败的错误
                      s.openLogin();
                    }}
                    className={`flex flex-1 items-center justify-center gap-1.5 rounded-[8px] py-2 transition-colors duration-150 ${
                      active ? "bg-card text-ink shadow-sm" : "text-ink-3 hover:text-ink"
                    }`}
                  >
                    <Icon size={14} strokeWidth={1.8} />
                    {tab.label}
                  </button>
                );
              })}
            </div>

            {/* 微软面板 */}
            {mode === "microsoft" && (
              <>
                {/* 初始状态:开始登录 */}
                {s.stage === "idle" && (
                  <>
                    <p className="mt-2 text-[13px] leading-relaxed text-ink-3">
                      {t("login.msHint")}
                    </p>
                    <motion.button
                      whileTap={{ scale: 0.97 }}
                      onClick={s.startLogin}
                      className="mt-6 w-full rounded-[12px] bg-accent py-3 text-[14px] font-semibold text-on-accent transition-colors hover:bg-accent-hover"
                    >
                      {t("login.start")}
                    </motion.button>
                  </>
                )}

                {/* 获取设备码中 */}
                {s.stage === "device" && (
                  <div className="mt-6 flex flex-col items-center gap-3 py-4">
                    <Loader2 size={22} className="animate-spin text-accent" />
                    <p className="text-[13px] text-ink-3">{t("login.fetching")}</p>
                  </div>
                )}

                {/* 等待授权:展示设备码 */}
                {s.stage === "waiting" && s.device && (
                  <>
                    <p className="mt-3 text-[13px] leading-relaxed text-ink-3">
                      {t("login.openBrowser")}
                    </p>
                    <div className="mt-4 flex items-center justify-center gap-3 rounded-[14px] bg-sidebar py-4">
                      <span className="font-mono text-[26px] font-bold tracking-[0.2em] text-ink">
                        {s.device.user_code}
                      </span>
                      <button
                        onClick={copyCode}
                        className="rounded-full p-2 text-ink-3 transition-colors hover:bg-hover"
                        aria-label="复制设备码"
                      >
                        {copied ? <Check size={15} className="text-accent" /> : <Copy size={15} />}
                      </button>
                    </div>
                    <p className="mt-2 text-center font-mono text-[12px] text-ink-3">
                      {s.device.verification_uri}
                    </p>
                    {s.statusMsg && (
                      <p className="mt-3 text-center text-[12.5px] text-ink-3">{s.statusMsg}</p>
                    )}
                    <div className="mt-4 flex items-center justify-center gap-2">
                      <span className="relative flex h-2 w-2">
                        <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-accent opacity-60" />
                        <span className="relative inline-flex h-2 w-2 rounded-full bg-accent" />
                      </span>
                      <span className="text-[12px] text-ink-2">{t("login.waiting")}</span>
                    </div>
                    <button
                      onClick={s.cancelLogin}
                      className="mt-5 w-full rounded-[12px] border border-divider py-2.5 text-[13.5px] font-medium text-ink-2 transition-colors hover:bg-hover"
                    >
                      {t("common.cancel")}
                    </button>
                  </>
                )}

                {/* 兑换令牌 / 保存中 */}
                {(s.stage === "exchanging" || s.stage === "saving") && (
                  <div className="mt-6 flex flex-col items-center gap-3 py-4">
                    <Loader2 size={22} className="animate-spin text-accent" />
                    <p className="text-[13px] text-ink-3">{s.statusMsg || t("login.completing")}</p>
                  </div>
                )}

                {/* 错误 */}
                {s.stage === "error" && (
                  <ErrorPanel error={s.loginError} onRetry={s.startLogin} onClose={s.closeLogin} />
                )}
              </>
            )}

            {/* 离线面板 */}
            {mode === "offline" && (
              <OfflinePanel error={s.stage === "error" ? s.loginError : ""} />
            )}
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}

function ErrorPanel({
  error,
  onRetry,
  onClose,
}: {
  error: string;
  onRetry: () => void;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  return (
    <>
      <p className="mt-3 rounded-[10px] bg-danger-50 px-3.5 py-2.5 text-[13px] leading-relaxed text-danger-600">
        {error}
      </p>
      <div className="mt-4 flex gap-3">
        <button
          onClick={onRetry}
          className="flex-1 rounded-[12px] bg-accent py-2.5 text-[13.5px] font-semibold text-on-accent transition-colors hover:bg-accent-hover"
        >
          {t("common.retry")}
        </button>
        <button
          onClick={onClose}
          className="flex-1 rounded-[12px] border border-divider py-2.5 text-[13.5px] font-medium text-ink-2 transition-colors hover:bg-hover"
        >
          {t("common.close")}
        </button>
      </div>
    </>
  );
}

function OfflinePanel({ error }: { error: string }) {
  const { t } = useTranslation();
  const s = useAccountStore();
  const [name, setName] = useState("");
  const [err, setErr] = useState("");
  const [busy, setBusy] = useState(false);

  const submit = async () => {
    if (!USERNAME_RE.test(name)) {
      setErr(t("login.usernameRule"));
      return;
    }
    setErr("");
    setBusy(true);
    await s.createOffline(name);
    setBusy(false);
  };

  return (
    <div className="mt-4">
      <p className="text-[13px] leading-relaxed text-ink-3">
        {t("login.offlineHint")}
      </p>
      <input
        value={name}
        onChange={(e) => {
          setName(e.target.value);
          if (err) setErr("");
        }}
        onKeyDown={(e) => {
          if (e.key === "Enter") submit();
        }}
        placeholder={t("login.usernamePlaceholder")}
        maxLength={16}
        className="mt-4 w-full rounded-[12px] bg-sidebar px-4 py-3 text-[14px] text-ink outline-none placeholder:text-ink-3 focus:ring-2 focus:ring-accent/40"
      />
      {err && <p className="mt-2 text-[12.5px] text-danger-600">{err}</p>}
      {error && <p className="mt-2 rounded-[10px] bg-danger-50 px-3.5 py-2.5 text-[13px] leading-relaxed text-danger-600">{error}</p>}
      <motion.button
        whileTap={{ scale: 0.97 }}
        onClick={submit}
        disabled={busy}
        className="mt-4 flex w-full items-center justify-center gap-2 rounded-[12px] bg-accent py-3 text-[14px] font-semibold text-on-accent transition-colors hover:bg-accent-hover disabled:opacity-60"
      >
        {busy && <Loader2 size={16} className="animate-spin" />}
        {t("login.createOffline")}
      </motion.button>
    </div>
  );
}
