import { useEffect, useState } from "react";
import { motion } from "framer-motion";
import { Save, Loader2, Coffee, Download, FolderOpen } from "lucide-react";
import { useSettingsStore } from "../stores/settings";

const ease = [0.32, 0.72, 0, 1] as const;

const themeLabels: Record<string, string> = {
  dark: "深色",
  light: "浅色",
};

export default function Settings() {
  const s = useSettingsStore();
  const [maxConcurrent, setMaxConcurrent] = useState(8);
  const [retryTimes, setRetryTimes] = useState(3);
  const [autoDetect, setAutoDetect] = useState(true);
  const [javaPath, setJavaPath] = useState("");
  const [theme, setTheme] = useState("dark");
  const [language, setLanguage] = useState("zh-CN");

  useEffect(() => {
    s.load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (s.config) {
      setMaxConcurrent(s.config.download.max_concurrent);
      setRetryTimes(s.config.download.retry_times);
      setAutoDetect(s.config.java.auto_detect);
      setJavaPath(s.config.java.default_java_path);
      setTheme(s.config.general.theme);
      setLanguage(s.config.general.language);
    }
  }, [s.config]);

  const handleSave = async () => {
    if (!s.config) return;
    await s.save({
      ...s.config,
      general: { ...s.config.general, theme, language },
      java: { auto_detect: autoDetect, default_java_path: javaPath },
      download: { ...s.config.download, max_concurrent: maxConcurrent, retry_times: retryTimes },
    });
  };

  return (
    <div className="flex-1 overflow-y-auto bg-[#f3f4f6] px-6 py-8">
      <motion.div
        initial={{ opacity: 0, y: 10 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.35, ease }}
        className="mx-auto max-w-2xl"
      >
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-[24px] font-bold tracking-tight text-ink">设置</h1>
            <p className="mt-1 text-[13px] text-ink-3">应用级配置,保存至 config.toml</p>
          </div>
        </div>

        {s.config ? (
          <div className="mt-6 flex flex-col gap-4">
            {/* 下载设置 */}
            <Section icon={<Download size={16} />} title="下载">
              <div className="grid grid-cols-2 gap-4">
                <Field label="最大并发数">
                  <input
                    type="number"
                    min={1}
                    value={maxConcurrent}
                    onChange={(e) => setMaxConcurrent(Number(e.target.value) || 1)}
                    className="mt-1.5 w-full rounded-[10px] border border-divider bg-white px-3.5 py-2.5 text-[13.5px] text-ink outline-none transition-colors focus:border-accent"
                  />
                </Field>
                <Field label="失败重试次数">
                  <input
                    type="number"
                    min={0}
                    value={retryTimes}
                    onChange={(e) => setRetryTimes(Number(e.target.value) || 0)}
                    className="mt-1.5 w-full rounded-[10px] border border-divider bg-white px-3.5 py-2.5 text-[13.5px] text-ink outline-none transition-colors focus:border-accent"
                  />
                </Field>
              </div>
            </Section>

            {/* Java 设置 */}
            <Section icon={<Coffee size={16} />} title="Java">
              <div className="flex items-center justify-between rounded-[10px] border border-divider px-3.5 py-2.5">
                <span className="text-[13.5px] text-ink">自动检测系统 Java</span>
                <button
                  onClick={() => setAutoDetect(!autoDetect)}
                  className={`relative h-6 w-10 shrink-0 rounded-full transition-colors ${
                    autoDetect ? "bg-accent" : "bg-black/[0.12]"
                  }`}
                  aria-label="自动检测"
                >
                  <span
                    className={`absolute top-0.5 h-5 w-5 rounded-full bg-white shadow transition-all ${
                      autoDetect ? "left-[18px]" : "left-0.5"
                    }`}
                  />
                </button>
              </div>
              {!autoDetect && (
                <Field label="Java 可执行文件路径">
                  <input
                    value={javaPath}
                    onChange={(e) => setJavaPath(e.target.value)}
                    placeholder="/usr/bin/java"
                    className="mt-1.5 w-full rounded-[10px] border border-divider bg-white px-3.5 py-2.5 text-[13.5px] text-ink outline-none transition-colors focus:border-accent"
                  />
                </Field>
              )}
            </Section>

            {/* 外观 */}
            <Section icon={<FolderOpen size={16} />} title="外观与语言">
              <div className="grid grid-cols-2 gap-4">
                <Field label="主题">
                  <select
                    value={theme}
                    onChange={(e) => setTheme(e.target.value)}
                    className="mt-1.5 w-full rounded-[10px] border border-divider bg-white px-3.5 py-2.5 text-[13.5px] text-ink outline-none transition-colors focus:border-accent"
                  >
                    {Object.entries(themeLabels).map(([k, v]) => (
                      <option key={k} value={k}>
                        {v}
                      </option>
                    ))}
                  </select>
                </Field>
                <Field label="语言">
                  <select
                    value={language}
                    onChange={(e) => setLanguage(e.target.value)}
                    className="mt-1.5 w-full rounded-[10px] border border-divider bg-white px-3.5 py-2.5 text-[13.5px] text-ink outline-none transition-colors focus:border-accent"
                  >
                    <option value="zh-CN">简体中文</option>
                    <option value="en-US">English</option>
                  </select>
                </Field>
              </div>
            </Section>

            {s.error && (
              <p className="rounded-[10px] bg-red-50 px-3.5 py-2.5 text-[12.5px] text-red-600">
                {s.error}
              </p>
            )}

            <motion.button
              whileTap={{ scale: 0.98 }}
              onClick={handleSave}
              disabled={s.saving}
              className="flex items-center justify-center gap-2 rounded-[12px] bg-accent py-2.5 text-[13.5px] font-semibold text-white transition-colors hover:bg-accent-hover disabled:opacity-50"
            >
              {s.saving ? <Loader2 size={14} className="animate-spin" /> : <Save size={14} />}
              保存设置
            </motion.button>
          </div>
        ) : (
          <div className="mt-6 flex items-center justify-center rounded-[16px] bg-white py-14 shadow-card">
            <Loader2 size={18} className="animate-spin text-ink-3" />
          </div>
        )}
      </motion.div>
    </div>
  );
}

function Section({ icon, title, children }: { icon: React.ReactNode; title: string; children: React.ReactNode }) {
  return (
    <div className="rounded-[16px] bg-white p-5 shadow-card">
      <div className="mb-4 flex items-center gap-2 text-ink">
        <span className="text-ink-3">{icon}</span>
        <h2 className="text-[14.5px] font-semibold">{title}</h2>
      </div>
      <div className="flex flex-col gap-4">{children}</div>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="flex flex-col gap-1.5">
      <span className="text-[12.5px] font-medium text-ink-2">{label}</span>
      {children}
    </label>
  );
}
