import { useState } from "react";
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
  Server,
  Image,
  RefreshCw,
  LogOut,
} from "lucide-react";
import type { PageKey } from "../App";
import { useAccountStore } from "../stores/account";
import logoUrl from "../assets/logo.png";

const navItems: { key: PageKey; label: string; icon: React.ElementType }[] = [
  { key: "home", label: "主页", icon: Home },
  { key: "instances", label: "实例", icon: Box },
  { key: "downloads", label: "下载", icon: Download },
  { key: "mods", label: "Mod", icon: Puzzle },
  { key: "packs", label: "资源包", icon: Image },
  { key: "servers", label: "服务器", icon: Server },
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
  const [menuOpen, setMenuOpen] = useState(false);

  // 与"点击账号"解耦:单纯的查看/切换,不再触发登出
  const handleSwitchAccount = () => {
    setMenuOpen(false);
    openLogin();
  };
  // 登出必须是用户主动、明确的操作 + 二次确认
  const handleLogout = () => {
    if (!activeAccount) return;
    if (
      window.confirm(`确定要退出登录「${activeAccount.username}」吗?下次启动将需要用该账号重新登录。`)
    ) {
      logout(activeAccount.id);
    }
    setMenuOpen(false);
  };

  return (
    <aside className="flex h-full w-[210px] shrink-0 flex-col bg-[#f6f6f7]">
      {/* Logo */}
      <div className="px-5 pt-6 pb-5">
        <div className="flex items-center gap-2.5">
          <div className="flex h-8 w-8 items-center justify-center">
            <img src={logoUrl} alt="RustMCL" className="h-8 w-8 rounded-[6px]" />
          </div>
          <span className="text-[18px] font-bold tracking-tight text-ink">RustMCL</span>
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
          <div className="relative">
            <button
              onClick={() => setMenuOpen((v) => !v)}
              className="flex w-full items-center gap-3 rounded-[10px] px-2 py-2 text-left transition-colors hover:bg-black/[0.04]"
            >
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
              <ChevronDown
                size={14}
                className={`shrink-0 text-ink-3 transition-transform duration-200 ${
                  menuOpen ? "rotate-180" : ""
                }`}
              />
            </button>

            {/* 账号菜单:查看/切换与登出完全独立,登出需二次确认 */}
            {menuOpen && (
              <div className="absolute bottom-full left-0 right-0 mb-2 overflow-hidden rounded-[12px] border border-[#e5e7eb] bg-white shadow-[0_12px_32px_rgba(0,0,0,0.12)]">
                <div className="border-b border-divider px-4 py-3">
                  <p className="text-[12px] font-semibold text-ink">{activeAccount.username}</p>
                  <p className="mt-0.5 text-[11px] text-ink-3">
                    {activeAccount.account_type === "microsoft" ? "微软账号" : "离线账号"} ·{" "}
                    {activeAccount.uuid.slice(0, 8)}
                  </p>
                </div>
                <button
                  onClick={handleSwitchAccount}
                  className="flex w-full items-center gap-2.5 px-4 py-2.5 text-left text-[13px] text-ink transition-colors hover:bg-black/[0.04]"
                >
                  <RefreshCw size={15} strokeWidth={1.8} />
                  切换账号
                </button>
                <button
                  onClick={handleLogout}
                  className="flex w-full items-center gap-2.5 border-t border-divider px-4 py-2.5 text-left text-[13px] text-red-600 transition-colors hover:bg-red-50"
                >
                  <LogOut size={15} strokeWidth={1.8} />
                  退出登录
                </button>
              </div>
            )}
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
