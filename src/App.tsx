import { useEffect, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { RefreshCw, X } from "lucide-react";
import Sidebar from "./components/Sidebar";
import LoginModal from "./components/LoginModal";
import Home from "./pages/Home";
import Downloads from "./pages/Downloads";
import Instances from "./pages/Instances";
import Mods from "./pages/Mods";
import JavaPage from "./pages/JavaPage";
import Settings from "./pages/Settings";
import Servers from "./pages/Servers";
import Packs from "./pages/Packs";
import Placeholder from "./components/Placeholder";
import { useAppStore } from "./stores/app";
import { useAccountStore } from "./stores/account";
import { checkForUpdate } from "./lib/api";
import type { UpdateInfo } from "./lib/types";

export type PageKey =
  | "home"
  | "instances"
  | "downloads"
  | "mods"
  | "packs"
  | "servers"
  | "java"
  | "settings";

const pages: Record<string, string> = {
  instances: "实例管理",
  downloads: "下载管理",
  java: "Java 管理",
};

export default function App() {
  const init = useAppStore((s) => s.init);
  const loadAccounts = useAccountStore((s) => s.loadAccounts);
  const [page, setPage] = useState<PageKey>("home");
  const [silentUpdate, setSilentUpdate] = useState<UpdateInfo | null>(null);

  useEffect(() => {
    init();
    loadAccounts();
    // 启动静默检查更新;未配置更新源或网络失败时静默忽略
    checkForUpdate()
      .then((info) => {
        if (info.has_update) setSilentUpdate(info);
      })
      .catch(() => undefined);
  }, [init, loadAccounts]);

  return (
    <div className="flex h-full">
      <Sidebar active={page} onSelect={setPage} />
      <main className="flex flex-1 flex-col overflow-hidden">
        <AnimatePresence>
          {silentUpdate && (
            <motion.div
              initial={{ opacity: 0, y: -12 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -12 }}
              transition={{ duration: 0.25, ease: [0.32, 0.72, 0, 1] }}
              className="flex items-center gap-3 border-b border-accent/30 bg-accent/[0.08] px-5 py-2.5 text-[12.5px]"
            >
              <RefreshCw size={14} className="shrink-0 text-accent" />
              <span className="text-ink">
                发现新版本 v{silentUpdate.latest}(当前 v{silentUpdate.current})
              </span>
              <button
                onClick={() => {
                  setPage("settings");
                  setSilentUpdate(null);
                }}
                className="ml-auto shrink-0 rounded-[8px] bg-accent px-3 py-1.5 text-[12px] font-semibold text-white transition-colors hover:bg-accent-hover"
              >
                去设置查看
              </button>
              <button
                onClick={() => setSilentUpdate(null)}
                className="shrink-0 rounded-full p-1.5 text-ink-3 transition-colors hover:bg-black/[0.05]"
                aria-label="关闭"
              >
                <X size={14} />
              </button>
            </motion.div>
          )}
        </AnimatePresence>
        <AnimatePresence mode="wait">
          <motion.div
            key={page}
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -8 }}
            transition={{ duration: 0.25, ease: [0.32, 0.72, 0, 1] }}
            className="flex h-full flex-1"
          >
            {page === "home" && <Home onNavigate={setPage} />}
            {page === "instances" && <Instances />}
            {page === "downloads" && <Downloads />}
            {page === "mods" && <Mods />}
            {page === "packs" && <Packs />}
            {page === "java" && <JavaPage />}
            {page === "servers" && <Servers />}
            {page === "settings" && <Settings />}
            {page in pages && <Placeholder title={pages[page as keyof typeof pages]} />}
          </motion.div>
        </AnimatePresence>
      </main>
      {/* 登录弹窗常驻,保证事件监听不因弹窗显隐而丢失 */}
      <LoginModal />
    </div>
  );
}
