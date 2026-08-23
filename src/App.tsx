import { useEffect, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import Sidebar from "./components/Sidebar";
import LoginModal from "./components/LoginModal";
import Home from "./pages/Home";
import Downloads from "./pages/Downloads";
import JavaPage from "./pages/JavaPage";
import Placeholder from "./components/Placeholder";
import { useAppStore } from "./stores/app";
import { useAccountStore } from "./stores/account";

export type PageKey = "home" | "instances" | "downloads" | "mods" | "java" | "settings";

const pages: Record<Exclude<PageKey, "home">, string> = {
  instances: "实例管理",
  downloads: "下载管理",
  mods: "Mod 管理",
  java: "Java 管理",
  settings: "设置",
};

export default function App() {
  const init = useAppStore((s) => s.init);
  const loadAccounts = useAccountStore((s) => s.loadAccounts);
  const [page, setPage] = useState<PageKey>("home");

  useEffect(() => {
    init();
    loadAccounts();
  }, [init, loadAccounts]);

  return (
    <div className="flex h-full">
      <Sidebar active={page} onSelect={setPage} />
      <main className="flex flex-1 flex-col overflow-hidden">
        <AnimatePresence mode="wait">
          <motion.div
            key={page}
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -8 }}
            transition={{ duration: 0.25, ease: [0.32, 0.72, 0, 1] }}
            className="flex h-full flex-1"
          >
            {page === "home" && <Home />}
            {page === "downloads" && <Downloads />}
            {page === "java" && <JavaPage />}
            {page in pages && <Placeholder title={pages[page as keyof typeof pages]} />}
          </motion.div>
        </AnimatePresence>
      </main>
      {/* 登录弹窗常驻,保证事件监听不因弹窗显隐而丢失 */}
      <LoginModal />
    </div>
  );
}
