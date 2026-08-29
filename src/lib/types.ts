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
    mirror_custom_base: string | null;
    retry_times: number;
  };
  curseforge_api_key: string | null;
}

// ---------- 下载镜像 ----------

export interface MirrorSpec {
  id: string;
  name: string;
  base: string;
}

export interface Mirror {
  id: string;
  base: string;
}

export interface SpeedResult {
  id: string;
  name: string;
  base: string;
  latency_ms: number;
  throughput: number;
  ok: boolean;
  error: string;
}

// ---------- 崩溃诊断 ----------

export interface CrashDiagnosis {
  found: boolean;
  path: string;
  summary: string;
  suggestions: string[];
  matched: string[];
  raw_content: string;
  truncated: boolean;
}

// ---------- JVM 内存推荐 ----------

export interface SystemMemory {
  total_mb: number;
  available_mb: number;
}

export interface JvmRecommendation {
  min_mb: number;
  max_mb: number;
  extra_args: string[];
  tier_label: string;
  note: string;
}

// ---------- 整合包 ----------

export interface ModpackProgress {
  current: number;
  total: number;
  file: string;
}

export interface ModpackFinished {
  ok: boolean;
  error: string;
  installed: string[];
  failures: string[];
  name: string;
}

// ---------- 服务器 ----------

export interface ServerEntry {
  id: string;
  name: string;
  address: string;
  port: number;
  is_favorite: boolean;
  icon_base64: string | null;
  last_ping_ms: number | null;
  sort_order: number;
  created_at: number;
}

export interface ServerStatus {
  id: string;
  motd: string;
  players_online: number;
  players_max: number;
  latency_ms: number;
  favicon: string | null;
  ok: boolean;
}

/** 从 servers.dat 导入的服务器记录 */
export interface ImportedServer {
  name: string;
  address: string;
  port: number;
}

// ---------- 资源包 / 光影包 ----------

export interface ResourcePackEntry {
  id: string;
  instance_id: string;
  type_kind: "resourcepack" | "shaderpack";
  file_name: string;
  enabled: boolean;
  created_at: number;
}

/** 光影依赖检测结果 */
export interface ShaderSupportInfo {
  supported: boolean;
  message: string;
}

// ---------- 存档 / 截图 ----------

export interface SaveInfo {
  name: string;
  path: string;
  size_bytes: number;
  modified_at: number;
}

export interface BackupInfo {
  name: string;
  path: string;
  size_bytes: number;
  created_at: number;
}

export interface ScreenshotInfo {
  name: string;
  path: string;
  size_bytes: number;
  modified_at: number;
}

// ---------- 更新 ----------

export interface UpdateInfo {
  current: string;
  latest: string;
  has_update: boolean;
  notes: string;
}

// ---------- 皮肤 ----------

export type SkinModel = "classic" | "slim";

export interface SkinEntry {
  id: string;
  name: string;
  model: SkinModel;
  width: number;
  height: number;
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
  dependencies: ModrinthDependency[];
}

export interface ModrinthDependency {
  version_id: string | null;
  project_id: string | null;
  dependency_type: string;
  file_name: string | null;
}

export type ModSourceType = "modrinth" | "curseforge";

/** 统一的搜索条目(Modrinth / CurseForge 字段对齐,前端统一展示) */
export interface ModSearchResult {
  source: ModSourceType;
  project_id: string;
  slug: string;
  title: string;
  description: string;
  downloads: number;
  icon_url: string | null;
  categories: string[];
  versions: string[];
  /** 仅 CurseForge 来源有值:false 表示禁止第三方启动器分发 */
  allow_mod_distribution?: boolean;
}

export interface CurseForgeHit {
  project_id: string;
  slug: string;
  title: string;
  description: string;
  categories: string[];
  downloads: number;
  icon_url: string | null;
  versions: string[];
  allow_mod_distribution: boolean;
}

export interface CurseForgeFile {
  file_id: number;
  filename: string;
  url: string;
  size: number;
  sha1: string;
}

export interface DepCheckResult {
  missing_required: { project_id: string; version_id: string; file_name: string }[];
  conflicts: string[];
  ok: boolean;
}

// ---------- Forge ----------

export interface ForgeVersionInfo {
  version: string;
  is_recommended: boolean;
  is_latest: boolean;
}
