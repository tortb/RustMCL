import { useEffect } from "react";
import { motion } from "framer-motion";
import { Play, HardDrive, Coffee, MoreHorizontal, Boxes, AlertTriangle, Pencil } from "lucide-react";
import { useInstanceStore } from "../stores/instance";
import { useAccountStore } from "../stores/account";
import type { PageKey } from "../App";
import type { InstanceDetail, Loader } from "../lib/types";
import backgroundUrl from "../assets/background.png";

const ease = [0.32, 0.72, 0, 1] as const;

const loaderLabels: Record<Loader, string> = {
  vanilla: "原版",
  forge: "Forge",
  fabric: "Fabric",
  quilt: "Quilt",
};

const fmtMem = (mb: number) => (mb >= 1024 ? `${(mb / 1024).toFixed(1)} GB` : `${mb} MB`);

function GrassBlock({ size = 40 }: { size?: number }) {
  return (
    <svg viewBox="0 0 40 40" width={size} height={size} className="shrink-0">
      <rect width="40" height="40" rx="6" fill="#7cb342" />
      <rect x="4" y="4" width="14" height="10" rx="2" fill="#558b2f" opacity="0.5" />
      <rect x="22" y="8" width="12" height="10" rx="2" fill="#8bc34a" opacity="0.6" />
      <rect x="6" y="20" width="28" height="14" rx="2" fill="#6d4c41" opacity="0.7" />
      <rect x="10" y="24" width="6" height="4" rx="1" fill="#5d4037" opacity="0.5" />
      <rect x="24" y="26" width="8" height="4" rx="1" fill="#4e342e" opacity="0.4" />
    </svg>
  );
}

function SkeletonCard() {
  return (
    <div className="flex animate-pulse items-center rounded-[14px] bg-white px-5 py-4 shadow-card">
      <div className="h-11 w-11 rounded-[6px] bg-gray-200" />
      <div className="ml-4 flex-1">
        <div className="h-4 w-40 rounded bg-gray-200" />
        <div className="mt-2 h-3 w-20 rounded bg-gray-200" />
      </div>
      <div className="flex items-center gap-6">
        <div className="h-4 w-16 rounded bg-gray-200" />
        <div className="h-4 w-24 rounded bg-gray-200" />
      </div>
    </div>
  );
}

export default function Home({ onNavigate }: { onNavigate: (p: PageKey) => void }) {
  const s = useInstanceStore();

  useEffect(() => {
    s.loadInstances();
    s.loadVersions();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const latestVersion = s.versions[0]?.id ?? "1.21";
  const latestLoader = "原版";

  // 英雄区"启动":有实例则启动最近的一个(走登录门禁),否则跳转到实例页
  const handleHeroLaunch = () => {
    const first = s.instances[0];
    if (!first) {
      onNavigate("instances");
      return;
    }
    // 未登录:先引导登录,不发起实际启动
    if (!useAccountStore.getState().active) {
      useAccountStore.getState().openLogin();
      return;
    }
    s.launch(first.id);
    // 跳转到实例页,让用户看到资源下载进度与运行日志(Home 不渲染这些反馈)
    onNavigate("instances");
  };

  return (
    <div className="flex-1 overflow-y-auto bg-[#f3f4f6]">
      {/* Hero Banner */}
      <motion.div
        initial={{ opacity: 0, y: 12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.4, ease }}
        className="relative mx-6 mt-6 overflow-hidden rounded-[16px] bg-white shadow-card"
        style={{ height: 220 }}
      >
        <div className="absolute inset-0">
          <img src={backgroundUrl} alt="" className="h-full w-full object-cover" />
        </div>

        <div
          className="absolute inset-0"
          style={{
            background:
              "linear-gradient(90deg, rgba(243,244,246,0.97) 0%, rgba(243,244,246,0.85) 35%, rgba(243,244,246,0.4) 60%, transparent 80%)",
          }}
        />

        <div className="relative z-10 flex h-full flex-col justify-center px-10">
          <h1 className="text-[32px] font-bold tracking-tight text-ink">
            Minecraft {latestVersion}
          </h1>
          <div className="mt-2 flex items-center gap-2 text-[14px] text-ink-2">
            <svg viewBox="0 0 16 16" className="h-4 w-4" fill="currentColor">
              <rect x="2" y="2" width="12" height="12" rx="2" opacity="0.6" />
            </svg>
            {latestLoader}
          </div>
          <button
            onClick={handleHeroLaunch}
            className="mt-6 flex items-center gap-2 rounded-[12px] bg-[#7cb342] px-7 py-3 text-[15px] font-semibold text-white shadow-lg transition-all duration-150 hover:bg-[#689f38] active:scale-[0.97]"
            style={{ boxShadow: "0 4px 14px rgba(124,179,66,0.35)" }}
          >
            <Play size={18} fill="white" strokeWidth={0} />
            启动
          </button>
        </div>
      </motion.div>

      {/* Recent Instances */}
      <motion.div
        initial={{ opacity: 0, y: 12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.1, duration: 0.4, ease }}
        className="px-6 pt-8 pb-10"
      >
        <h2 className="mb-4 text-[16px] font-bold text-ink">最近实例</h2>

        {/* 加载态 */}
        {s.loading && (
          <div className="flex flex-col gap-3">
            <SkeletonCard />
            <SkeletonCard />
            <SkeletonCard />
          </div>
        )}

        {/* 错误态 */}
        {!s.loading && s.error && (
          <div className="flex flex-col items-center gap-3 rounded-[14px] border border-red-100 bg-red-50/60 px-6 py-8 text-center">
            <AlertTriangle size={22} className="text-red-500" />
            <p className="text-[13.5px] leading-relaxed text-red-600">{s.error}</p>
            <button
              onClick={() => s.loadInstances()}
              className="mt-1 rounded-[10px] bg-accent px-4 py-2 text-[13px] font-semibold text-white transition-colors hover:bg-accent-hover"
            >
              重试
            </button>
          </div>
        )}

        {/* 空态 */}
        {!s.loading && !s.error && s.instances.length === 0 && (
          <div className="flex flex-col items-center gap-3 rounded-[14px] border border-dashed border-divider px-6 py-10 text-center">
            <Boxes size={26} className="text-ink-3" />
            <p className="text-[13.5px] leading-relaxed text-ink-2">
              还没有任何实例,去「实例」页创建你的第一个版本吧。
            </p>
          </div>
        )}

        {/* 数据态 */}
        {!s.loading && !s.error && s.instances.length > 0 && (
          <div className="flex flex-col gap-3">
            {s.instances.map((inst, i) => (
              <InstanceRow key={inst.id} inst={inst} index={i} onNavigate={onNavigate} />
            ))}
          </div>
        )}

        {!s.loading && !s.error && s.instances.length > 0 && (
          <p className="mt-4 text-center text-[12.5px] text-ink-3">共 {s.instances.length} 个实例</p>
        )}
      </motion.div>
    </div>
  );
}

function InstanceRow({
  inst,
  index,
  onNavigate,
}: {
  inst: InstanceDetail;
  index: number;
  onNavigate: (p: PageKey) => void;
}) {
  const loader = inst.loader && inst.loader !== "vanilla" ? inst.loader : null;
  const memory = inst.config.jvm.max_memory;
  return (
    <motion.div
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ delay: 0.15 + index * 0.06, duration: 0.3, ease }}
      className="flex items-center rounded-[14px] bg-white px-5 py-4 shadow-card transition-shadow duration-200 hover:shadow-card-hover"
    >
      <GrassBlock size={44} />

      <div className="ml-4 flex flex-1 flex-col">
        <span className="text-[15px] font-semibold text-ink">{inst.name}</span>
        <span
          className={`mt-1 inline-flex w-fit items-center rounded-[6px] px-2 py-0.5 text-[11.5px] font-medium ${
            loader ? "bg-[#f1f8e9] text-[#558b2f]" : "bg-gray-100 text-ink-3"
          }`}
        >
          {inst.mc_version}
          {loader ? ` · ${loaderLabels[loader as Loader] ?? loader}` : " 原版"}
        </span>
      </div>

      <div className="flex items-center gap-6 text-[13px] text-ink-2">
        <span className="flex items-center gap-1.5">
          <HardDrive size={15} strokeWidth={1.8} />
          {fmtMem(memory)}
        </span>
        <span className="flex items-center gap-1.5">
          <Coffee size={15} strokeWidth={1.8} />
          Java 21
        </span>
      </div>

      <div className="ml-6 flex items-center gap-2">
        <button
          onClick={() => onNavigate("instances")}
          className="flex h-9 w-9 items-center justify-center rounded-full border border-divider text-ink-2 transition-colors hover:bg-gray-50 hover:text-ink"
          aria-label="编辑实例"
        >
          <Pencil size={14} />
        </button>
        <button
          onClick={() => onNavigate("instances")}
          className="flex h-9 w-9 items-center justify-center rounded-full text-ink-3 transition-colors hover:bg-gray-50 hover:text-ink-2"
          aria-label="更多"
        >
          <MoreHorizontal size={16} />
        </button>
      </div>
    </motion.div>
  );
}
