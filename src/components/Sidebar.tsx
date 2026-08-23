import { motion } from "framer-motion";

const navItems = [
  { key: "home", label: "主页" },
  { key: "versions", label: "版本" },
  { key: "instances", label: "实例" },
  { key: "mods", label: "Mod" },
  { key: "settings", label: "设置" },
];

export default function Sidebar() {
  return (
    <aside className="flex h-full w-56 shrink-0 flex-col border-r border-divider bg-frosted backdrop-blur-xl">
      <div className="px-5 pt-6 pb-4">
        <div className="flex items-center gap-2.5">
          <div className="flex h-8 w-8 items-center justify-center rounded-[10px] bg-accent text-sm font-bold text-white shadow-card">
            R
          </div>
          <span className="text-[17px] font-semibold tracking-tight">Runa</span>
        </div>
      </div>

      <nav className="flex flex-1 flex-col gap-1 px-3">
        {navItems.map((item, i) => (
          <motion.button
            key={item.key}
            initial={{ opacity: 0, x: -8 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ delay: 0.05 * i, duration: 0.3, ease: [0.32, 0.72, 0, 1] }}
            className={`flex items-center rounded-[9px] px-3.5 py-2 text-left text-[13.5px] transition-colors duration-150 ${
              i === 0
                ? "bg-card font-medium text-ink shadow-card"
                : "text-ink-2 hover:bg-card/60 hover:text-ink"
            }`}
          >
            {item.label}
          </motion.button>
        ))}
      </nav>

      <div className="border-t border-divider px-5 py-4 text-[11px] text-ink-3">
        Runa v0.1.0
      </div>
    </aside>
  );
}
