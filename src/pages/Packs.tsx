import { useEffect, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import {
  Search,
  Image,
  Loader2,
  Trash2,
  Sun,
  Package,
  RefreshCw,
} from "lucide-react";
import { usePacksStore, type PackType } from "../stores/packs";
import { AppSelect } from "../components/AppSelect";

const ease = [0.32, 0.72, 0, 1] as const;

const typeLabels: Record<PackType, string> = {
  resourcepack: "资源包",
  shaderpack: "光影包",
};

export default function Packs() {
  const s = usePacksStore();
  const [query, setQuery] = useState("");

  useEffect(() => {
    s.loadInstances();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="flex-1 overflow-y-auto bg-[#f3f4f6] px-6 py-8">
      <motion.div
        initial={{ opacity: 0, y: 10 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.35, ease }}
        className="mx-auto max-w-3xl"
      >
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-[24px] font-bold tracking-tight text-ink">资源包与光影</h1>
            <p className="mt-1 text-[13px] text-ink-3">管理实例的材质包与光影包</p>
          </div>
          <AppSelect
            value={s.selectedInstanceId}
            onChange={(v) => {
              usePacksStore.setState({ selectedInstanceId: v });
              void usePacksStore.getState().scan();
            }}
            placeholder={s.instances.length === 0 ? "暂无实例" : undefined}
            className="max-w-[220px]"
            options={s.instances.map((inst) => ({ value: inst.id, label: inst.name }))}
          />
        </div>

        {/* 类型切换 */}
        <div className="mt-5 flex items-center gap-2">
          {(["resourcepack", "shaderpack"] as const).map((t) => (
            <button
              key={t}
              onClick={() => s.setType(t)}
              className={`flex items-center gap-1.5 rounded-full px-4 py-1.5 text-[12.5px] font-medium transition-colors ${
                s.type === t
                  ? "bg-accent text-white"
                  : "border border-divider text-ink-2 hover:bg-black/[0.03]"
              }`}
            >
              {t === "resourcepack" ? <Image size={13} /> : <Sun size={13} />}
              {typeLabels[t]}
            </button>
          ))}
          <button
            onClick={() => s.scan()}
            className="ml-auto flex items-center gap-1.5 rounded-[10px] border border-divider px-3.5 py-1.5 text-[12px] text-ink-2 transition-colors hover:bg-black/[0.03]"
          >
            <RefreshCw size={13} />
            重新扫描
          </button>
        </div>

        {/* 光影依赖提示 */}
        {s.type === "shaderpack" && s.shaderSupport && !s.shaderSupport.supported && (
          <div className="mt-4 rounded-[10px] border border-amber-200 bg-amber-50 px-3.5 py-2.5">
            <p className="text-[12px] font-medium text-amber-600">可能无法显示光影</p>
            <p className="mt-0.5 text-[12px] text-amber-700">{s.shaderSupport.message}</p>
          </div>
        )}

        {/* 搜索 */}
        <div className="mt-5 flex gap-3">
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && s.search()}
            placeholder={`搜索 ${typeLabels[s.type]}(Modrinth)...`}
            className="flex-1 rounded-[12px] border border-divider bg-white px-4 py-2.5 text-[13.5px] text-ink outline-none transition-colors focus:border-accent"
          />
          <motion.button
            whileTap={{ scale: 0.97 }}
            onClick={() => s.search()}
            disabled={s.searching}
            className="flex items-center gap-2 rounded-[12px] bg-accent px-4 py-2.5 text-[13.5px] font-semibold text-white transition-colors hover:bg-accent-hover disabled:opacity-40"
          >
            {s.searching ? <Loader2 size={15} className="animate-spin" /> : <Search size={15} />}
            搜索
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
                className="flex items-center gap-3 rounded-[14px] bg-white px-4 py-3 shadow-card"
              >
                <div className="flex h-10 w-10 shrink-0 items-center justify-center overflow-hidden rounded-[12px] bg-[#e8f5e9]">
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
              </motion.div>
            ))}
          </div>
        )}

        {/* 本地列表 */}
        <div className="mt-8">
          <h2 className="text-[13px] font-semibold uppercase tracking-wide text-ink-3">
            已安装 {typeLabels[s.type]}({s.packs.length})
          </h2>
          {s.packs.length === 0 && !s.loading ? (
            <div className="mt-3 flex flex-col items-center gap-2 rounded-[16px] bg-white py-10 shadow-card">
              <Package size={24} className="text-ink-3" strokeWidth={1.5} />
              <p className="text-[13px] text-ink-3">
                目录中没有 {typeLabels[s.type]},可以放到实例的 {s.type === "shaderpack" ? "shaderpacks" : "resourcepacks"} 目录
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
                      pack.enabled ? "border-divider bg-white" : "border-dashed border-ink-3/40 bg-black/[0.02] opacity-70"
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
                        pack.enabled ? "bg-accent" : "bg-black/[0.12]"
                      }`}
                      aria-label={pack.enabled ? "禁用" : "启用"}
                    >
                      <span
                        className={`absolute top-0.5 h-5 w-5 rounded-full bg-white shadow transition-all ${
                          pack.enabled ? "left-[18px]" : "left-0.5"
                        }`}
                      />
                    </button>
                    <button
                      onClick={() => s.remove(pack)}
                      className="shrink-0 rounded-[8px] border border-divider p-1.5 text-ink-3 transition-colors hover:bg-red-50 hover:text-red-500"
                      aria-label="删除"
                    >
                      <Trash2 size={13} />
                    </button>
                  </motion.div>
                ))}
              </AnimatePresence>
            </div>
          )}
        </div>
      </motion.div>
    </div>
  );
}
