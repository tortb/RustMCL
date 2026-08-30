import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { AnimatePresence, motion } from "framer-motion";
import {
  Search,
  Image,
  Loader2,
  Trash2,
  Sun,
  Package,
  RefreshCw,
  Plus,
  X,
  Download,
  CheckCircle2,
} from "lucide-react";
import { usePacksStore, type PackType } from "../stores/packs";
import { AppSelect } from "../components/AppSelect";
import type { ModrinthVersion } from "../lib/types";

const ease = [0.32, 0.72, 0, 1] as const;

export default function Packs() {
  const { t } = useTranslation();
  const s = usePacksStore();
  const [query, setQuery] = useState("");
  const typeLabel = (tp: PackType) => (tp === "resourcepack" ? t("packs.type.resourcepack") : t("packs.type.shaderpack"));

  useEffect(() => {
    s.loadInstances();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="flex-1 overflow-y-auto bg-bg px-6 py-8">
      <motion.div
        initial={{ opacity: 0, y: 10 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.35, ease }}
        className="mx-auto max-w-3xl"
      >
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-[24px] font-bold tracking-tight text-ink">{t("packs.title")}</h1>
            <p className="mt-1 text-[13px] text-ink-3">{t("packs.subtitle")}</p>
          </div>
          <AppSelect
            value={s.selectedInstanceId}
            onChange={(v) => {
              usePacksStore.setState({ selectedInstanceId: v });
              void usePacksStore.getState().scan();
            }}
            placeholder={s.instances.length === 0 ? t("packs.noInstances") : undefined}
            className="max-w-[220px]"
            options={s.instances.map((inst) => ({ value: inst.id, label: inst.name }))}
          />
        </div>

        {/* 类型切换 */}
        <div className="mt-5 flex items-center gap-2">
          {(["resourcepack", "shaderpack"] as const).map((type) => (
            <button
              key={type}
              onClick={() => s.setType(type)}
              className={`flex items-center gap-1.5 rounded-full px-4 py-1.5 text-[12.5px] font-medium transition-colors ${
                s.type === type
                  ? "bg-accent text-on-accent"
                  : "border border-divider text-ink-2 hover:bg-hover"
              }`}
            >
              {type === "resourcepack" ? <Image size={13} /> : <Sun size={13} />}
              {typeLabel(type)}
            </button>
          ))}
          <button
            onClick={() => s.scan()}
            className="ml-auto flex items-center gap-1.5 rounded-[10px] border border-divider px-3.5 py-1.5 text-[12px] text-ink-2 transition-colors hover:bg-hover"
          >
            <RefreshCw size={13} />
            {t("packs.rescan")}
          </button>
        </div>

        {/* 光影依赖提示 */}
        {s.type === "shaderpack" && s.shaderSupport && !s.shaderSupport.supported && (
          <div className="mt-4 rounded-[10px] border border-warning-50 bg-warning-50 px-3.5 py-2.5">
            <p className="text-[12px] font-medium text-warning-600">{t("packs.shaderWarning")}</p>
            <p className="mt-0.5 text-[12px] text-warning-700">{s.shaderSupport.message}</p>
          </div>
        )}

        {/* 搜索 */}
        <div className="mt-5 flex gap-3">
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && s.search()}
            placeholder={t("packs.searchPlaceholder", { type: typeLabel(s.type) })}
            className="flex-1 rounded-[12px] border border-divider bg-card px-4 py-2.5 text-[13.5px] text-ink outline-none transition-colors focus:border-accent"
          />
          <motion.button
            whileTap={{ scale: 0.97 }}
            onClick={() => s.search()}
            disabled={s.searching}
            className="flex items-center gap-2 rounded-[12px] bg-accent px-4 py-2.5 text-[13.5px] font-semibold text-on-accent transition-colors hover:bg-accent-hover disabled:opacity-40"
          >
            {s.searching ? <Loader2 size={15} className="animate-spin" /> : <Search size={15} />}
            {t("packs.search")}
          </motion.button>
        </div>

        {/* 搜索结果 */}
        {s.searching && (
          <div className="mt-6 flex justify-center text-ink-3">
            <Loader2 size={18} className="animate-spin" />
          </div>
        )}
        {!s.searching && s.results.length > 0 && (
          <div className="mt-4 flex flex-col gap-2">
            {s.results.map((hit, i) => (
              <motion.div
                key={hit.project_id}
                initial={{ opacity: 0, y: 6 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: 0.03 * i, duration: 0.25, ease }}
                className="flex items-center gap-3 rounded-[14px] bg-card px-4 py-3 shadow-card"
              >
                <div className="flex h-10 w-10 shrink-0 items-center justify-center overflow-hidden rounded-[12px] bg-nav-active">
                  {hit.icon_url ? (
                    <img src={hit.icon_url} alt="" className="h-full w-full object-cover" />
                  ) : (
                    <Image size={18} className="text-accent" />
                  )}
                </div>
                <div className="min-w-0 flex-1">
                  <span className="block truncate text-[14px] font-medium text-ink">{hit.title}</span>
                  <span className="block truncate text-[12px] text-ink-3">{hit.description}</span>
                </div>
                <motion.button
                  whileTap={{ scale: 0.97 }}
                  onClick={() => s.openVersions(hit)}
                  disabled={s.installing}
                  className="flex shrink-0 items-center gap-1.5 rounded-[10px] bg-accent px-3 py-1.5 text-[12px] font-semibold text-on-accent transition-colors hover:bg-accent-hover disabled:opacity-40"
                >
                  <Plus size={14} strokeWidth={2.4} />
                  {t("packs.add")}
                </motion.button>
              </motion.div>
            ))}
          </div>
        )}

        {/* 本地列表 */}
        <div className="mt-8">
          <h2 className="text-[13px] font-semibold uppercase tracking-wide text-ink-3">
            {t("packs.installedTitle", { type: typeLabel(s.type), count: s.packs.length })}
          </h2>
          {s.packs.length === 0 && !s.loading ? (
            <div className="mt-3 flex flex-col items-center gap-2 rounded-[16px] bg-card py-10 shadow-card">
              <Package size={24} className="text-ink-3" strokeWidth={1.5} />
              <p className="text-[13px] text-ink-3">
                {t("packs.emptyHint", { type: typeLabel(s.type), dir: s.type === "shaderpack" ? "shaderpacks" : "resourcepacks" })}
              </p>
            </div>
          ) : (
            <div className="mt-3 flex flex-col gap-2">
              <AnimatePresence mode="popLayout">
                {s.packs.map((pack, i) => (
                  <motion.div
                    key={pack.id}
                    layout
                    initial={{ opacity: 0, y: 6 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0, scale: 0.97 }}
                    transition={{ delay: 0.03 * i, duration: 0.25, ease }}
                    className={`flex items-center gap-3 rounded-[12px] border px-3.5 py-2.5 ${
                      pack.enabled ? "border-divider bg-card" : "border-dashed border-ink-3/40 bg-hover opacity-70"
                    }`}
                  >
                    <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-[10px] bg-badge-bg">
                      <Image size={15} className="text-badge-text" />
                    </div>
                    <span className="min-w-0 flex-1 truncate text-[13px] font-medium text-ink">
                      {pack.file_name}
                    </span>
                    <button
                      onClick={() => s.toggle(pack)}
                      className={`relative h-6 w-10 shrink-0 rounded-full transition-colors ${
                        pack.enabled ? "bg-accent" : "bg-hover"
                      }`}
                      aria-label={pack.enabled ? t("packs.disable") : t("packs.enable")}
                    >
                      <span
                        className={`absolute top-0.5 h-5 w-5 rounded-full bg-card shadow transition-all ${
                          pack.enabled ? "left-[18px]" : "left-0.5"
                        }`}
                      />
                    </button>
                    <button
                      onClick={() => s.remove(pack)}
                      className="shrink-0 rounded-[8px] border border-divider p-1.5 text-ink-3 transition-colors hover:bg-danger-50 hover:text-danger-500"
                      aria-label={t("packs.delete")}
                    >
                      <Trash2 size={13} />
                    </button>
                  </motion.div>
                ))}
              </AnimatePresence>
            </div>
          )}
        </div>

        {/* 版本选择弹窗(从搜索结果添加) */}
        <AnimatePresence>
          {s.versionModalProject && (
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              transition={{ duration: 0.2 }}
              className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 backdrop-blur-sm"
              onClick={s.closeVersions}
            >
              <motion.div
                initial={{ opacity: 0, scale: 0.95, y: 8 }}
                animate={{ opacity: 1, scale: 1, y: 0 }}
                exit={{ opacity: 0, scale: 0.95, y: 8 }}
                transition={{ duration: 0.28, ease }}
                onClick={(e) => e.stopPropagation()}
                className="w-[440px] rounded-[20px] bg-card p-6 shadow-[0_24px_64px_rgba(0,0,0,0.16)]"
              >
                <div className="flex items-center justify-between">
                  <div>
                    <h2 className="text-[17px] font-bold tracking-tight text-ink">
                      {s.versionModalProject.title}
                    </h2>
                    <p className="mt-0.5 text-[12px] text-ink-3">
                      {t("packs.versionHint", { type: typeLabel(s.type) })}
                    </p>
                  </div>
                  <button
                    onClick={s.closeVersions}
                    className="rounded-full p-1.5 text-ink-3 transition-colors hover:bg-hover"
                    aria-label={t("common.close")}
                  >
                    <X size={16} />
                  </button>
                </div>

                <div className="mt-4 max-h-[320px] overflow-y-auto">
                  {s.loadingVersions ? (
                    <div className="flex justify-center py-10 text-ink-3">
                      <Loader2 size={20} className="animate-spin" />
                    </div>
                  ) : s.versions.length === 0 ? (
                    <p className="py-8 text-center text-[13px] text-ink-3">
                      {t("packs.noCompatibleVersion", { type: typeLabel(s.type) })}
                    </p>
                  ) : (
                    <div className="flex flex-col gap-2">
                      {s.versions.map((v: ModrinthVersion) => (
                        <button
                          key={v.id}
                          onClick={() => s.install(v)}
                          disabled={s.installing}
                          className="flex items-center gap-3 rounded-[12px] border border-divider px-3.5 py-3 text-left transition-colors hover:bg-hover disabled:opacity-50"
                        >
                          <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-[10px] bg-badge-bg">
                            {s.installing ? (
                              <Loader2 size={14} className="animate-spin text-badge-text" />
                            ) : (
                              <Download size={14} className="text-badge-text" />
                            )}
                          </div>
                          <div className="min-w-0 flex-1">
                            <span className="block truncate text-[13.5px] font-medium text-ink">
                              {v.name || v.version_number}
                            </span>
                            <span className="block truncate text-[11.5px] text-ink-3">
                              {v.game_versions.slice(0, 3).join(", ")}
                              {v.loaders.length > 0 && ` · ${v.loaders.slice(0, 2).join("/")} `}
                            </span>
                          </div>
                          {s.installing && <CheckCircle2 size={16} className="shrink-0 text-accent" />}
                        </button>
                      ))}
                    </div>
                  )}
                </div>

                {s.installError && (
                  <p className="mt-3 rounded-[10px] bg-danger-50 px-3.5 py-2.5 text-[12.5px] text-danger-600">
                    {t("packs.installError", { error: s.installError })}
                  </p>
                )}
              </motion.div>
            </motion.div>
          )}
        </AnimatePresence>
      </motion.div>
    </div>
  );
}
