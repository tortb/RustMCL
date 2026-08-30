import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { motion, Reorder } from "framer-motion";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Plus,
  Trash2,
  Star,
  Play,
  RefreshCw,
  X,
  Server as ServerIcon,
  Loader2,
  FileUp,
} from "lucide-react";
import { useServerStore } from "../stores/servers";
import { importServers, listInstances } from "../lib/api";
import type { Instance, ServerEntry, ServerStatus } from "../lib/types";

const ease = [0.32, 0.72, 0, 1] as const;

function latencyColor(ms: number | null | undefined): string {
  if (ms === null || ms === undefined || ms < 0) return "text-ink-3 bg-hover";
  if (ms < 80) return "text-success-600 bg-success-50";
  if (ms < 150) return "text-warning-600 bg-warning-50";
  return "text-danger-500 bg-danger-50";
}

function latencyLabel(ms: number | null | undefined, t: (key: string) => string): string {
  if (ms === null || ms === undefined || ms < 0) return t("servers.offline");
  return `${ms}ms`;
}

// 简易 favicon 解析:status favicon 是 data:image/png;base64,...;直接用原样展示
function ServerFavicon({ status }: { status?: ServerStatus | null }) {
  const favicon = status?.favicon;
  if (favicon && favicon.startsWith("data:image")) {
    return <img src={favicon} alt="" className="h-9 w-9 rounded-md object-contain" />;
  }
  return (
    <div className="flex h-9 w-9 items-center justify-center rounded-md bg-nav-active text-accent">
      <ServerIcon size={17} strokeWidth={1.8} />
    </div>
  );
}

export default function Servers() {
  const { t } = useTranslation();
  const s = useServerStore();
  const [statusMap, setStatusMap] = useState<Record<string, ServerStatus>>({});
  const [showAdd, setShowAdd] = useState(false);
  const [joinServer, setJoinServer] = useState<ServerEntry | null>(null);
  const [instances, setInstances] = useState<Instance[]>([]);

  useEffect(() => {
    s.load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handlePing = async (id: string) => {
    const st = await s.ping(id);
    setStatusMap((m) => ({ ...m, [id]: st ?? { ...m[id], ok: false } }));
  };

  const handlePingAll = async () => {
    const ids = s.servers.map((x) => x.id);
    for (const id of ids) {
      const st = await s.ping(id);
      setStatusMap((m) => ({ ...m, [id]: st ?? { ...m[id], ok: false } }));
    }
  };

  const openJoin = async (sv: ServerEntry) => {
    setJoinServer(sv);
    try {
      const list = await listInstances();
      setInstances(list);
    } catch {
      setInstances([]);
    }
  };

  const doJoin = async (instId: string) => {
    if (!joinServer) return;
    await s.join(joinServer.id, instId);
    setJoinServer(null);
  };

  const [importMsg, setImportMsg] = useState("");
  const handleImportDat = async () => {
    const selected = await open({
      multiple: false,
      filters: [{ name: t("servers.importFilterName"), extensions: ["dat"] }],
    });
    if (!selected || Array.isArray(selected)) return;
    setImportMsg("");
    try {
      const imported = await importServers(selected);
      setImportMsg(t("servers.importSuccess", { count: imported.length }));
      await s.load();
    } catch (e) {
      setImportMsg(String(e));
    }
  };

  return (
    <div className="flex-1 overflow-y-auto bg-bg px-6 py-8">
      <motion.div
        initial={{ opacity: 0, y: 10 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.35, ease }}
        className="mx-auto max-w-3xl"
      >
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-[24px] font-bold tracking-tight text-ink">{t("servers.title")}</h1>
            <p className="mt-1 text-[13px] text-ink-3">{t("servers.subtitle")}</p>
          </div>
          <div className="flex items-center gap-2">
            <motion.button
              whileTap={{ scale: 0.96 }}
              onClick={handleImportDat}
              className="flex items-center gap-1.5 rounded-[10px] border border-divider px-3.5 py-2 text-[12.5px] font-medium text-ink-2 transition-colors hover:bg-hover"
            >
              <FileUp size={13} />
              {t("servers.import")}
            </motion.button>
            <motion.button
              whileTap={{ scale: 0.96 }}
              onClick={handlePingAll}
              className="flex items-center gap-1.5 rounded-[10px] border border-divider px-3.5 py-2 text-[12.5px] font-medium text-ink-2 transition-colors hover:bg-hover"
            >
              <RefreshCw size={13} />
              {t("servers.pingAll")}
            </motion.button>
            <motion.button
              whileTap={{ scale: 0.96 }}
              onClick={() => setShowAdd(true)}
              className="flex items-center gap-2 rounded-[12px] bg-accent px-4 py-2.5 text-[13.5px] font-semibold text-on-accent transition-colors hover:bg-accent-hover"
            >
              <Plus size={16} strokeWidth={2.4} />
              {t("servers.addServer")}
            </motion.button>
          </div>
        </div>

        {s.error && (
          <p className="mt-4 rounded-[10px] bg-danger-50 px-3.5 py-2.5 text-[12.5px] text-danger-600">
            {s.error}
          </p>
        )}

        {importMsg && (
          <p className="mt-4 rounded-[10px] bg-hover px-3.5 py-2.5 text-[12.5px] text-ink-2">
            {importMsg}
          </p>
        )}

        {s.servers.length === 0 && !s.loading ? (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ delay: 0.15, duration: 0.3 }}
            className="mt-8 flex flex-col items-center gap-3 rounded-[16px] bg-card py-14 shadow-card"
          >
            <ServerIcon size={28} className="text-ink-3" strokeWidth={1.5} />
            <p className="text-[13.5px] text-ink-3">{t("servers.empty")}</p>
          </motion.div>
        ) : (
          <Reorder.Group
            axis="y"
            values={s.servers}
            onReorder={(newOrder) => s.reorder(newOrder)}
            className="mt-6 flex flex-col gap-3"
          >
            {s.servers.map((sv, i) => {
              const st = statusMap[sv.id];
              const pinged = st?.ok === true;
              return (
                <Reorder.Item
                  key={sv.id}
                  value={sv}
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: 0.05 * i, duration: 0.3, ease }}
                  className="flex cursor-grab items-center gap-4 rounded-[16px] bg-card px-5 py-4 shadow-card active:cursor-grabbing"
                >
                  <button
                    onClick={() => s.toggleFavorite(sv.id, !sv.is_favorite)}
                    className="shrink-0 text-ink-3 transition-colors hover:text-warning-500"
                    aria-label={t("servers.favorite")}
                  >
                    <Star
                      size={18}
                      className={sv.is_favorite ? "fill-yellow-400 text-warning-500" : ""}
                    />
                  </button>

                  <ServerFavicon status={st} />

                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <span className="truncate text-[14px] font-semibold text-ink">
                        {sv.name || sv.address}
                      </span>
                      <span className="text-[11.5px] text-ink-3">
                        {sv.address}:{sv.port}
                      </span>
                    </div>
                    {pinged && (
                      <div className="mt-0.5 flex items-center gap-2">
                        <p className="truncate text-[12.5px] text-ink-2">
                          {st!.motd || t("servers.noMOTD")}
                        </p>
                        <span className="shrink-0 text-[11.5px] text-ink-3">
                          {t("servers.players", {
                            online: st!.players_online,
                            max: st!.players_max,
                          })}
                        </span>
                      </div>
                    )}
                  </div>

                  <span
                    className={`shrink-0 rounded-full px-2.5 py-1 text-[11.5px] font-medium ${latencyColor(
                      pinged ? st!.latency_ms : null,
                    )}`}
                  >
                    {s.pingingId === sv.id ? t("servers.pinging") : latencyLabel(pinged ? st!.latency_ms : null, t)}
                  </span>

                  <button
                    onClick={() => handlePing(sv.id)}
                    className="shrink-0 rounded-[10px] border border-divider p-2 text-ink-2 transition-colors hover:bg-hover"
                    aria-label={t("servers.ping")}
                  >
                    {s.pingingId === sv.id ? (
                      <Loader2 size={14} className="animate-spin" />
                    ) : (
                      <RefreshCw size={14} />
                    )}
                  </button>

                  <motion.button
                    whileTap={{ scale: 0.95 }}
                    onClick={() => openJoin(sv)}
                    className="flex shrink-0 items-center gap-1.5 rounded-[10px] bg-accent px-3.5 py-2 text-[12.5px] font-semibold text-on-accent transition-colors hover:bg-accent-hover"
                  >
                    <Play size={13} fill="white" strokeWidth={0} />
                    {t("servers.join")}
                  </motion.button>

                  <motion.button
                    whileTap={{ scale: 0.95 }}
                    onClick={() => s.remove(sv.id)}
                    className="shrink-0 rounded-[10px] border border-divider p-2 text-ink-3 transition-colors hover:bg-danger-50 hover:text-danger-500"
                    aria-label={t("servers.delete")}
                  >
                    <Trash2 size={14} />
                  </motion.button>
                </Reorder.Item>
              );
            })}
          </Reorder.Group>
        )}
      </motion.div>

      <AddServerModal open={showAdd} onClose={() => setShowAdd(false)} onSaved={() => s.load()} />

      {/* 加入实例选择 */}
      <AnimatePresenceWrap show={joinServer !== null} onClose={() => setJoinServer(null)} title={t("servers.chooseInstance")}>
        {instances.length === 0 ? (
          <p className="py-4 text-[13px] text-ink-3">{t("servers.noInstance")}</p>
        ) : (
          <div className="flex flex-col gap-2">
            {instances.map((inst) => (
              <button
                key={inst.id}
                onClick={() => doJoin(inst.id)}
                className="flex items-center justify-between rounded-[10px] border border-divider px-3.5 py-2.5 text-left transition-colors hover:bg-hover"
              >
                <span className="text-[13px] font-medium text-ink">{inst.name}</span>
                <span className="text-[11.5px] text-ink-3">MC {inst.mc_version}</span>
              </button>
            ))}
          </div>
        )}
      </AnimatePresenceWrap>
    </div>
  );
}

function AddServerModal({
  open,
  onClose,
  onSaved,
}: {
  open: boolean;
  onClose: () => void;
  onSaved: () => void;
}) {
  const { t } = useTranslation();
  const s = useServerStore();
  const [name, setName] = useState("");
  const [address, setAddress] = useState("");
  const [port, setPort] = useState(25565);
  const [saving, setSaving] = useState(false);

  const submit = async () => {
    if (!address.trim()) return;
    setSaving(true);
    await s.add(name.trim() || address.trim(), address.trim(), port || 25565);
    setSaving(false);
    onSaved();
    setName("");
    setAddress("");
    setPort(25565);
    onClose();
  };

  return (
    <AnimatePresenceWrap show={open} onClose={onClose} title={t("servers.addServer")}>
      <div className="flex flex-col gap-4">
        <div>
          <label className="text-[12.5px] font-medium text-ink-2">{t("servers.nameLabel")}</label>
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder={t("servers.namePlaceholder")}
            className="mt-1.5 w-full rounded-[10px] border border-divider bg-card px-3.5 py-2.5 text-[13.5px] text-ink outline-none transition-colors focus:border-accent"
          />
        </div>
        <div>
          <label className="text-[12.5px] font-medium text-ink-2">{t("servers.addressLabel")}</label>
          <input
            value={address}
            onChange={(e) => setAddress(e.target.value)}
            placeholder="play.example.com"
            className="mt-1.5 w-full rounded-[10px] border border-divider bg-card px-3.5 py-2.5 text-[13.5px] text-ink outline-none transition-colors focus:border-accent"
          />
        </div>
        <div>
          <label className="text-[12.5px] font-medium text-ink-2">{t("servers.portLabel")}</label>
          <input
            type="number"
            value={port}
            onChange={(e) => setPort(Number(e.target.value) || 25565)}
            className="mt-1.5 w-full rounded-[10px] border border-divider bg-card px-3.5 py-2.5 text-[13.5px] text-ink outline-none transition-colors focus:border-accent"
          />
        </div>
        <button
          onClick={submit}
          disabled={saving || !address.trim()}
          className="flex items-center justify-center gap-2 rounded-[12px] bg-accent py-2.5 text-[13.5px] font-semibold text-on-accent transition-colors hover:bg-accent-hover disabled:opacity-50"
        >
          {saving && <Loader2 size={14} className="animate-spin" />}
          {t("common.save")}
        </button>
      </div>
    </AnimatePresenceWrap>
  );
}

// 简易模态壳(复用 framer-motion 动画)
function AnimatePresenceWrap({
  show,
  onClose,
  title,
  children,
}: {
  show: boolean;
  onClose: () => void;
  title: string;
  children: React.ReactNode;
}) {
  const { t } = useTranslation();
  if (!show) return null;
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 backdrop-blur-sm">
      <motion.div
        initial={{ opacity: 0, scale: 0.95, y: 8 }}
        animate={{ opacity: 1, scale: 1, y: 0 }}
        transition={{ duration: 0.28, ease }}
        className="w-[420px] rounded-[20px] bg-card p-6 shadow-[0_24px_64px_rgba(0,0,0,0.16)]"
      >
        <div className="flex items-center justify-between">
          <h2 className="text-[17px] font-bold tracking-tight text-ink">{title}</h2>
          <button
            onClick={onClose}
            className="rounded-full p-1.5 text-ink-3 transition-colors hover:bg-hover"
            aria-label={t("common.close")}
          >
            <X size={16} />
          </button>
        </div>
        <div className="mt-4">{children}</div>
      </motion.div>
    </div>
  );
}
