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
