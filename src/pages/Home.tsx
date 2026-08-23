import { motion } from "framer-motion";
import { useAppStore } from "../stores/app";

export default function Home() {
  const { appInfo, dbTables, loading, error } = useAppStore();

  return (
    <div className="mx-auto max-w-3xl px-10 py-12">
      <motion.div
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.35, ease: [0.32, 0.72, 0, 1] }}
      >
        <h1 className="text-[28px] font-bold tracking-tight">欢迎使用 Runa</h1>
        <p className="mt-1.5 text-[14px] text-ink-2">
          现代化 Minecraft 启动器 —— 启动速度快,内存占用低。
        </p>
      </motion.div>

      <motion.div
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.08, duration: 0.35, ease: [0.32, 0.72, 0, 1] }}
        className="mt-8 grid grid-cols-2 gap-4"
      >
        <StatusCard title="应用" delay={0.12}>
          {loading ? (
            <p className="text-[13px] text-ink-3">加载中…</p>
          ) : error ? (
            <p className="text-[13px] text-red-500">{error}</p>
          ) : (
            <>
              <InfoRow label="版本" value={appInfo?.version ?? "-"} />
              <InfoRow label="数据目录" value={appInfo?.data_dir ?? "-"} mono />
            </>
          )}
        </StatusCard>

        <StatusCard title="数据库" delay={0.16}>
          {loading ? (
            <p className="text-[13px] text-ink-3">加载中…</p>
          ) : error ? (
            <p className="text-[13px] text-red-500">{error}</p>
          ) : (
            <ul className="flex flex-wrap gap-1.5 pt-0.5">
              {dbTables.map((t) => (
                <li
                  key={t}
                  className="rounded-[8px] bg-bg px-2.5 py-1 font-mono text-[11.5px] text-ink-2"
                >
                  {t}
                </li>
              ))}
            </ul>
          )}
        </StatusCard>
      </motion.div>
    </div>
  );
}

function StatusCard({
  title,
  delay,
  children,
}: {
  title: string;
  delay: number;
  children: React.ReactNode;
}) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ delay, duration: 0.35, ease: [0.32, 0.72, 0, 1] }}
      className="rounded-card bg-card p-5 shadow-card"
    >
      <h2 className="text-[15px] font-semibold">{title}</h2>
      <div className="mt-3">{children}</div>
    </motion.div>
  );
}

function InfoRow({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="flex items-baseline justify-between gap-3 border-t border-divider py-2 first:border-t-0 first:pt-0">
      <span className="text-[12.5px] text-ink-3">{label}</span>
      <span
        className={`truncate text-[12.5px] text-ink ${
          mono ? "font-mono text-[11.5px]" : ""
        }`}
      >
        {value}
      </span>
    </div>
  );
}
