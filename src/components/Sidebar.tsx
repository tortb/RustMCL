import { motion } from "framer-motion";
import {
  Home,
  Box,
  Download,
  Puzzle,
  Coffee,
  Settings,
  ChevronDown,
  LogIn,
} from "lucide-react";
import type { PageKey } from "../App";
import { useAccountStore } from "../stores/account";

const navItems: { key: PageKey; label: string; icon: React.ElementType }[] = [
  { key: "home", label: "主页", icon: Home },
  { key: "instances", label: "实例", icon: Box },
  { key: "downloads", label: "下载", icon: Download },
  { key: "mods", label: "Mod", icon: Puzzle },
  { key: "java", label: "Java", icon: Coffee },
  { key: "settings", label: "设置", icon: Settings },
];

export default function Sidebar({
  active,
  onSelect,
}: {
  active: PageKey;
  onSelect: (key: PageKey) => void;
}) {
  const activeAccount = useAccountStore((s) => s.active);
  const openLogin = useAccountStore((s) => s.openLogin);
  const logout = useAccountStore((s) => s.logout);

  return (
    <aside className="flex h-full w-[210px] shrink-0 flex-col bg-[#f6f6f7]">
      {/* Logo */}
      <div className="px-5 pt-6 pb-5">
        <div className="flex items-center gap-2.5">
          <div className="flex h-8 w-8 items-center justify-center">
            <svg viewBox="0 0 32 32" className="h-8 w-8">
              <rect x="2" y="2" width="28" height="28" rx="4" fill="#7cb342" />
              <rect x="6" y="6" width="8" height="8" rx="1" fill="#558b2f" opacity="0.6" />
              <rect x="18" y="10" width="8" height="8" rx="1" fill="#8bc34a" opacity="0.7" />
              <rect x="10" y="18" width="10" height="8" rx="1" fill="#4e342e" opacity="0.5" />
            </svg>
          </div>
          <span className="text-[18px] font-bold tracking-tight text-ink">Runa</span>
        </div>
      </div>

      {/* Nav */}
      <nav className="flex flex-1 flex-col gap-0.5 px-3">
        {navItems.map((item, i) => {
          const Icon = item.icon;
          const isActive = active === item.key;
          return (
            <motion.button
              key={item.key}
              initial={{ opacity: 0, x: -8 }}
              animate={{ opacity: 1, x: 0 }}
              transition={{ delay: 0.04 * i, duration: 0.25, ease: [0.32, 0.72, 0, 1] }}
              onClick={() => onSelect(item.key)}
              className={`flex items-center gap-3 rounded-[10px] px-3.5 py-2.5 text-left text-[14px] transition-colors duration-150 ${
                isActive
                  ? "bg-[#e8f5e9] font-medium text-ink"
                  : "text-ink-2 hover:bg-black/[0.04] hover:text-ink"
              }`}
            >
              <Icon size={18} strokeWidth={1.8} />
              {item.label}
            </motion.button>
          );
        })}
      </nav>

      {/* User profile */}
      <div className="border-t border-divider px-4 py-3.5">
        {activeAccount ? (
          <div className="flex items-center gap-3 rounded-[10px] px-2 py-2">
            <div className="flex h-9 w-9 shrink-0 items-center justify-center overflow-hidden rounded-full bg-[#e0e0e0]">
              <svg viewBox="0 0 36 36" className="h-9 w-9">
                <rect width="36" height="36" fill="#8d6e63" />
                <rect x="8" y="6" width="20" height="14" rx="2" fill="#f5deb3" />
                <rect x="11" y="10" width="4" height="3" rx="0.5" fill="#5d4037" />
                <rect x="21" y="10" width="4" height="3" rx="0.5" fill="#5d4037" />
                <rect x="14" y="16" width="8" height="2" rx="0.5" fill="#5d4037" />
                <rect x="6" y="22" width="24" height="10" rx="2" fill="#4caf50" />
              </svg>
            </div>
            <div className="flex flex-1 flex-col items-start">
              <span className="text-[13.5px] font-medium text-ink">{activeAccount.username}</span>
              <span className="flex items-center gap-1.5 text-[11.5px] text-ink-3">
                <span className="inline-block h-1.5 w-1.5 rounded-full bg-[#4caf50]" />
                {activeAccount.account_type === "microsoft" ? "微软账号" : "离线账号"}
              </span>
            </div>
            <button
              onClick={() => logout(activeAccount.id)}
              className="rounded-full p-1.5 text-ink-3 transition-colors hover:bg-black/[0.05]"
              aria-label="退出登录"
            >
              <ChevronDown size={14} />
            </button>
          </div>
        ) : (
          <button
            onClick={openLogin}
            className="flex w-full items-center gap-3 rounded-[10px] px-2 py-2 transition-colors hover:bg-black/[0.04]"
          >
            <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full border border-dashed border-ink-3 text-ink-3">
              <LogIn size={15} />
            </div>
            <div className="flex flex-1 flex-col items-start">
              <span className="text-[13.5px] font-medium text-ink-2">未登录</span>
              <span className="text-[11.5px] text-ink-3">点击登录微软账号</span>
            </div>
            <ChevronDown size={14} className="text-ink-3" />
          </button>
        )}
      </div>
    </aside>
  );
}
