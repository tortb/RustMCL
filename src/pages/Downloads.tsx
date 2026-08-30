import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { Download, Play, Terminal, Loader2, CheckCircle2, XCircle } from "lucide-react";
import { useDownloadsStore } from "../stores/download";
import { AppSelect } from "../components/AppSelect";
import type { DownloadFinished, DownloadProgress, GameExit, GameLog } from "../lib/types";

const ease = [0.32, 0.72, 0, 1] as const;

export default function Downloads() {
  const { t } = useTranslation();
  const s = useDownloadsStore();
  const logRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    s.loadVersions();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    let unlisteners: UnlistenFn[] = [];
    let mounted = true;
    Promise.all([
      listen<DownloadProgress>("download-progress", (e) =>
        s.setProgress({
          phase: e.payload.phase,
          current: e.payload.current,
          total: e.payload.total,
          file: e.payload.file,
        }),
      ),
      // 仅当确实是下载页发起下载时才响应,避免实例启动路径的 download-finished 误改状态
      listen<DownloadFinished>("download-finished", (e) => {
        if (useDownloadsStore.getState().dlState === "downloading") {
          s.setDownloadFinished(e.payload.ok, e.payload.error);
        }
      }),
      listen<GameLog>("game-log", (e) => s.appendLog(e.payload.line)),
      listen<GameExit>("game-exit", (e) => {
        s.appendLog(`[RustMCL] ${t("downloads.gameExit", { code: e.payload.code })}`);
        s.setRunState("exited", e.payload.code);
      }),
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

  // 日志自动滚动
  useEffect(() => {
    const el = logRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [s.logs]);

  const downloading = s.dlState === "downloading";
  const pct = s.progress ? Math.round((s.progress.current / s.progress.total) * 100) : 0;

  return (
    <div className="flex-1 overflow-y-auto bg-bg px-6 py-8">
      <motion.div
        initial={{ opacity: 0, y: 10 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.35, ease }}
        className="mx-auto max-w-2xl"
      >
        <h1 className="text-[24px] font-bold tracking-tight text-ink">{t("downloads.title")}</h1>
        <p className="mt-1 text-[13px] text-ink-3">{t("downloads.subtitle")}</p>

        {/* 版本选择卡片 */}
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.06, duration: 0.35, ease }}
          className="mt-6 rounded-[16px] bg-card p-6 shadow-card"
        >
          <label className="text-[13px] font-medium text-ink-2">{t("downloads.versionLabel")}</label>
          <div className="mt-2 flex items-center gap-3">
            <AppSelect
              value={s.selected}
              onChange={(v) => s.setSelected(v)}
              disabled={downloading || s.runState === "running"}
              placeholder={s.versionsLoading ? t("common.loading") : undefined}
              className="flex-1"
              options={s.versions.map((v) => ({ value: v.id, label: v.id }))}
            />
            <motion.button
              whileTap={{ scale: 0.97 }}
              onClick={s.startDownload}
              disabled={downloading || s.runState === "running"}
              className="flex items-center gap-2 rounded-[12px] bg-accent px-5 py-2.5 text-[14px] font-semibold text-on-accent transition-colors hover:bg-accent-hover disabled:opacity-50"
            >
              {downloading ? <Loader2 size={16} className="animate-spin" /> : <Download size={16} strokeWidth={2.2} />}
              {downloading ? t("downloads.downloading") : t("downloads.downloadResources")}
            </motion.button>
          </div>

          {/* 下载状态 */}
          {downloading && s.progress && (
            <div className="mt-5">
              <div className="flex items-baseline justify-between text-[12.5px] text-ink-2">
                <span className="truncate">
                  {s.progress.phase === "core" ? t("downloads.coreFiles") : t("downloads.assetFiles")} · {s.progress.file}
                </span>
                <span className="ml-3 shrink-0 font-mono">
                  {s.progress.current}/{s.progress.total} · {pct}%
                </span>
              </div>
              <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-hover">
                <motion.div
                  className="h-full rounded-full bg-accent"
                  animate={{ width: `${pct}%` }}
                  transition={{ duration: 0.3, ease }}
                />
              </div>
            </div>
          )}

          {s.dlState === "done" && (
            <div className="mt-4 flex items-center gap-2 rounded-[10px] bg-badge-bg px-3.5 py-2.5 text-[13px] font-medium text-badge-text">
              <CheckCircle2 size={16} />
              {t("downloads.ready")}
            </div>
          )}
          {s.dlState === "error" && (
            <div className="mt-4 flex items-start gap-2 rounded-[10px] bg-danger-50 px-3.5 py-2.5 text-[13px] text-danger-600">
              <XCircle size={16} className="mt-0.5 shrink-0" />
              <span className="break-all">{s.dlError}</span>
            </div>
          )}
        </motion.div>

        {/* 启动卡片 */}
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.12, duration: 0.35, ease }}
          className="mt-4 rounded-[16px] bg-card p-6 shadow-card"
        >
          <div className="flex items-center justify-between">
            <div>
              <h2 className="text-[15px] font-semibold text-ink">{t("downloads.launchTitle")}</h2>
              <p className="mt-0.5 text-[12.5px] text-ink-3">
                {t("downloads.launchHint")}
              </p>
            </div>
            <motion.button
              whileTap={{ scale: 0.97 }}
              onClick={s.startLaunch}
              disabled={s.dlState !== "done" || s.runState === "running"}
              className="flex items-center gap-2 rounded-[12px] bg-accent px-5 py-2.5 text-[14px] font-semibold text-on-accent transition-colors hover:bg-accent-hover disabled:opacity-40"
            >
              {s.runState === "running" ? (
                <Loader2 size={16} className="animate-spin" />
              ) : (
                <Play size={15} fill="white" strokeWidth={0} />
              )}
              {s.runState === "running" ? t("downloads.running") : t("downloads.launchTitle")}
            </motion.button>
          </div>
          {s.runState === "exited" && (
            <p className="mt-3 text-[12.5px] text-ink-3">
              {t("downloads.exited", { code: s.exitCode })}
            </p>
          )}
        </motion.div>

        {/* 日志终端 */}
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.18, duration: 0.35, ease }}
          className="mt-4 overflow-hidden rounded-[16px] bg-terminal-bg shadow-card"
        >
          <div className="flex items-center gap-2 border-b border-white/[0.06] px-4 py-2.5">
            <Terminal size={14} className="text-on-accent/40" />
            <span className="text-[12px] font-medium text-on-accent/50">{t("downloads.gameLog")}</span>
            <span className="ml-auto text-[11px] text-on-accent/30">{t("downloads.lineCount", { count: s.logs.length })}</span>
          </div>
          <div ref={logRef} className="h-64 overflow-y-auto px-4 py-3 font-mono text-[12px] leading-relaxed text-terminal-text">
            {s.logs.length === 0 ? (
              <p className="text-on-accent/25">{t("downloads.noLogs")}</p>
            ) : (
              s.logs.map((line, i) => (
                <p key={i} className={line.startsWith("[RustMCL]") ? "text-terminal-accent" : ""}>
                  {line || "\u00a0"}
                </p>
              ))
            )}
          </div>
        </motion.div>
      </motion.div>
    </div>
  );
}
