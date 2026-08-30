import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Save,
  Loader2,
  Coffee,
  Download,
  FolderOpen,
  Zap,
  KeyRound,
  RefreshCw,
  ArrowUpCircle,
  UserRound,
  Upload,
  Trash2,
} from "lucide-react";
import {
  checkForUpdate,
  getOfflineSkin,
  getSkinImage,
  importSkin,
  installUpdate,
  listMirrors,
  listSkins,
  removeSkin,
  setMirror,
  setOfflineSkin,
  testAllMirrorSpeed,
  uploadSkin,
} from "../lib/api";
import type { MirrorSpec, SkinEntry, SpeedResult, UpdateInfo } from "../lib/types";
import { useSettingsStore } from "../stores/settings";
import { useAccountStore } from "../stores/account";
import SkinPreview from "../components/SkinPreview";
import { AppSelect } from "../components/AppSelect";

const ease = [0.32, 0.72, 0, 1] as const;

export default function Settings() {
  const { t } = useTranslation();
  const s = useSettingsStore();
  const [maxConcurrent, setMaxConcurrent] = useState(8);
  const [retryTimes, setRetryTimes] = useState(3);
  const [autoDetect, setAutoDetect] = useState(true);
  const [javaPath, setJavaPath] = useState("");
  const [theme, setTheme] = useState("dark");
  const [language, setLanguage] = useState("zh-CN");

  // 下载源
  const [mirrors, setMirrors] = useState<MirrorSpec[]>([]);
  const [selectedMirror, setSelectedMirror] = useState("official");
  const [customBase, setCustomBase] = useState("");
  const [speeds, setSpeeds] = useState<SpeedResult[]>([]);
  const [testing, setTesting] = useState(false);
  const [cfKey, setCfKey] = useState("");

  // 自更新
  const [checking, setChecking] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [updateError, setUpdateError] = useState("");
  const [updateMsg, setUpdateMsg] = useState("");

  // 皮肤
  const [skins, setSkins] = useState<SkinEntry[]>([]);
  const [selectedSkin, setSelectedSkin] = useState<SkinEntry | null>(null);
  const [skinModel, setSkinModel] = useState<"classic" | "slim">("classic");
  const [skinPreview, setSkinPreview] = useState<string | null>(null);
  const [skinImporting, setSkinImporting] = useState(false);
  const [skinUploading, setSkinUploading] = useState(false);
  const [skinMsg, setSkinMsg] = useState<{ ok: boolean; text: string } | null>(null);

  const activeAccount = useAccountStore((s) => s.active);
  const canUpload = activeAccount?.account_type === "microsoft";
  const isOffline = activeAccount?.account_type === "offline";

  // 离线账号皮肤关联
  const [offlineSkinId, setOfflineSkinId] = useState<string | null>(null);
  const [offlineSkinMsg, setOfflineSkinMsg] = useState("");

  useEffect(() => {
    if (isOffline && activeAccount) {
      getOfflineSkin(activeAccount.id)
        .then(setOfflineSkinId)
        .catch(() => setOfflineSkinId(null));
    } else {
      setOfflineSkinId(null);
    }
  }, [activeAccount, isOffline]);

  useEffect(() => {
    s.load();
    listMirrors()
      .then(setMirrors)
      .catch(() => setMirrors([]));
    listSkins()
      .then(setSkins)
      .catch(() => setSkins([]));
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
      setSelectedMirror(s.config.download.mirror);
      setCustomBase(s.config.download.mirror_custom_base ?? "");
      setCfKey(s.config.curseforge_api_key ?? "");
    }
  }, [s.config]);

  const handleSpeedTest = async () => {
    setTesting(true);
    try {
      const results = await testAllMirrorSpeed();
      setSpeeds(results);
    } catch (e) {
      setSpeeds([]);
      console.error(e);
    } finally {
      setTesting(false);
    }
  };

  const handleCheckUpdate = async () => {
    setChecking(true);
    setUpdateError("");
    setUpdateInfo(null);
    try {
      const info = await checkForUpdate();
      setUpdateInfo(info);
    } catch (e) {
      setUpdateError(String(e));
    } finally {
      setChecking(false);
    }
  };

  const handleInstallUpdate = async () => {
    setInstalling(true);
    setUpdateError("");
    try {
      const msg = await installUpdate();
      setUpdateMsg(msg);
    } catch (e) {
      setUpdateError(String(e));
    } finally {
      setInstalling(false);
    }
  };

  const reloadSkins = async () => {
    const list = await listSkins();
    setSkins(list);
    return list;
  };

  const handleImportSkin = async () => {
    const selected = await open({
      multiple: false,
      filters: [{ name: "皮肤", extensions: ["png"] }],
    });
    if (!selected || Array.isArray(selected)) return;
    setSkinImporting(true);
    setSkinMsg(null);
    try {
      const entry = await importSkin(selected, "", skinModel);
      await reloadSkins();
      setSelectedSkin(entry);
      const url = await getSkinImage(entry.id);
      setSkinPreview(url);
      setSkinMsg({ ok: true, text: "已导入皮肤库" });
    } catch (e) {
      setSkinMsg({ ok: false, text: String(e) });
    } finally {
      setSkinImporting(false);
    }
  };

  const handleSelectSkin = async (entry: SkinEntry) => {
    setSelectedSkin(entry);
    setSkinModel(entry.model);
    setSkinMsg(null);
    try {
      const url = await getSkinImage(entry.id);
      setSkinPreview(url);
    } catch (e) {
      setSkinPreview(null);
      setSkinMsg({ ok: false, text: String(e) });
    }
  };

  const handleUploadSkin = async () => {
    if (!selectedSkin) return;
    setSkinUploading(true);
    setSkinMsg(null);
    try {
      await uploadSkin(selectedSkin.id);
      setSkinMsg({ ok: true, text: `已将「${selectedSkin.name}」上传到微软账号` });
    } catch (e) {
      setSkinMsg({ ok: false, text: String(e) });
    } finally {
      setSkinUploading(false);
    }
  };

  const handleRemoveSkin = async (entry: SkinEntry) => {
    setSkinMsg(null);
    try {
      await removeSkin(entry.id);
      await reloadSkins();
      if (selectedSkin?.id === entry.id) {
        setSelectedSkin(null);
        setSkinPreview(null);
      }
      setSkinMsg({ ok: true, text: "已删除" });
    } catch (e) {
      setSkinMsg({ ok: false, text: String(e) });
    }
  };

  const handleSetOfflineSkin = async () => {
    if (!selectedSkin || !activeAccount) return;
    setOfflineSkinMsg("");
    try {
      await setOfflineSkin(activeAccount.id, selectedSkin.id);
      setOfflineSkinId(selectedSkin.id);
      setOfflineSkinMsg("已设为该离线账号的皮肤");
    } catch (e) {
      setOfflineSkinMsg(String(e));
    }
  };

  const handleClearOfflineSkin = async () => {
    if (!activeAccount) return;
    setOfflineSkinMsg("");
    try {
      await setOfflineSkin(activeAccount.id, null);
      setOfflineSkinId(null);
      setOfflineSkinMsg("已清除离线皮肤关联");
    } catch (e) {
      setOfflineSkinMsg(String(e));
    }
  };

  const handleSave = async () => {
    if (!s.config) return;
    await setMirror(selectedMirror, customBase.trim() || null).catch(() => undefined);
    await s.save({
      ...s.config,
      general: { ...s.config.general, theme, language },
      java: { auto_detect: autoDetect, default_java_path: javaPath },
      download: {
        ...s.config.download,
        max_concurrent: maxConcurrent,
        retry_times: retryTimes,
        mirror: selectedMirror,
        mirror_custom_base: customBase.trim() || null,
      },
      curseforge_api_key: cfKey.trim() || null,
    });
  };

  return (
    <div className="flex-1 overflow-y-auto bg-bg px-6 py-8">
      <motion.div
        initial={{ opacity: 0, y: 10 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.35, ease }}
        className="mx-auto max-w-2xl"
      >
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-[24px] font-bold tracking-tight text-ink">{t("settings.title")}</h1>
            <p className="mt-1 text-[13px] text-ink-3">{t("settings.subtitle")}</p>
          </div>
        </div>

        {s.config ? (
          <div className="mt-6 flex flex-col gap-4">
            {/* 下载设置 */}
            <Section icon={<Download size={16} />} title={t("settings.section.download")}>
              <div className="grid grid-cols-2 gap-4">
                <Field label="最大并发数">
                  <input
                    type="number"
                    min={1}
                    value={maxConcurrent}
                    onChange={(e) => setMaxConcurrent(Number(e.target.value) || 1)}
                    className="mt-1.5 w-full rounded-[10px] border border-divider bg-card px-3.5 py-2.5 text-[13.5px] text-ink outline-none transition-colors focus:border-accent"
                  />
                </Field>
                <Field label="失败重试次数">
                  <input
                    type="number"
                    min={0}
                    value={retryTimes}
                    onChange={(e) => setRetryTimes(Number(e.target.value) || 0)}
                    className="mt-1.5 w-full rounded-[10px] border border-divider bg-card px-3.5 py-2.5 text-[13.5px] text-ink outline-none transition-colors focus:border-accent"
                  />
                </Field>
              </div>
            </Section>

            {/* 下载源 */}
            <Section icon={<Zap size={16} />} title={t("settings.section.mirror")}>
              <Field label="镜像源">
                <AppSelect
                  value={selectedMirror}
                  onChange={(v) => setSelectedMirror(v)}
                  className="mt-1.5 w-full"
                  options={[
                    ...mirrors.map((m) => ({ value: m.id, label: m.name })),
                    ...(mirrors.every((m) => m.id !== "custom")
                      ? [{ value: "custom", label: "自定义" }]
                      : []),
                  ]}
                />
              </Field>

              {selectedMirror === "custom" && (
                <Field label="自定义镜像基址">
                  <input
                    value={customBase}
                    onChange={(e) => setCustomBase(e.target.value)}
                    placeholder="https://example.com"
                    className="mt-1.5 w-full rounded-[10px] border border-divider bg-card px-3.5 py-2.5 text-[13.5px] text-ink outline-none transition-colors focus:border-accent"
                  />
                </Field>
              )}

              <div className="flex items-center justify-between gap-3">
                <button
                  onClick={handleSpeedTest}
                  disabled={testing}
                  className="flex items-center gap-1.5 rounded-[10px] border border-divider px-3.5 py-2 text-[12.5px] font-medium text-ink-2 transition-colors hover:bg-hover disabled:opacity-50"
                >
                  {testing ? <Loader2 size={13} className="animate-spin" /> : <Zap size={13} />}
                  测速全部节点
                </button>
                <span className="text-[12px] text-ink-3">切换后保存生效</span>
              </div>

              {speeds.length > 0 && (
                <ul className="flex flex-col gap-2">
                  {speeds.map((r) => (
                    <li
                      key={r.id}
                      className={`flex items-center gap-3 rounded-[10px] border px-3.5 py-2.5 text-[12.5px] ${
                        r.ok ? "border-divider" : "border-danger-50 bg-danger-50"
                      }`}
                    >
                      <span className="w-16 shrink-0 font-medium text-ink">{r.name || r.id}</span>
                      {r.ok ? (
                        <>
                          <div className="h-1.5 flex-1 overflow-hidden rounded-full bg-hover">
                            <div
                              className="h-full rounded-full bg-accent transition-all"
                              style={{ width: `${Math.min(100, r.throughput / 100)}%` }}
                            />
                          </div>
                          <span className="w-24 shrink-0 text-right text-ink-2">
                            {r.latency_ms >= 1000
                              ? `${(r.latency_ms / 1000).toFixed(1)}s`
                              : `${r.latency_ms}ms`}
                            · {r.throughput.toFixed(0)} KB/s
                          </span>
                        </>
                      ) : (
                        <span className="flex-1 truncate text-danger-500">{r.error}</span>
                      )}
                    </li>
                  ))}
                </ul>
              )}
            </Section>

            {/* 网络 */}
            <Section icon={<KeyRound size={16} />} title={t("settings.section.network")}>
              <Field label="CurseForge API Key">
                <input
                  value={cfKey}
                  onChange={(e) => setCfKey(e.target.value)}
                  placeholder="可选,用于 CurseForge 源搜索/安装"
                  className="mt-1.5 w-full rounded-[10px] border border-divider bg-card px-3.5 py-2.5 text-[13.5px] text-ink outline-none transition-colors focus:border-accent"
                />
              </Field>
              <p className="text-[11.5px] text-ink-3">
                前往 curseforge.com 申请个人 API Key;配置后请在 Mod 页切换到 CurseForge 源。
              </p>
            </Section>

            {/* 检查更新 */}
            <Section icon={<RefreshCw size={16} />} title={t("settings.section.update")}>
              <div className="flex items-center justify-between gap-3">
                <div className="flex flex-col gap-1 text-[12.5px] text-ink-2">
                  <span>检查是否有新版本可用</span>
                  {updateInfo && (
                    <span className="font-medium text-ink">
                      当前 v{updateInfo.current} · 最新 v{updateInfo.latest}
                    </span>
                  )}
                </div>
                <button
                  onClick={handleCheckUpdate}
                  disabled={checking}
                  className="flex shrink-0 items-center gap-1.5 rounded-[10px] border border-divider px-3.5 py-2 text-[12.5px] font-medium text-ink-2 transition-colors hover:bg-hover disabled:opacity-50"
                >
                  {checking ? <Loader2 size={13} className="animate-spin" /> : <RefreshCw size={13} />}
                  检查
                </button>
              </div>

              {updateInfo && updateInfo.has_update && (
                <div className="rounded-[10px] border border-accent/40 bg-accent/[0.06] p-3.5 text-[12.5px]">
                  <div className="mb-1 flex items-center gap-1.5 font-semibold text-accent">
                    <ArrowUpCircle size={14} />
                    发现新版本
                  </div>
                  {updateInfo.notes ? (
                    <p className="break-words text-ink-2">{updateInfo.notes}</p>
                  ) : (
                    <p className="text-ink-3">前往发布页获取新版本。</p>
                  )}
                  <button
                    onClick={handleInstallUpdate}
                    disabled={installing}
                    className="mt-3 flex items-center gap-1.5 rounded-[10px] bg-accent px-3.5 py-2 text-[12.5px] font-semibold text-on-accent transition-colors hover:bg-accent-hover disabled:opacity-50"
                  >
                    {installing ? <Loader2 size={13} className="animate-spin" /> : <RefreshCw size={13} />}
                    {installing ? "正在更新..." : "下载并更新"}
                  </button>
                </div>
              )}

              {updateInfo && !updateInfo.has_update && (
                <div className="rounded-[10px] border border-divider bg-hover p-3.5 text-[12.5px] text-ink-2">
                  当前已是最新版本。
                </div>
              )}

              {updateMsg && (
                <p className="rounded-[10px] bg-success-50 px-3.5 py-2.5 text-[12.5px] text-success-600">
                  {updateMsg}
                </p>
              )}

              {updateError && (
                <p className="rounded-[10px] bg-danger-50 px-3.5 py-2.5 text-[12.5px] text-danger-600">
                  {updateError}
                </p>
              )}
            </Section>

            {/* 皮肤 */}
            <Section icon={<UserRound size={16} />} title={t("settings.section.skin")}>
              <div className="flex flex-col gap-4 sm:flex-row">
                {/* 3D 预览 */}
                <div className="flex w-full shrink-0 items-center justify-center rounded-[12px] bg-hover p-3 sm:w-[240px]">
                  <SkinPreview skin={skinPreview} model={skinModel} width={210} height={300} />
                </div>

                {/* 皮肤库 */}
                <div className="flex min-w-0 flex-1 flex-col gap-3">
                  <div className="flex items-center gap-2">
                    <label className="text-[12.5px] font-medium text-ink-2">模型</label>
                    <AppSelect
                      value={skinModel}
                      onChange={(v) => setSkinModel(v as "classic" | "slim")}
                      options={[
                        { value: "classic", label: "经典(粗臂)" },
                        { value: "slim", label: "细臂" },
                      ]}
                    />
                    <button
                      onClick={handleImportSkin}
                      disabled={skinImporting}
                      className="ml-auto flex items-center gap-1.5 rounded-[10px] border border-divider px-3.5 py-2 text-[12.5px] font-medium text-ink-2 transition-colors hover:bg-hover disabled:opacity-50"
                    >
                      {skinImporting ? <Loader2 size={13} className="animate-spin" /> : <Download size={13} />}
                      导入 PNG
                    </button>
                  </div>

                  {canUpload ? (
                    <button
                      onClick={handleUploadSkin}
                      disabled={!selectedSkin || skinUploading}
                      className="flex items-center justify-center gap-1.5 rounded-[10px] bg-accent px-3.5 py-2 text-[12.5px] font-semibold text-on-accent transition-colors hover:bg-accent-hover disabled:opacity-50"
                    >
                      {skinUploading ? <Loader2 size={13} className="animate-spin" /> : <Upload size={13} />}
                      上传到当前微软账号
                    </button>
                  ) : isOffline ? (
                    <div className="flex flex-col gap-2">
                      <div className="flex flex-wrap items-center gap-2">
                        <button
                          onClick={handleSetOfflineSkin}
                          disabled={!selectedSkin}
                          className="flex items-center justify-center gap-1.5 rounded-[10px] bg-accent px-3.5 py-2 text-[12.5px] font-semibold text-on-accent transition-colors hover:bg-accent-hover disabled:opacity-50"
                        >
                          <UserRound size={13} />
                          {offlineSkinId ? "更新离线皮肤" : "设为此账号的离线皮肤"}
                        </button>
                        {offlineSkinId && (
                          <button
                            onClick={handleClearOfflineSkin}
                            className="flex items-center gap-1.5 rounded-[10px] border border-divider px-3.5 py-2 text-[12.5px] font-medium text-ink-2 transition-colors hover:bg-hover"
                          >
                            清除关联
                          </button>
                        )}
                      </div>
                      {offlineSkinMsg && (
                        <p className="text-[11.5px] text-ink-2">{offlineSkinMsg}</p>
                      )}
                      <p className="rounded-[10px] bg-hover px-3.5 py-2.5 text-[12px] text-ink-3">
                        离线账号无 Mojang 账号系统支撑,游戏内的皮肤实际渲染因版本而异(需额外方案)。此处将所选本地皮肤与该账号关联,便于本地管理与预览。
                      </p>
                    </div>
                  ) : (
                    <p className="rounded-[10px] bg-hover px-3.5 py-2.5 text-[12px] text-ink-3">
                      上传皮肤需先登录微软账号;离线账号可先导入本地皮肤库。
                    </p>
                  )}

                  <ul className="flex flex-wrap gap-2">
                    {skins.map((sk) => (
                      <li key={sk.id}>
                        <div
                          onClick={() => handleSelectSkin(sk)}
                          className={`flex cursor-pointer items-center gap-2 rounded-[10px] border px-3 py-2 text-[12.5px] transition-colors ${
                            selectedSkin?.id === sk.id
                              ? "border-accent bg-accent/[0.06] text-ink"
                              : "border-divider bg-card text-ink-2 hover:bg-hover"
                          }`}
                        >
                          <span className="max-w-[120px] truncate">{sk.name}</span>
                          <span className="rounded bg-hover px-1.5 py-0.5 text-[10.5px] text-ink-3">
                            {sk.width}x{sk.height}
                          </span>
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              handleRemoveSkin(sk);
                            }}
                            className="ml-1 text-ink-3 transition-colors hover:text-danger-500"
                            aria-label="删除"
                          >
                            <Trash2 size={13} />
                          </button>
                        </div>
                      </li>
                    ))}
                    {skins.length === 0 && (
                      <li className="text-[12px] text-ink-3">暂无皮肤,点击「导入 PNG」添加</li>
                    )}
                  </ul>

                  {skinMsg && (
                    <p
                      className={`rounded-[10px] px-3.5 py-2.5 text-[12px] ${
                        skinMsg.ok ? "bg-hover text-ink-2" : "bg-danger-50 text-danger-600"
                      }`}
                    >
                      {skinMsg.text}
                    </p>
                  )}
                </div>
              </div>
            </Section>

            {/* Java 设置 */}
            <Section icon={<Coffee size={16} />} title={t("settings.section.java")}>
              <div className="flex items-center justify-between rounded-[10px] border border-divider px-3.5 py-2.5">
                <span className="text-[13.5px] text-ink">自动检测系统 Java</span>
                <button
                  onClick={() => setAutoDetect(!autoDetect)}
                  className={`relative h-6 w-10 shrink-0 rounded-full transition-colors ${
                    autoDetect ? "bg-accent" : "bg-hover"
                  }`}
                  aria-label="自动检测"
                >
                  <span
                    className={`absolute top-0.5 h-5 w-5 rounded-full bg-card shadow transition-all ${
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
                    className="mt-1.5 w-full rounded-[10px] border border-divider bg-card px-3.5 py-2.5 text-[13.5px] text-ink outline-none transition-colors focus:border-accent"
                  />
                </Field>
              )}
            </Section>

            {/* 外观 */}
            <Section icon={<FolderOpen size={16} />} title={t("settings.section.appearance")}>
              <div className="grid grid-cols-2 gap-4">
                <Field label="主题">
                  <AppSelect
                    value={theme}
                    onChange={(v) => setTheme(v)}
                    className="mt-1.5 w-full"
                    options={[
                      { value: "dark", label: t("settings.theme.dark") },
                      { value: "light", label: t("settings.theme.light") },
                    ]}
                  />
                </Field>
                <Field label="语言">
                  <AppSelect
                    value={language}
                    onChange={(v) => setLanguage(v)}
                    className="mt-1.5 w-full"
                    options={[
                      { value: "zh-CN", label: t("settings.language.zh") },
                      { value: "en-US", label: t("settings.language.en") },
                    ]}
                  />
                </Field>
              </div>
            </Section>

            {s.error && (
              <p className="rounded-[10px] bg-danger-50 px-3.5 py-2.5 text-[12.5px] text-danger-600">
                {s.error}
              </p>
            )}

            <motion.button
              whileTap={{ scale: 0.98 }}
              onClick={handleSave}
              disabled={s.saving}
              className="flex items-center justify-center gap-2 rounded-[12px] bg-accent py-2.5 text-[13.5px] font-semibold text-on-accent transition-colors hover:bg-accent-hover disabled:opacity-50"
            >
              {s.saving ? <Loader2 size={14} className="animate-spin" /> : <Save size={14} />}
              {t("settings.save")}
            </motion.button>
          </div>
        ) : (
          <div className="mt-6 flex items-center justify-center rounded-[16px] bg-card py-14 shadow-card">
            <Loader2 size={18} className="animate-spin text-ink-3" />
          </div>
        )}
      </motion.div>
    </div>
  );
}

function Section({ icon, title, children }: { icon: React.ReactNode; title: string; children: React.ReactNode }) {
  return (
    <div className="rounded-[16px] bg-card p-5 shadow-card">
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
