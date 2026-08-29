import { useEffect, useState } from "react";
import {
  Box,
  Loader2,
  Trash2,
  Archive,
  Camera,
  RotateCcw,
  Folder,
  X,
} from "lucide-react";
import { useSavesStore } from "../stores/saves";
import { getScreenshotImage } from "../lib/api";

type Tab = "saves" | "backups" | "screenshots";

function formatBytes(n: number): string {
  if (n >= 1_048_576) return `${(n / 1_048_576).toFixed(1)}MB`;
  if (n >= 1024) return `${(n / 1024).toFixed(0)}KB`;
  return `${n}B`;
}

export default function SavePanel({ instanceId }: { instanceId: string }) {
  const s = useSavesStore();
  const [tab, setTab] = useState<Tab>("saves");
  // 截图画廊:分页 + 大图预览
  const [visibleCount, setVisibleCount] = useState(48);
  const [preview, setPreview] = useState<{ name: string; src: string } | null>(null);

  useEffect(() => {
    s.load(instanceId);
    setVisibleCount(48);
    setPreview(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [instanceId]);

  // 恢复目标存档名
  const [restoreTarget, setRestoreTarget] = useState<string>("");

  return (
    <div className="border-t border-divider px-5 py-4">
      <div className="flex items-center gap-2">
        {(
          [
            { key: "saves", label: "存档", icon: Box },
            { key: "backups", label: "备份", icon: Archive },
            { key: "screenshots", label: "截图", icon: Camera },
          ] as const
        ).map((t) => (
          <button
            key={t.key}
            onClick={() => setTab(t.key)}
            className={`flex items-center gap-1.5 rounded-full px-3.5 py-1.5 text-[12px] font-medium transition-colors ${
              tab === t.key
                ? "bg-accent text-white"
                : "border border-divider text-ink-2 hover:bg-black/[0.03]"
            }`}
          >
            <t.icon size={12} />
            {t.label}
          </button>
        ))}
        {s.loading && <Loader2 size={13} className="ml-auto animate-spin text-ink-3" />}
      </div>

      {s.message && (
        <p className="mt-3 rounded-[10px] bg-green-50 px-3.5 py-2.5 text-[12px] text-green-600">
          {s.message}
        </p>
      )}
      {s.error && (
        <p className="mt-3 rounded-[10px] bg-red-50 px-3.5 py-2.5 text-[12px] text-red-600">
          {s.error}
        </p>
      )}

      <div className="mt-3">
        {tab === "saves" && (
          <div className="flex flex-col gap-2">
            {s.saves.length === 0 && (
              <p className="py-4 text-center text-[12.5px] text-ink-3">还没有世界存档</p>
            )}
            {s.saves.map((sv) => (
              <div
                key={sv.name}
                className="flex items-center gap-3 rounded-[12px] border border-divider px-3.5 py-2.5"
              >
                <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-[10px] bg-badge-bg">
                  <Folder size={15} className="text-badge-text" />
                </div>
                <div className="min-w-0 flex-1">
                  <span className="block truncate text-[13px] font-medium text-ink">{sv.name}</span>
                  <span className="block text-[11px] text-ink-3">{formatBytes(sv.size_bytes)}</span>
                </div>
                <button
                  onClick={() => s.backup(sv.name)}
                  className="flex shrink-0 items-center gap-1 rounded-[8px] bg-accent px-2.5 py-1.5 text-[11.5px] font-medium text-white transition-colors hover:bg-accent-hover"
                >
                  <Archive size={12} />
                  备份
                </button>
                <button
                  onClick={() => {
                    if (confirm(`确定删除存档「${sv.name}」吗?`)) s.removeSave(sv.name);
                  }}
                  className="shrink-0 rounded-[8px] border border-divider p-1.5 text-ink-3 transition-colors hover:bg-red-50 hover:text-red-500"
                  aria-label="删除存档"
                >
                  <Trash2 size={13} />
                </button>
              </div>
            ))}
          </div>
        )}

        {tab === "backups" && (
          <div className="flex flex-col gap-2">
            {s.backups.length === 0 && (
              <p className="py-4 text-center text-[12.5px] text-ink-3">还没有备份,可在「存档」页一键备份</p>
            )}
            {s.backups.map((bk) => (
              <div
                key={bk.name}
                className="flex items-center gap-3 rounded-[12px] border border-divider px-3.5 py-2.5"
              >
                <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-[10px] bg-badge-bg">
                  <Archive size={15} className="text-badge-text" />
                </div>
                <div className="min-w-0 flex-1">
                  <span className="block truncate text-[12.5px] font-medium text-ink">{bk.name}</span>
                  <span className="block text-[11px] text-ink-3">{formatBytes(bk.size_bytes)}</span>
                </div>
                <input
                  value={restoreTarget}
                  onChange={(e) => setRestoreTarget(e.target.value)}
                  placeholder="恢复为..."
                  className="w-24 rounded-[8px] border border-divider px-2 py-1.5 text-[11.5px] text-ink outline-none focus:border-accent"
                />
                <button
                  onClick={() => {
                    const target = restoreTarget.trim() || bk.name;
                    if (confirm(`将备份恢复为「${target}」?`)) s.restore(bk.name, target);
                  }}
                  className="flex shrink-0 items-center gap-1 rounded-[8px] border border-divider px-2.5 py-1.5 text-[11.5px] font-medium text-ink-2 transition-colors hover:bg-black/[0.03]"
                >
                  <RotateCcw size={12} />
                  恢复
                </button>
              </div>
            ))}
          </div>
        )}

        {tab === "screenshots" && (
          <div className="flex flex-col gap-2">
            {s.screenshots.length === 0 && (
              <p className="py-4 text-center text-[12.5px] text-ink-3">还没有截图</p>
            )}
            {s.screenshots.length > 0 && (
              <>
                <div className="grid grid-cols-4 gap-2 sm:grid-cols-6">
                  {s.screenshots.slice(0, visibleCount).map((sc) => (
                    <ShotThumb
                      key={sc.name}
                      instanceId={instanceId}
                      name={sc.name}
                      onOpen={(src) => setPreview({ name: sc.name, src })}
                      onDelete={() => s.removeScreenshot(sc.name)}
                    />
                  ))}
                </div>
                {visibleCount < s.screenshots.length && (
                  <button
                    onClick={() => setVisibleCount((v) => v + 48)}
                    className="mt-2 flex items-center justify-center gap-1.5 rounded-[10px] border border-divider px-3.5 py-2 text-[12.5px] font-medium text-ink-2 transition-colors hover:bg-black/[0.03]"
                  >
                    <Camera size={13} />
                    加载更多
                  </button>
                )}
              </>
            )}
          </div>
        )}
      </div>

      {preview && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6 backdrop-blur-sm">
          <div className="max-h-[90vh] max-w-[90vw]">
            <img
              src={preview.src}
              alt={preview.name}
              className="max-h-[88vh] rounded-[12px] object-contain shadow-2xl"
            />
            <div className="mt-3 flex items-center justify-center gap-3">
              <span className="break-all text-[12px] text-white/70">{preview.name}</span>
              <button
                onClick={() => s.removeScreenshot(preview.name)}
                className="flex items-center gap-1.5 rounded-[10px] border border-white/20 px-3.5 py-2 text-[12.5px] font-medium text-white transition-colors hover:bg-red-500 hover:border-red-500"
              >
                <Trash2 size={13} />
                删除
              </button>
              <button
                onClick={() => setPreview(null)}
                className="flex items-center gap-1.5 rounded-[10px] border border-white/20 px-3.5 py-2 text-[12.5px] font-medium text-white transition-colors hover:bg-white/10"
              >
                <X size={13} />
                关闭
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

/** 单张截图缩略图:按需拉取 data URL,懒加载渲染 */
function ShotThumb({
  instanceId,
  name,
  onOpen,
  onDelete,
}: {
  instanceId: string;
  name: string;
  onOpen: (src: string) => void;
  onDelete: () => void;
}) {
  const [src, setSrc] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    getScreenshotImage(instanceId, name)
      .then((u) => {
        if (alive) setSrc(u);
      })
      .catch(() => {
        if (alive) setSrc(null);
      });
    return () => {
      alive = false;
    };
  }, [instanceId, name]);

  return (
    <div className="group relative aspect-video overflow-hidden rounded-[10px] border border-divider bg-black/[0.03]">
      {src ? (
        <button
          onClick={() => onOpen(src)}
          className="h-full w-full"
        >
          <img src={src} alt={name} loading="lazy" className="h-full w-full object-cover" />
        </button>
      ) : (
        <div className="flex h-full w-full items-center justify-center text-ink-3">
          <Camera size={16} />
        </div>
      )}
      <button
        onClick={onDelete}
        className="absolute right-1.5 top-1.5 rounded-[8px] bg-black/40 p-1.5 text-white opacity-0 transition-opacity hover:bg-red-500 group-hover:opacity-100"
        aria-label="删除截图"
      >
        <Trash2 size={12} />
      </button>
    </div>
  );
}
