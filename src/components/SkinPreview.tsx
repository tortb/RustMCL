import { useEffect, useRef } from "react";
import { SkinViewer, WaveAnimation } from "skinview3d";

interface Props {
  /** 皮肤图片 URL 或 data URL;为空时显示空模型 */
  skin?: string | null;
  /** classic / slim */
  model?: string;
  width?: number;
  height?: number;
}

/**
 * 3D 玩家皮肤预览(skinview3d / three.js)。
 * 只在挂载时创建一次 SkinViewer,皮肤变更时仅替换纹理,避免反复创建 WebGL 上下文。
 */
export default function SkinPreview({
  skin,
  model = "classic",
  width = 220,
  height = 320,
}: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const viewerRef = useRef<SkinViewer | null>(null);

  useEffect(() => {
    if (!canvasRef.current) return;
    const viewer = new SkinViewer({
      canvas: canvasRef.current,
      width,
      height,
      model: model === "slim" ? "slim" : "default",
      animation: new WaveAnimation(),
    });
    viewerRef.current = viewer;
    return () => {
      viewer.dispose();
      viewerRef.current = null;
    };
    // 仅挂载时初始化一次
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const viewer = viewerRef.current;
    if (!viewer) return;
    if (skin) {
      viewer
        .loadSkin(skin, { model: model === "slim" ? "slim" : "default" })
        .catch(() => undefined);
    } else {
      viewer.resetSkin();
    }
  }, [skin, model]);

  return <canvas ref={canvasRef} style={{ width, height, display: "block" }} />;
}
