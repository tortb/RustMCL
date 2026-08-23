import { invoke } from "@tauri-apps/api/core";
import type {
  Account,
  AppInfo,
  Instance,
  InstanceDetail,
  InstanceInput,
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
