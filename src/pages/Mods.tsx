import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { AnimatePresence, motion } from "framer-motion";
import {
  Search,
  Puzzle,
  Loader2,
  Download,
  Trash2,
  X,
  Package,
  ExternalLink,
} from "lucide-react";
import { useModsStore } from "../stores/mods";
import { AppSelect } from "../components/AppSelect";
import type {
  CurseForgeFile,
  ModEntry,
  ModSearchResult,
  ModrinthVersion,
} from "../lib/types";

const ease = [0.32, 0.72, 0, 1] as const;

function formatDownloads(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return String(n);
}

function formatSize(bytes: number): string {
  if (bytes >= 1_048_576) return `${(bytes / 1_048_576).toFixed(1)}MB`;
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(0)}KB`;
  return `${bytes}B`;
}

export default function Mods() {
  const { t } = useTranslation();
  const s = useModsStore();
  const [query, setQuery] = useState("");

  useEffect(() => {
    s.loadInstances();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleSearch = () => {
    s.setQuery(query);
    s.search();
  };

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
            <h1 className="text-[24px] font-bold tracking-tight text-ink">Mod</h1>
            <p className="mt-1 text-[13px] text-ink-3">{t("mods.subtitle")}</p>
          </div>
          {/* 实例选择 */}
          <AppSelect
            value={s.selectedInstanceId}
            onChange={(v) => s.selectInstance(v)}
            placeholder={s.instances.length === 0 ? t("mods.noInstances") : undefined}
            className="max-w-[220px]"
            options={s.instances.map((inst) => ({
              value: inst.id,
              label: `${inst.name} (${inst.config.meta.mc_version} · ${
                inst.config.meta.loader || "vanilla"
              })`,
            }))}
          />
        </div>

        {/* 来源切换 */}
        <div className="mt-5 flex items-center gap-2">
          {(
            [
              { key: "modrinth", label: "Modrinth" },
              { key: "curseforge", label: "CurseForge" },
            ] as const
          ).map((src) => (
            <button
              key={src.key}
              onClick={() => s.setSource(src.key)}
              className={`rounded-full px-4 py-1.5 text-[12.5px] font-medium transition-colors ${
                s.source === src.key
                  ? "bg-accent text-on-accent"
                  : "border border-divider text-ink-2 hover:bg-hover"
              }`}
            >
              {src.label}
            </button>
          ))}
          {s.source === "curseforge" && (
            <span className="text-[11.5px] text-ink-3">{t("mods.cfKeyHint")}</span>
          )}
        </div>

        {/* 搜索框 */}
        <div className="mt-6 flex gap-3">
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleSearch()}
            placeholder={t("mods.searchPlaceholder")}
            className="flex-1 rounded-[12px] border border-divider bg-card px-4 py-2.5 text-[13.5px] text-ink outline-none transition-colors focus:border-accent"
          />
          <motion.button
            whileTap={{ scale: 0.97 }}
            onClick={handleSearch}
            disabled={s.searching || !s.selectedInstanceId}
            className="flex items-center gap-2 rounded-[12px] bg-accent px-4 py-2.5 text-[13.5px] font-semibold text-on-accent transition-colors hover:bg-accent-hover disabled:opacity-40"
          >
            {s.searching ? <Loader2 size={15} className="animate-spin" /> : <Search size={15} />}
            {t("mods.search")}
          </motion.button>
        </div>

        {/* 搜索结果 */}
        {s.searched && (
          <div className="mt-6">
            <SectionTitle title={t("mods.resultCount", { count: s.results.length })} />
            {s.results.length === 0 ? (
              <EmptyHint text={t("mods.noResults")} />
            ) : (
              <div className="mt-3 flex flex-col gap-3">
                <AnimatePresence mode="popLayout">
                  {s.results.map((hit, i) => (
                    <SearchCard key={hit.project_id} hit={hit} index={i} />
                  ))}
                </AnimatePresence>
              </div>
            )}
          </div>
        )}

        {/* 已安装 mod */}
        <div className="mt-8">
          <SectionTitle title={t("mods.installedTitle", { count: s.installed.length })} />
          {s.installed.length === 0 && !s.loadingInstalled ? (
            <EmptyHint text={t("mods.emptyInstalled")} />
          ) : (
            <div className="mt-3 flex flex-col gap-3">
              {s.installed.map((mod, i) => (
                <InstalledCard key={mod.id} mod={mod} index={i} />
              ))}
            </div>
          )}
        </div>
      </motion.div>

      {/* 版本选择弹窗 */}
      <VersionModal />
    </div>
  );
}

function SectionTitle({ title }: { title: string }) {
  return <h2 className="text-[13px] font-semibold uppercase tracking-wide text-ink-3">{title}</h2>;
}

function EmptyHint({ text }: { text: string }) {
  return (
    <div className="mt-3 flex flex-col items-center gap-2 rounded-[16px] bg-card py-10 shadow-card">
      <Package size={24} className="text-ink-3" strokeWidth={1.5} />
      <p className="text-[13px] text-ink-3">{text}</p>
    </div>
  );
}

function SearchCard({ hit, index }: { hit: ModSearchResult; index: number }) {
  const { t } = useTranslation();
  const s = useModsStore();
  return (
    <motion.div
      layout
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, scale: 0.97 }}
      transition={{ delay: 0.03 * index, duration: 0.3, ease }}
      className="flex items-center gap-4 rounded-[16px] bg-card px-4 py-3.5 shadow-card"
    >
      <ModIcon hit={hit} />
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="truncate text-[14.5px] font-semibold text-ink">{hit.title}</span>
        </div>
        <p className="mt-0.5 truncate text-[12.5px] text-ink-3">{hit.description}</p>
        <div className="mt-1.5 flex items-center gap-3 text-[11.5px] text-ink-3">
          <span className="flex items-center gap-1">
            <Download size={11} />
            {formatDownloads(hit.downloads)}
          </span>
          {hit.categories.slice(0, 2).map((c) => (
            <span key={c} className="rounded-full bg-badge-bg px-2 py-0.5 font-medium text-badge-text">
              {c}
            </span>
          ))}
        </div>
      </div>
      <motion.button
        whileTap={{ scale: 0.95 }}
        onClick={() => s.openVersions(hit)}
        className="shrink-0 rounded-[10px] bg-accent px-3.5 py-2 text-[12.5px] font-semibold text-on-accent transition-colors hover:bg-accent-hover"
      >
        {t("mods.install")}
      </motion.button>
    </motion.div>
  );
}

function ModIcon({ hit }: { hit: ModSearchResult }) {
  return (
    <div className="flex h-11 w-11 shrink-0 items-center justify-center overflow-hidden rounded-[12px] bg-nav-active">
      {hit.icon_url ? (
        <img src={hit.icon_url} alt="" className="h-full w-full object-cover" draggable={false} />
      ) : (
        <Puzzle size={20} className="text-accent" strokeWidth={1.8} />
      )}
    </div>
  );
}

function InstalledCard({ mod, index }: { mod: ModEntry; index: number }) {
  const { t } = useTranslation();
  const s = useModsStore();
  return (
    <motion.div
      layout
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, scale: 0.97 }}
      transition={{ delay: 0.03 * index, duration: 0.3, ease }}
      className="flex items-center gap-4 rounded-[16px] bg-card px-4 py-3 shadow-card"
    >
      <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-[12px] bg-badge-bg">
        <Puzzle size={18} className="text-badge-text" strokeWidth={1.8} />
      </div>
      <div className="min-w-0 flex-1">
        <span className="block truncate text-[14px] font-medium text-ink">{mod.file_name}</span>
        <span className="mt-0.5 block text-[11.5px] text-ink-3">{mod.source ?? "local"}</span>
      </div>
      {/* 启用开关 */}
      <button
        onClick={() => s.toggle(mod, !mod.enabled)}
        className={`relative h-6 w-10 shrink-0 rounded-full transition-colors ${
          mod.enabled ? "bg-accent" : "bg-hover"
        }`}
        aria-label={mod.enabled ? t("mods.disable") : t("mods.enable")}
      >
        <span
          className={`absolute top-0.5 h-5 w-5 rounded-full bg-card shadow transition-all ${
            mod.enabled ? "left-[18px]" : "left-0.5"
          }`}
        />
      </button>
      <button
        onClick={() => {
          if (confirm(t("mods.deleteConfirm", { name: mod.file_name }))) s.remove(mod);
        }}
        className="shrink-0 rounded-[10px] border border-divider p-2 text-ink-3 transition-colors hover:bg-danger-50 hover:text-danger-500"
        aria-label={t("mods.delete")}
      >
        <Trash2 size={14} />
      </button>
    </motion.div>
  );
}

function VersionModal() {
  const { t } = useTranslation();
  const s = useModsStore();
  const hit = s.versionModalProject;
  const isCf = hit?.source === "curseforge";
  return (
    <AnimatePresence>
      {hit && (
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
            className="w-[460px] rounded-[20px] bg-card p-7 shadow-[0_24px_64px_rgba(0,0,0,0.16)]"
          >
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <ModIcon hit={hit} />
                <h2 className="text-[17px] font-bold tracking-tight text-ink">{hit.title}</h2>
              </div>
              <button
                onClick={s.closeVersions}
                disabled={s.installing}
                className="rounded-full p-1.5 text-ink-3 transition-colors hover:bg-hover"
                aria-label={t("common.close")}
              >
                <X size={16} />
              </button>
            </div>

            <p className="mt-3 text-[12.5px] text-ink-3">
              {t("mods.versionHint")}
            </p>

            {/* CurseForge 禁止第三方分发提示 */}
            {isCf && hit.allow_mod_distribution === false && (
              <div className="mt-3 rounded-[10px] border border-warning-50 bg-warning-50 px-3.5 py-2.5">
                <p className="text-[12px] font-medium text-warning-600">
                  {t("mods.distributionBlocked")}
                </p>
                <p className="mt-0.5 text-[12px] text-warning-700">
                  {t("mods.distributionHint")}
                </p>
              </div>
            )}

            {/* 依赖检查横幅(非阻断式) */}
            {s.depResult && !isCf && (
              <div className="mt-3 rounded-[10px] border border-warning-50 bg-warning-50 px-3.5 py-2.5">
                {s.depResult.conflicts.map((c, i) => (
                  <p key={i} className="text-[12px] font-medium text-warning-600">
                    {t("mods.conflict", { name: c })}
                  </p>
                ))}
                {s.depResult.missing_required.length > 0 && (
                  <div className="mt-1">
                    <p className="text-[12px] font-medium text-warning-600">
                      {t("mods.missingDeps", { count: s.depResult.missing_required.length })}
                    </p>
                    <div className="mt-1 flex flex-col gap-1">
                      {s.depResult.missing_required.map((dep, i) => (
                        <div key={i} className="flex items-center justify-between text-[12px] text-warning-700">
                          <span className="truncate">
                            {dep.file_name || dep.project_id}
                          </span>
                          {dep.version_id && (
                            <button
                              onClick={() => s.installDep(dep.version_id)}
                              disabled={s.installing}
                              className="ml-2 shrink-0 rounded-[6px] bg-warning-500 px-2 py-1 text-[11px] font-medium text-on-accent transition-colors hover:bg-warning-600 disabled:opacity-50"
                            >
                              {t("mods.autoInstall")}
                            </button>
                          )}
                        </div>
                      ))}
                    </div>
                  </div>
                )}
                {s.depResult.ok && (
                  <p className="text-[12px] text-success-600">{t("mods.depsOk")}</p>
                )}
              </div>
            )}

            <div className="mt-4 max-h-[320px] overflow-y-auto">
              {s.loadingVersions ? (
                <div className="flex items-center justify-center py-10 text-ink-3">
                  <Loader2 size={18} className="animate-spin" />
                </div>
              ) : isCf ? s.cfFiles.length === 0 ? (
                <p className="py-8 text-center text-[13px] text-ink-3">{t("mods.noCompatibleFiles")}</p>
              ) : (
                <div className="flex flex-col gap-2.5">
                  <AnimatePresence initial={false}>
                    {s.cfFiles.map((f, i) => (
                      <CfFileRow key={f.file_id} file={f} index={i} />
                    ))}
                  </AnimatePresence>
                </div>
              ) : s.versions.length === 0 ? (
                <p className="py-8 text-center text-[13px] text-ink-3">
                  {t("mods.noCompatibleVersions")}
                </p>
              ) : (
                <div className="flex flex-col gap-2.5">
                  <AnimatePresence initial={false}>
                    {s.versions.map((v, i) => (
                      <VersionRow key={v.id} version={v} index={i} />
                    ))}
                  </AnimatePresence>
                </div>
              )}
            </div>

            <a
              href={
                isCf
                  ? `https://www.curseforge.com/minecraft/mc-mods/${hit.slug}`
                  : `https://modrinth.com/mod/${hit.slug}`
              }
              target="_blank"
              rel="noreferrer"
              className="mt-4 flex items-center gap-1 text-[12px] text-ink-3 transition-colors hover:text-accent"
            >
              <ExternalLink size={12} />
              {t("mods.viewProject")}
            </a>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}

function VersionRow({ version, index }: { version: ModrinthVersion; index: number }) {
  const { t } = useTranslation();
  const s = useModsStore();
  const file = version.files.find((f) => f.primary) ?? version.files[0];
  return (
    <motion.div
      initial={{ opacity: 0, x: 8 }}
      animate={{ opacity: 1, x: 0 }}
      transition={{ delay: 0.03 * index, duration: 0.25, ease }}
      className="flex items-center gap-3 rounded-[12px] border border-divider px-3.5 py-2.5"
    >
      <div className="min-w-0 flex-1">
        <span className="block truncate text-[13.5px] font-medium text-ink">
          {version.name || version.version_number}
        </span>
        <span className="mt-0.5 block text-[11.5px] text-ink-3">
          {version.loaders.join(", ")} · {file ? formatSize(file.size) : ""}
        </span>
      </div>
      <button
        onClick={() => s.checkDeps(version.id)}
        className="shrink-0 rounded-[8px] border border-divider px-2 py-1 text-[11.5px] text-ink-3 transition-colors hover:text-warning-600"
      >
        {t("mods.dependencies")}
      </button>
      <motion.button
        whileTap={{ scale: 0.95 }}
        onClick={() => s.install(version)}
        disabled={s.installing}
        className="shrink-0 rounded-[10px] bg-accent px-3 py-1.5 text-[12px] font-semibold text-on-accent transition-colors hover:bg-accent-hover disabled:opacity-50"
      >
        {s.installing ? <Loader2 size={12} className="animate-spin" /> : t("mods.install")}
      </motion.button>
    </motion.div>
  );
}

function CfFileRow({ file, index }: { file: CurseForgeFile; index: number }) {
  const { t } = useTranslation();
  const s = useModsStore();
  return (
    <motion.div
      initial={{ opacity: 0, x: 8 }}
      animate={{ opacity: 1, x: 0 }}
      transition={{ delay: 0.03 * index, duration: 0.25, ease }}
      className="flex items-center gap-3 rounded-[12px] border border-divider px-3.5 py-2.5"
    >
      <div className="min-w-0 flex-1">
        <span className="block truncate text-[13.5px] font-medium text-ink">{file.filename}</span>
        <span className="mt-0.5 block text-[11.5px] text-ink-3">{formatSize(file.size)}</span>
      </div>
      <motion.button
        whileTap={{ scale: 0.95 }}
        onClick={() => s.installCfFile(file)}
        disabled={s.installing}
        className="shrink-0 rounded-[10px] bg-accent px-3 py-1.5 text-[12px] font-semibold text-on-accent transition-colors hover:bg-accent-hover disabled:opacity-50"
      >
        {s.installing ? <Loader2 size={12} className="animate-spin" /> : t("mods.install")}
      </motion.button>
    </motion.div>
  );
}
