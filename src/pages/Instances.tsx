import { useEffect, useRef, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  Plus,
  Play,
  Loader2,
  Trash2,
  Pencil,
  ChevronDown,
  Terminal,
  X,
  Box,
} from "lucide-react";
import { useInstanceStore } from "../stores/instance";
import type {
  DownloadProgress,
  GameExit,
  GameLog,
  InstanceDetail,
  InstanceInput,
  Loader,
  LoaderInstallFinished,
} from "../lib/types";

const ease = [0.32, 0.72, 0, 1] as const;

const loaderLabels: Record<Loader, string> = {
  vanilla: "原版",
  forge: "Forge",
  fabric: "Fabric",
  quilt: "Quilt",
};

export default function Instances() {
  const s = useInstanceStore();
  const [expanded, setExpanded] = useState<string | null>(null);
  const logRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    s.loadInstances();
    s.loadVersions();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    let unlisteners: UnlistenFn[] = [];
    let mounted = true;
    Promise.all([
      listen<GameLog>("game-log", (e) => s.appendLog(e.payload.line)),
      listen<GameExit>("game-exit", (e) => {
        s.appendLog(`[RustMCL] 游戏进程退出,退出码 ${e.payload.code}`);
        s.setRunning(null);
      }),
      // 加载器安装进度/结束(仅在有安装任务时响应)
      listen<DownloadProgress>("download-progress", (e) => {
        const st = useInstanceStore.getState();
        if (st.installingId) st.setInstallProgress(e.payload);
      }),
      listen<LoaderInstallFinished>("loader-install-finished", (e) => {
        const st = useInstanceStore.getState();
        if (!e.payload.ok) st.setInstallError(e.payload.error);
        st.setInstalling(null);
        st.setInstallProgress(null);
      }),
    ]).then((un) => {
      if (mounted) unlisteners = un;
    });
    return () => {
      mounted = false;
      unlisteners.forEach((u) => u());
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const el = logRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [s.logs]);

  const handleDelete = async (inst: InstanceDetail) => {
    if (confirm(`确定删除实例「${inst.name}」吗?该操作会删除实例目录。`)) {
      await s.remove(inst.id);
    }
  };

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
            <h1 className="text-[24px] font-bold tracking-tight text-ink">实例</h1>
            <p className="mt-1 text-[13px] text-ink-3">管理你的游戏实例,每个实例独立配置</p>
          </div>
          <motion.button
            whileTap={{ scale: 0.97 }}
            onClick={s.openCreate}
            className="flex items-center gap-2 rounded-[12px] bg-accent px-4 py-2.5 text-[13.5px] font-semibold text-white transition-colors hover:bg-accent-hover"
          >
            <Plus size={16} strokeWidth={2.4} />
            新建实例
          </motion.button>
        </div>

        {/* 实例卡片列表 */}
        {s.instances.length === 0 && !s.loading ? (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ delay: 0.15, duration: 0.3 }}
            className="mt-8 flex flex-col items-center gap-3 rounded-[16px] bg-white py-14 shadow-card"
          >
            <Box size={28} className="text-ink-3" strokeWidth={1.5} />
            <p className="text-[13.5px] text-ink-3">还没有实例,点击右上角创建一个</p>
          </motion.div>
        ) : (
          <div className="mt-6 flex flex-col gap-3">
            <AnimatePresence mode="popLayout">
              {s.instances.map((inst, i) => (
                <motion.div
                  key={inst.id}
                  layout
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, scale: 0.97 }}
                  transition={{ delay: 0.04 * i, duration: 0.3, ease }}
                  className="overflow-hidden rounded-[16px] bg-white shadow-card"
                >
                  {/* 卡片头部 */}
                  <div className="flex items-center gap-4 px-5 py-4">
                    <button
                      onClick={() => setExpanded(expanded === inst.id ? null : inst.id)}
                      className="flex flex-1 items-center gap-3 text-left"
                    >
                      <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-[12px] bg-[#e8f5e9] text-accent">
                        <Box size={19} strokeWidth={1.8} />
                      </div>
                      <div className="flex-1">
                        <div className="flex items-center gap-2">
                          <span className="text-[14.5px] font-semibold text-ink">{inst.name}</span>
                          <span className="rounded-full bg-badge-bg px-2 py-0.5 text-[11px] font-medium text-badge-text">
                            {loaderLabels[(inst.loader ?? "vanilla") as Loader]}
                          </span>
                        </div>
                        <p className="mt-0.5 text-[12.5px] text-ink-3">
                          MC {inst.mc_version} · {inst.config.jvm.min_memory}-
                          {inst.config.jvm.max_memory}MB · {inst.config.game.resolution.width}×
                          {inst.config.game.resolution.height}
                        </p>
                      </div>
                      <ChevronDown
                        size={16}
                        className={`shrink-0 text-ink-3 transition-transform duration-200 ${expanded === inst.id ? "rotate-180" : ""}`}
                      />
                    </button>
                    <div className="flex shrink-0 items-center gap-2">
                      <motion.button
                        whileTap={{ scale: 0.95 }}
                        onClick={() => s.launch(inst.id)}
                        disabled={s.runningId !== null}
                        className="flex items-center gap-1.5 rounded-[10px] bg-accent px-3.5 py-2 text-[12.5px] font-semibold text-white transition-colors hover:bg-accent-hover disabled:opacity-40"
                      >
                        {s.runningId === inst.id ? (
                          <Loader2 size={14} className="animate-spin" />
                        ) : (
                          <Play size={13} fill="white" strokeWidth={0} />
                        )}
                        {s.runningId === inst.id ? "启动中" : "启动"}
                      </motion.button>
                      <motion.button
                        whileTap={{ scale: 0.95 }}
                        onClick={() => s.openEdit(inst)}
                        className="rounded-[10px] border border-divider p-2 text-ink-2 transition-colors hover:bg-black/[0.03]"
                        aria-label="编辑"
                      >
                        <Pencil size={14} />
                      </motion.button>
                      <motion.button
                        whileTap={{ scale: 0.95 }}
                        onClick={() => handleDelete(inst)}
                        className="rounded-[10px] border border-divider p-2 text-ink-3 transition-colors hover:bg-red-50 hover:text-red-500"
                        aria-label="删除"
                      >
                        <Trash2 size={14} />
                      </motion.button>
                    </div>
                  </div>

                  {/* 加载器安装进度 */}
                  {s.installingId === inst.id && (
                    <div className="border-t border-divider px-5 py-3">
                      <div className="flex items-center justify-between text-[12px]">
                        <span className="flex items-center gap-1.5 text-ink-2">
                          <Loader2 size={12} className="animate-spin text-accent" />
                          正在安装 {loaderLabels[(inst.config.meta.loader ?? "vanilla") as Loader]}...
                        </span>
                        {s.installProgress && (
                          <span className="text-ink-3">
                            {s.installProgress.current}/{s.installProgress.total}
                          </span>
                        )}
                      </div>
                      <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-black/[0.06]">
                        <div
                          className="h-full rounded-full bg-accent transition-all duration-300"
                          style={{
                            width: s.installProgress
                              ? `${Math.min(100, (s.installProgress.current / Math.max(1, s.installProgress.total)) * 100)}%`
                              : "8%",
                          }}
                        />
                      </div>
                      {s.installError && (
                        <p className="mt-2 text-[12px] text-red-500">安装失败:{s.installError}</p>
                      )}
                    </div>
                  )}

                  {/* 展开详情(layout animation) */}
                  <AnimatePresence initial={false}>
                    {expanded === inst.id && (
                      <motion.div
                        key="detail"
                        initial={{ height: 0, opacity: 0 }}
                        animate={{ height: "auto", opacity: 1 }}
                        exit={{ height: 0, opacity: 0 }}
                        transition={{ duration: 0.28, ease }}
                        className="overflow-hidden"
                      >
                        <div className="grid grid-cols-3 gap-4 border-t border-divider px-5 py-4 text-[12.5px]">
                          <div>
                            <p className="text-ink-3">启动器</p>
                            <p className="mt-1 font-medium text-ink">
                              {loaderLabels[(inst.config.meta.loader ?? "vanilla") as Loader]}{" "}
                              {inst.config.meta.loader_version}
                            </p>
                          </div>
                          <div>
                            <p className="text-ink-3">游戏目录</p>
                            <p className="mt-1 break-all font-mono text-[11.5px] text-ink-2">
                              {inst.game_dir}
                            </p>
                          </div>
                          <div>
                            <p className="text-ink-3">内存</p>
                            <p className="mt-1 font-medium text-ink">
                              {inst.config.jvm.min_memory} - {inst.config.jvm.max_memory} MB
                            </p>
                          </div>
                          <div>
                            <p className="text-ink-3">分辨率</p>
                            <p className="mt-1 font-medium text-ink">
                              {inst.config.game.resolution.width} ×{" "}
                              {inst.config.game.resolution.height}
                            </p>
                          </div>
                          <div>
                            <p className="text-ink-3">全屏</p>
                            <p className="mt-1 font-medium text-ink">
                              {inst.config.game.fullscreen ? "是" : "否"}
                            </p>
                          </div>
                          <div>
                            <p className="text-ink-3">创建时间</p>
                            <p className="mt-1 font-medium text-ink">
                              {new Date(inst.created_at * 1000).toLocaleDateString("zh-CN")}
                            </p>
                          </div>
                        </div>
                      </motion.div>
                    )}
                  </AnimatePresence>
                </motion.div>
              ))}
            </AnimatePresence>
          </div>
        )}

        {/* 日志终端(启动实例时显示) */}
        {s.logs.length > 0 && (
          <motion.div
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.3, ease }}
            className="mt-5 overflow-hidden rounded-[16px] bg-[#1a1a1a] shadow-card"
          >
            <div className="flex items-center gap-2 border-b border-white/[0.06] px-4 py-2.5">
              <Terminal size={14} className="text-white/40" />
              <span className="text-[12px] font-medium text-white/50">运行日志</span>
              <span className="ml-auto text-[11px] text-white/30">{s.logs.length} 行</span>
            </div>
            <div
              ref={logRef}
              className="h-56 overflow-y-auto px-4 py-3 font-mono text-[12px] leading-relaxed text-[#d4d4d4]"
            >
              {s.logs.map((line, i) => (
                <p key={i} className={line.startsWith("[RustMCL]") ? "text-[#7cb342]" : ""}>
                  {line || "\u00a0"}
                </p>
              ))}
            </div>
          </motion.div>
        )}
      </motion.div>

      {/* 创建/编辑弹窗 */}
      <InstanceModal />
    </div>
  );
}

function InstanceModal() {
  const s = useInstanceStore();
  const editing = s.editing;

  // 表单初始值
  const [name, setName] = useState("");
  const [mcVersion, setMcVersion] = useState("");
  const [loader, setLoader] = useState<Loader>("vanilla");
  const [forgeVersion, setForgeVersion] = useState("");
  const [minMemory, setMinMemory] = useState(1024);
  const [maxMemory, setMaxMemory] = useState(4096);
  const [width, setWidth] = useState(1280);
  const [height, setHeight] = useState(720);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  // 打开弹窗时预填(编辑)或重置(新建)
  useEffect(() => {
    if (s.modalOpen) {
      setName(editing?.name ?? "");
      setMcVersion(editing?.mc_version ?? "");
      setLoader((editing?.loader ?? "vanilla") as Loader);
      setForgeVersion(editing?.config.meta.loader_version ?? "");
      setMinMemory(editing?.config.jvm.min_memory ?? 1024);
      setMaxMemory(editing?.config.jvm.max_memory ?? 4096);
      setWidth(editing?.config.game.resolution.width ?? 1280);
      setHeight(editing?.config.game.resolution.height ?? 720);
      setError("");
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [s.modalOpen]);

  // loader 选 Forge 且选定 MC 版本时,拉取可用 Forge 版本
  useEffect(() => {
    if (s.modalOpen && loader === "forge" && mcVersion) {
      s.loadForgeVersions(mcVersion);
    } else {
      setForgeVersion("");
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [loader, mcVersion, s.modalOpen]);

  const submit = async () => {
    if (!name.trim()) {
      setError("请填写实例名称");
      return;
    }
    if (!editing && !mcVersion) {
      setError("请选择 Minecraft 版本");
      return;
    }
    if (loader === "forge" && !forgeVersion) {
      setError("请选择 Forge 版本");
      return;
    }
    setSaving(true);
    setError("");
    const input: InstanceInput = {
      name: name.trim(),
      loader,
      min_memory: minMemory,
      max_memory: maxMemory,
      width,
      height,
    };
    if (!editing) input.mc_version = mcVersion;
    // fix: Forge 版本由用户选择,其它加载器自动解析
    if (loader === "forge") input.loader_version = forgeVersion;
    try {
      await s.save(input);
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <AnimatePresence>
      {s.modalOpen && (
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
            className="w-[440px] rounded-[20px] bg-white p-7 shadow-[0_24px_64px_rgba(0,0,0,0.16)]"
          >
            <div className="flex items-center justify-between">
              <h2 className="text-[18px] font-bold tracking-tight text-ink">
                {editing ? "编辑实例" : "新建实例"}
              </h2>
              <button
                onClick={s.closeModal}
                className="rounded-full p-1.5 text-ink-3 transition-colors hover:bg-black/[0.05]"
                aria-label="关闭"
              >
                <X size={16} />
              </button>
            </div>

            <div className="mt-5 flex flex-col gap-4">
              <div>
                <label className="text-[12.5px] font-medium text-ink-2">实例名称</label>
                <input
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="我的生存"
                  className="mt-1.5 w-full rounded-[10px] border border-divider bg-white px-3.5 py-2.5 text-[13.5px] text-ink outline-none transition-colors focus:border-accent"
                />
              </div>

              {!editing && (
                <div>
                  <label className="text-[12.5px] font-medium text-ink-2">Minecraft 版本</label>
                  <select
                    value={mcVersion}
                    onChange={(e) => setMcVersion(e.target.value)}
                    className="mt-1.5 w-full rounded-[10px] border border-divider bg-white px-3.5 py-2.5 text-[13.5px] text-ink outline-none transition-colors focus:border-accent"
                  >
                    <option value="">选择版本</option>
                    {s.versions.map((v) => (
                      <option key={v.id} value={v.id}>
                        {v.id}
                      </option>
                    ))}
                  </select>
                </div>
              )}

              <div>
                <label className="text-[12.5px] font-medium text-ink-2">加载器</label>
                <select
                  value={loader}
                  onChange={(e) => setLoader(e.target.value as Loader)}
                  className="mt-1.5 w-full rounded-[10px] border border-divider bg-white px-3.5 py-2.5 text-[13.5px] text-ink outline-none transition-colors focus:border-accent"
                >
                  {(Object.keys(loaderLabels) as Loader[]).map((l) => (
                    <option key={l} value={l}>
                      {loaderLabels[l]}
                    </option>
                  ))}
                </select>
                {loader !== "vanilla" && loader !== "forge" && (
                  <p className="mt-1.5 text-[11.5px] text-ink-3">
                    将自动安装最新版 {loaderLabels[loader]} 并下载对应资源
                  </p>
                )}
              </div>

              {loader === "forge" && (
                <div>
                  <label className="text-[12.5px] font-medium text-ink-2">
                    Forge 版本
                    {!mcVersion && <span className="ml-1 text-ink-3">(先选择 MC 版本)</span>}
                  </label>
                  <select
                    value={forgeVersion}
                    onChange={(e) => setForgeVersion(e.target.value)}
                    disabled={!mcVersion || s.forgeVersionsLoading}
                    className="mt-1.5 w-full rounded-[10px] border border-divider bg-white px-3.5 py-2.5 text-[13.5px] text-ink outline-none transition-colors focus:border-accent disabled:opacity-50"
                  >
                    <option value="">
                      {s.forgeVersionsLoading
                        ? "加载中..."
                        : s.forgeVersions.length === 0
                          ? "该版本暂无 Forge"
                          : "选择版本"}
                    </option>
                    {s.forgeVersions.map((fv) => (
                      <option key={fv.version} value={fv.version}>
                        {fv.version}
                        {fv.is_recommended ? " (推荐)" : fv.is_latest ? " (最新)" : ""}
                      </option>
                    ))}
                  </select>
                  {mcVersion && s.forgeVersions.length > 0 && (
                    <p className="mt-1.5 text-[11.5px] text-ink-3">
                      安装过程将执行 Forge 处理器,耗时较长请耐心等待
                    </p>
                  )}
                </div>
              )}

              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="text-[12.5px] font-medium text-ink-2">最小内存 (MB)</label>
                  <input
                    type="number"
                    value={minMemory}
                    onChange={(e) => setMinMemory(Number(e.target.value) || 0)}
                    className="mt-1.5 w-full rounded-[10px] border border-divider bg-white px-3.5 py-2.5 text-[13.5px] text-ink outline-none transition-colors focus:border-accent"
                  />
                </div>
                <div>
                  <label className="text-[12.5px] font-medium text-ink-2">最大内存 (MB)</label>
                  <input
                    type="number"
                    value={maxMemory}
                    onChange={(e) => setMaxMemory(Number(e.target.value) || 0)}
                    className="mt-1.5 w-full rounded-[10px] border border-divider bg-white px-3.5 py-2.5 text-[13.5px] text-ink outline-none transition-colors focus:border-accent"
                  />
                </div>
              </div>

              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="text-[12.5px] font-medium text-ink-2">宽度</label>
                  <input
                    type="number"
                    value={width}
                    onChange={(e) => setWidth(Number(e.target.value) || 0)}
                    className="mt-1.5 w-full rounded-[10px] border border-divider bg-white px-3.5 py-2.5 text-[13.5px] text-ink outline-none transition-colors focus:border-accent"
                  />
                </div>
                <div>
                  <label className="text-[12.5px] font-medium text-ink-2">高度</label>
                  <input
                    type="number"
                    value={height}
                    onChange={(e) => setHeight(Number(e.target.value) || 0)}
                    className="mt-1.5 w-full rounded-[10px] border border-divider bg-white px-3.5 py-2.5 text-[13.5px] text-ink outline-none transition-colors focus:border-accent"
                  />
                </div>
              </div>

              {error && (
                <p className="rounded-[10px] bg-red-50 px-3.5 py-2.5 text-[12.5px] text-red-600">
                  {error}
                </p>
              )}
            </div>

            <div className="mt-6 flex gap-3">
              <button
                onClick={submit}
                disabled={saving}
                className="flex flex-1 items-center justify-center gap-2 rounded-[12px] bg-accent py-2.5 text-[13.5px] font-semibold text-white transition-colors hover:bg-accent-hover disabled:opacity-50"
              >
                {saving && <Loader2 size={14} className="animate-spin" />}
                {editing ? "保存修改" : "创建实例"}
              </button>
              <button
                onClick={s.closeModal}
                className="flex-1 rounded-[12px] border border-divider py-2.5 text-[13.5px] font-medium text-ink-2 transition-colors hover:bg-black/[0.03]"
              >
                取消
              </button>
            </div>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
