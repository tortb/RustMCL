import { invoke } from "@tauri-apps/api/core";
import type { Account, AppInfo, VersionFilter, VersionInfo } from "./types";

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
