import { invoke } from "@tauri-apps/api/core";
import type {
  Account,
  AppInfo,
  Instance,
  InstanceDetail,
  InstanceInput,
  ModrinthHit,
  ModrinthVersion,
  ModEntry,
  VersionFilter,
  VersionInfo,
} from "./types";

/**
 * Tauri command 统一封装。
 * 组件/页面禁止直接调用 invoke(),一律走本模块。
 */

export function getAppInfo(): Promise<AppInfo> {
  return invoke<AppInfo>("get_app_info");
}

export function dbHealth(): Promise<string[]> {
  return invoke<string[]>("db_health");
}

export function listAccounts(): Promise<Account[]> {
  return invoke<Account[]>("list_accounts");
}

export function getActiveAccount(): Promise<Account | null> {
  return invoke<Account | null>("get_active_account");
}

/** 启动微软 Device Code 登录流程,结果通过事件 ms-login-* 上报 */
export function startMicrosoftLogin(): Promise<void> {
  return invoke("start_microsoft_login");
}

export function cancelMicrosoftLogin(): Promise<void> {
  return invoke("cancel_microsoft_login");
}

export function logoutAccount(id: string): Promise<void> {
  return invoke("logout_account", { id });
}

// ---------- 实例 ----------

export function createInstance(input: InstanceInput): Promise<InstanceDetail> {
  return invoke<InstanceDetail>("create_instance", { input });
}

export function listInstances(): Promise<Instance[]> {
  return invoke<Instance[]>("list_instances");
}

export function getInstance(id: string): Promise<InstanceDetail | null> {
  return invoke<InstanceDetail | null>("get_instance", { id });
}

export function updateInstance(id: string, input: InstanceInput): Promise<InstanceDetail> {
  return invoke<InstanceDetail>("update_instance", { id, input });
}

export function deleteInstance(id: string): Promise<void> {
  return invoke("delete_instance", { id });
}

/** 按实例启动(自动补齐资源),日志/退出通过 game-log / game-exit 事件上报 */
export function launchInstance(id: string): Promise<void> {
  return invoke("launch_instance", { instanceId: id });
}

/** 查询指定 MC 版本可用的最新加载器版本(fabric/quilt) */
export function getLatestLoaderVersion(
  mcVersion: string,
  loader: string,
): Promise<string> {
  return invoke<string>("get_latest_loader_version", { mcVersion, loader });
}

/** 后台安装加载器,进度通过 download-progress 事件,结束通过 loader-install-finished */
export function installLoader(
  mcVersion: string,
  loader: string,
  loaderVersion: string,
): Promise<void> {
  return invoke("install_loader", { mcVersion, loader, loaderVersion });
}

// ---------- Mod ----------

/** 搜索 Modrinth 项目 */
export function searchMods(query: string, limit?: number): Promise<ModrinthHit[]> {
  return invoke<ModrinthHit[]>("search_mods", { query, limit });
}

/** 获取某项目与指定实例兼容的版本列表 */
export function getModVersions(projectId: string, instanceId: string): Promise<ModrinthVersion[]> {
  return invoke<ModrinthVersion[]>("get_mod_versions", { projectId, instanceId });
}

/** 安装 mod 到实例(幂等),进度通过 mod-install 事件 */
export function installMod(instanceId: string, versionId: string): Promise<ModEntry> {
  return invoke<ModEntry>("install_mod", { instanceId, versionId });
}

/** 列出实例已安装的 mod */
export function listInstanceMods(instanceId: string): Promise<ModEntry[]> {
  return invoke<ModEntry[]>("list_instance_mods", { instanceId });
}

/** 启用/禁用 mod */
export function setModEnabled(id: string, enabled: boolean): Promise<void> {
  return invoke("set_mod_enabled", { id, enabled });
}

/** 删除 mod(DB 记录 + 文件) */
export function deleteMod(id: string): Promise<void> {
  return invoke("delete_mod", { id });
}

export function listVersions(
  filter: VersionFilter = "all",
  forceRefresh = false,
): Promise<VersionInfo[]> {
  return invoke<VersionInfo[]>("list_versions", { filter, forceRefresh });
}

/** 后台下载指定版本资源(client + libraries + natives + assets) */
export function downloadVersion(mcVersion: string): Promise<void> {
  return invoke("download_version", { mcVersion });
}

/** 启动指定版本(离线账号),日志/退出通过事件上报 */
export function launchVersion(mcVersion: string, username?: string): Promise<void> {
  return invoke("launch_version", { mcVersion, username });
}
