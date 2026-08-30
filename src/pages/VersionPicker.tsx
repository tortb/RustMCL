import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import { AnimatePresence, motion } from "framer-motion";
import { useVersionsStore } from "../stores/versions";
import type { VersionFilter } from "../lib/types";

const appleEase = [0.32, 0.72, 0, 1] as const;

const filters: { key: VersionFilter; labelKey: string }[] = [
  { key: "release", labelKey: "versionPicker.filterRelease" },
  { key: "snapshot", labelKey: "versionPicker.filterSnapshot" },
  { key: "all", labelKey: "versionPicker.filterAll" },
];

const listVariants = {
  hidden: {},
  show: { transition: { staggerChildren: 0.035 } },
};

const itemVariants = {
  hidden: { opacity: 0, y: 8 },
  show: {
    opacity: 1,
    y: 0,
    transition: { duration: 0.3, ease: appleEase },
  },
};

export default function VersionPicker() {
  const { t } = useTranslation();
  const { versions, loading, error, filter, query, setFilter, setQuery, load } =
    useVersionsStore();
  const [touched, setTouched] = useState(false);

  useEffect(() => {
    load();
    setTouched(true);
  }, [load]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return versions;
    return versions.filter((v) => v.id.toLowerCase().includes(q));
  }, [versions, query]);

  const versionTypeLabel = (type: string, t: TFunction) => {
    switch (type) {
      case "release":
        return t("versionPicker.filterRelease");
      case "snapshot":
        return t("versionPicker.filterSnapshot");
      case "old_beta":
        return "Beta";
      case "old_alpha":
        return "Alpha";
      default:
        return type;
    }
  };

  return (
    <div className="mx-auto max-w-3xl px-10 py-12">
      <motion.div
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.35, ease: appleEase }}
        className="flex items-end justify-between"
      >
        <div>
          <h1 className="text-[28px] font-bold tracking-tight">{t("versionPicker.title")}</h1>
          <p className="mt-1.5 text-[14px] text-ink-2">
            {t("versionPicker.subtitle")}
          </p>
        </div>
        <button
          onClick={() => load(true)}
          className="rounded-btn bg-accent px-4 py-2 text-[13px] font-medium text-on-accent shadow-card transition-transform duration-100 active:scale-[0.97]"
        >
          {t("versionPicker.refresh")}
        </button>
      </motion.div>

      {/* 过滤 tab + 搜索 */}
      <motion.div
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.06, duration: 0.35, ease: appleEase }}
        className="mt-6 flex items-center justify-between gap-4"
      >
        <div className="flex gap-1 rounded-btn bg-card p-1 shadow-card">
          {filters.map((f) => (
            <button
              key={f.key}
              onClick={() => setFilter(f.key)}
              className={`rounded-[8px] px-3.5 py-1.5 text-[13px] transition-colors duration-150 ${
                filter === f.key
                  ? "bg-accent font-medium text-on-accent"
                  : "text-ink-2 hover:text-ink"
              }`}
            >
              {t(f.labelKey)}
            </button>
          ))}
        </div>
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder={t("versionPicker.searchPlaceholder")}
          className="w-52 rounded-input bg-card px-3.5 py-2 text-[13px] text-ink shadow-card outline-none placeholder:text-ink-3 focus:ring-2 focus:ring-accent/40"
        />
      </motion.div>

      {/* 列表 */}
      <div className="mt-5">
        {loading && !touched ? (
          <div className="flex items-center justify-center py-16">
            <div className="h-7 w-7 animate-spin rounded-full border-2 border-accent border-t-transparent" />
          </div>
        ) : error ? (
          <div className="rounded-card bg-card p-6 text-center shadow-card">
            <p className="text-[13.5px] text-danger-500">{error}</p>
          </div>
        ) : filtered.length === 0 ? (
          <div className="rounded-card bg-card p-6 text-center shadow-card">
            <p className="text-[13.5px] text-ink-3">
              {versions.length === 0 ? t("versionPicker.noVersionData") : t("versionPicker.noMatch")}
            </p>
          </div>
        ) : (
          <AnimatePresence mode="popLayout">
            <motion.ul
              key={filter}
              variants={listVariants}
              initial="hidden"
              animate="show"
              exit={{ opacity: 0, transition: { duration: 0.15 } }}
              className="flex flex-col gap-2"
            >
              {filtered.map((v) => (
                <motion.li key={v.id} variants={itemVariants}>
                  <div className="flex cursor-default items-center justify-between rounded-card bg-card px-5 py-3.5 shadow-card transition-shadow duration-200 hover:shadow-card-hover">
                    <div className="flex items-center gap-3">
                      <span className="text-[15px] font-semibold">{v.id}</span>
                      <span
                        className={`rounded-[6px] px-2 py-0.5 text-[11px] font-medium ${
                          v.version_type === "release"
                            ? "bg-accent/10 text-accent"
                            : "bg-bg text-ink-3"
                        }`}
                      >
                        {versionTypeLabel(v.version_type, t)}
                      </span>
                    </div>
                    <span className="text-[12px] text-ink-3">
                      {new Date(v.release_time).toLocaleDateString("zh-CN")}
                    </span>
                  </div>
                </motion.li>
              ))}
            </motion.ul>
          </AnimatePresence>
        )}
      </div>
    </div>
  );
}
