import { motion } from "framer-motion";

export default function JavaPage() {
  return (
    <div className="flex-1 overflow-y-auto bg-[#f3f4f6] px-6 py-8">
      <motion.div
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.35, ease: [0.32, 0.72, 0, 1] }}
        className="rounded-[14px] bg-white p-10 text-center shadow-card"
      >
        <h1 className="text-[22px] font-bold text-ink">Java 管理</h1>
        <p className="mt-2 text-[13.5px] text-ink-3">该模块将在后续阶段实现</p>
      </motion.div>
    </div>
  );
}
