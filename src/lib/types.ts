/**
 * 与 Rust struct 对应的 TS 类型,需与 src-tauri/src/db/schema.rs 保持同步
 */

export interface AppInfo {
  name: string;
  version: string;
  data_dir: string;
}

export type Loader = "vanilla" | "forge" | "fabric" | "quilt";

export interface Instance {
  id: string;
  name: string;
  mc_version: string;
  loader: Loader | null;
  loader_version: string | null;
  game_dir: string;
  icon_path: string | null;
  created_at: number;
  last_played: number | null;
}

export type AccountType = "microsoft" | "offline";

export interface Account {
  id: string;
  username: string;
  uuid: string;
  account_type: AccountType;
  is_active: boolean;
  refreshed_at: number | null;
}

export type ModSource = "modrinth" | "curseforge" | "local";

export interface ModEntry {
  id: string;
  instance_id: string;
  file_name: string;
  source: ModSource | null;
  project_id: string | null;
  version_id: string | null;
  enabled: boolean;
}

export interface AssetCache {
  sha1: string;
  path: string;
  size: number;
}

// ---------- 版本清单 ----------

export type VersionType = "release" | "snapshot" | "old_beta" | "old_alpha";
export type VersionFilter = "all" | "release" | "snapshot";

export interface VersionInfo {
  id: string;
  version_type: VersionType;
  url: string;
  time: string;
  release_time: string;
  sha1: string;
  compliance_level: number | null;
}

export interface VersionManifest {
  latest: { release: string; snapshot: string };
  versions: VersionInfo[];
}

// ---------- 下载 / 启动 ----------

export interface DownloadProgress {
  phase: "core" | "assets";
  current: number;
  total: number;
  file: string;
}

export interface DownloadFinished {
  ok: boolean;
  error: string;
}

export interface LoaderInstallFinished {
  ok: boolean;
  error: string;
}

export interface GameLog {
  line: string;
}

export interface GameExit {
  code: number;
}

// ---------- 微软登录 ----------

export interface MsDeviceInfo {
  user_code: string;
  verification_uri: string;
  expires_in: number;
  message: string | null;
}

export interface MsLoginStatus {
  stage: string;
  message: string;
}

export interface MsLoginFinished {
  ok: boolean;
  error: string;
}

// ---------- 实例 ----------

export interface InstanceConfig {
  meta: {
    name: string;
    mc_version: string;
    loader: Loader;
    loader_version: string;
  };
  jvm: {
    min_memory: number;
    max_memory: number;
    extra_args: string[];
  };
  game: {
    resolution: { width: number; height: number };
    fullscreen: boolean;
  };
}

/** 创建/更新实例的入参(未传字段用后端默认值) */
export interface InstanceInput {
  name: string;
  mc_version?: string;
  loader?: Loader;
  loader_version?: string;
  min_memory?: number;
  max_memory?: number;
  width?: number;
  height?: number;
}

/** 实例详情:DB 记录 + TOML 配置 */
export type InstanceDetail = Instance & { config: InstanceConfig };

// ---------- 应用配置 ----------

export interface AppConfig {
  general: {
    data_dir: string;
    theme: string;
    language: string;
  };
  java: {
    auto_detect: boolean;
    default_java_path: string;
  };
  download: {
    max_concurrent: number;
    mirror: string;
    retry_times: number;
  };
}

// ---------- Mod ----------

export interface ModrinthHit {
  project_id: string;
  slug: string;
  title: string;
  description: string;
  categories: string[];
  downloads: number;
  icon_url: string | null;
  versions: string[];
}

export interface ModrinthFile {
  url: string;
  filename: string;
  primary: boolean;
  size: number;
  hashes: Record<string, string>;
}

export interface ModrinthVersion {
  id: string;
  project_id: string;
  name: string;
  version_number: string;
  game_versions: string[];
  loaders: string[];
  files: ModrinthFile[];
}

// ---------- Forge ----------

export interface ForgeVersionInfo {
  version: string;
  is_recommended: boolean;
  is_latest: boolean;
}
