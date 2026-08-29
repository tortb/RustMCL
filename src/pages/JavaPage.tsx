import { useEffect } from "react";
import { motion } from "framer-motion";
import { Coffee, Loader2, CheckCircle2, XCircle, AlertTriangle } from "lucide-react";
import { useSettingsStore } from "../stores/settings";

const ease = [0.32, 0.72, 0, 1] as const;

export default function JavaPage() {
  const s = useSettingsStore();

  useEffect(() => {
    s.load();
    s.detect();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const autoDetect = s.config?.java.auto_detect ?? true;
  const configuredPath = s.config?.java.default_java_path ?? "";

  return (
    <div className="flex-1 overflow-y-auto bg-[#f3f4f6] px-6 py-8">
      <motion.div
        initial={{ opacity: 0, y: 10 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.35, ease }}
        className="mx-auto max-w-2xl"
      >
        <div>
          <h1 className="text-[24px] font-bold tracking-tight text-ink">Java</h1>
          <p className="mt-1 text-[13px] text-ink-3">检测运行游戏所需的 Java 环境</p>
        </div>

        <motion.div
          initial={{ opacity: 0, y: 8 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.05, duration: 0.3, ease }}
          className="mt-6 rounded-[16px] bg-white p-6 shadow-card"
        >
          <div className="flex items-center gap-3">
            <div className="flex h-11 w-11 items-center justify-center rounded-[12px] bg-[#e8f5e9]">
              <Coffee size={20} className="text-accent" strokeWidth={1.8} />
            </div>
            <h2 className="text-[15px] font-semibold text-ink">系统 Java 检测</h2>
          </div>

          <div className="mt-5">
            {s.detectingJava ? (
              <div className="flex items-center gap-2 text-[13.5px] text-ink-3">
                <Loader2 size={14} className="animate-spin" />
                正在检测 java -version...
              </div>
            ) : s.javaVersion ? (
              <div className="flex items-center gap-2 rounded-[10px] bg-[#f1f8e9] px-3.5 py-3 text-[13.5px] text-ink">
                <CheckCircle2 size={16} className="text-accent" />
                检测到 Java <span className="font-semibold">{s.javaVersion}</span>
              </div>
            ) : (
              <div className="flex items-start gap-2 rounded-[10px] bg-red-50 px-3.5 py-3 text-[13px] text-red-600">
                <AlertTriangle size={16} className="mt-0.5 shrink-0" />
                未检测到 Java。请安装 JDK 17 / 21,或在设置页手动指定 Java 路径。
              </div>
            )}
          </div>

          <div className="mt-4 flex items-center gap-2">
            <motion.button
              whileTap={{ scale: 0.97 }}
              onClick={() => s.detect()}
              disabled={s.detectingJava}
              className="rounded-[10px] border border-divider px-3.5 py-2 text-[12.5px] font-medium text-ink-2 transition-colors hover:bg-black/[0.03] disabled:opacity-50"
            >
              重新检测
            </motion.button>
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 8 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.1, duration: 0.3, ease }}
          className="mt-4 rounded-[16px] bg-white p-6 shadow-card"
        >
          <h2 className="text-[15px] font-semibold text-ink">运行配置</h2>
          <div className="mt-4 flex flex-col gap-3 text-[13.5px]">
            <div className="flex items-center justify-between rounded-[10px] border border-divider px-3.5 py-2.5">
              <span className="text-ink-2">自动检测</span>
              <span
                className={`flex items-center gap-1.5 font-medium ${
                  autoDetect ? "text-accent" : "text-ink-3"
                }`}
              >
                {autoDetect ? (
                  <>
                    <CheckCircle2 size={14} /> 开启
                  </>
                ) : (
                  <>
                    <XCircle size={14} /> 关闭
                  </>
                )}
              </span>
            </div>
            {configuredPath && (
              <div className="flex items-center justify-between rounded-[10px] border border-divider px-3.5 py-2.5">
                <span className="text-ink-2">自定义路径</span>
                <span className="break-all font-mono text-[12px] text-ink">{configuredPath}</span>
              </div>
            )}
            <p className="text-[12px] text-ink-3">
              提示:自动检测关闭后,游戏将使用设置页指定的 Java 路径。
            </p>
          </div>
        </motion.div>
      </motion.div>
    </div>
  );
}
