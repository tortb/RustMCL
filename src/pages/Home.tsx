import { motion } from "framer-motion";
import { Play, HardDrive, Coffee, MoreHorizontal } from "lucide-react";

interface InstanceCard {
  id: string;
  name: string;
  loader: string;
  mods: number;
  size: string;
  java: string;
}

const mockInstances: InstanceCard[] = [
  {
    id: "1",
    name: "1.21.8 Fabric",
    loader: "Fabric",
    mods: 12,
    size: "2.3 GB",
    java: "Java 21",
  },
  {
    id: "2",
    name: "1.21.8 Vanilla",
    loader: "Vanilla",
    mods: 0,
    size: "1.1 GB",
    java: "Java 21",
  },
];

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

export default function Home() {
  const latestVersion = "1.21.8";
  const latestLoader = "Fabric";

  return (
    <div className="flex-1 overflow-y-auto bg-[#f3f4f6]">
      {/* Hero Banner */}
      <motion.div
        initial={{ opacity: 0, y: 12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.4, ease: [0.32, 0.72, 0, 1] }}
        className="relative mx-6 mt-6 overflow-hidden rounded-[16px] bg-white shadow-card"
        style={{ height: 220 }}
      >
        {/* Background landscape */}
        <div className="absolute inset-0">
          <svg viewBox="0 0 800 220" className="h-full w-full" preserveAspectRatio="xMidYMid slice">
            {/* Sky */}
            <rect width="800" height="220" fill="#b3d9f2" />
            {/* Clouds */}
            <rect x="100" y="30" width="80" height="20" rx="10" fill="white" opacity="0.8" />
            <rect x="130" y="20" width="50" height="20" rx="10" fill="white" opacity="0.8" />
            <rect x="500" y="40" width="60" height="16" rx="8" fill="white" opacity="0.7" />
            <rect x="650" y="25" width="70" height="18" rx="9" fill="white" opacity="0.75" />
            {/* Hills */}
            <rect x="0" y="120" width="800" height="100" fill="#7cb342" />
            <rect x="0" y="140" width="800" height="80" fill="#689f38" />
            <rect x="0" y="160" width="800" height="60" fill="#558b2f" />
            {/* Tree */}
            <rect x="580" y="60" width="40" height="80" rx="4" fill="#4e342e" />
            <rect x="550" y="20" width="100" height="60" rx="8" fill="#388e3c" />
            <rect x="560" y="10" width="80" height="40" rx="6" fill="#43a047" />
            {/* Ground blocks */}
            <rect x="0" y="180" width="800" height="40" fill="#5d4037" />
          </svg>
        </div>

        {/* Gradient overlay */}
        <div
          className="absolute inset-0"
          style={{
            background:
              "linear-gradient(90deg, rgba(243,244,246,0.97) 0%, rgba(243,244,246,0.85) 35%, rgba(243,244,246,0.4) 60%, transparent 80%)",
          }}
        />

        {/* Content */}
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
        transition={{ delay: 0.1, duration: 0.4, ease: [0.32, 0.72, 0, 1] }}
        className="px-6 pt-8 pb-10"
      >
        <h2 className="mb-4 text-[16px] font-bold text-ink">最近实例</h2>

        <div className="flex flex-col gap-3">
          {mockInstances.map((inst, i) => (
            <motion.div
              key={inst.id}
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{
                delay: 0.15 + i * 0.06,
                duration: 0.3,
                ease: [0.32, 0.72, 0, 1],
              }}
              className="flex items-center rounded-[14px] bg-white px-5 py-4 shadow-card transition-shadow duration-200 hover:shadow-card-hover"
            >
              <GrassBlock size={44} />

              <div className="ml-4 flex flex-1 flex-col">
                <span className="text-[15px] font-semibold text-ink">
                  {inst.name}
                </span>
                <span
                  className={`mt-1 inline-flex w-fit items-center rounded-[6px] px-2 py-0.5 text-[11.5px] font-medium ${
                    inst.mods > 0
                      ? "bg-[#f1f8e9] text-[#558b2f]"
                      : "bg-gray-100 text-ink-3"
                  }`}
                >
                  {inst.mods} 个 Mod
                </span>
              </div>

              <div className="flex items-center gap-6 text-[13px] text-ink-2">
                <span className="flex items-center gap-1.5">
                  <HardDrive size={15} strokeWidth={1.8} />
                  {inst.size}
                </span>
                <span className="flex items-center gap-1.5">
                  <Coffee size={15} strokeWidth={1.8} />
                  {inst.java}
                </span>
              </div>

              <div className="ml-6 flex items-center gap-2">
                <button className="flex h-9 w-9 items-center justify-center rounded-full border border-divider text-ink-2 transition-colors hover:bg-gray-50 hover:text-ink">
                  <Play size={14} fill="currentColor" strokeWidth={0} />
                </button>
                <button className="flex h-9 w-9 items-center justify-center rounded-full text-ink-3 transition-colors hover:bg-gray-50 hover:text-ink-2">
                  <MoreHorizontal size={16} />
                </button>
              </div>
            </motion.div>
          ))}
        </div>

        <p className="mt-4 text-center text-[12.5px] text-ink-3">
          共 {mockInstances.length} 个实例
        </p>
      </motion.div>
    </div>
  );
}
