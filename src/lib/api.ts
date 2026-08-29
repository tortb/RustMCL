import { invoke } from "@tauri-apps/api/core";
import type {
  Account,
  AppConfig,
  AppInfo,
  CrashDiagnosis,
  CurseForgeFile,
  CurseForgeHit,
  DepCheckResult,
  ForgeVersionInfo,
  Instance,
  InstanceDetail,
  InstanceInput,
  JvmRecommendation,
  Mirror,
  MirrorSpec,
  ModrinthHit,
  ModrinthVersion,
  ModEntry,
  ResourcePackEntry,
  SaveInfo,
  ScreenshotInfo,
  BackupInfo,
  ShaderSupportInfo,
  SpeedResult,
  SystemMemory,
  ImportedServer,
  ServerEntry,
  ServerStatus,
  SkinEntry,
  UpdateInfo,
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

/** 创建(或再次登录)离线账号,生成固定 UUID 并置为当前账号 */
export function createOfflineAccount(username: string): Promise<Account> {
  return invoke<Account>("create_offline_account", { username });
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

/** 检测安装某版本时的依赖缺失/冲突(建议性) */
export function checkModDependencies(instanceId: string, versionId: string): Promise<DepCheckResult> {
  return invoke<DepCheckResult>("check_mod_dependencies", { instanceId, versionId });
}

/** 搜索 CurseForge 项目 */
export function searchCurseforgeMods(
  query: string,
  mcVersion: string,
  loader: string,
  limit?: number,
): Promise<CurseForgeHit[]> {
  return invoke<CurseForgeHit[]>("search_curseforge_mods", { query, mcVersion, loader, limit });
}

/** 获取某 CurseForge mod 与当前实例兼容的文件列表 */
export function getCurseforgeFileVersions(projectId: string, instanceId: string): Promise<CurseForgeFile[]> {
  return invoke<CurseForgeFile[]>("get_curseforge_file_versions", { projectId, instanceId });
}

/** 下载并安装一个 CurseForge 文件到实例 */
export function installCurseforgeFile(
  instanceId: string,
  projectId: string,
  file: CurseForgeFile,
): Promise<ModEntry> {
  return invoke<ModEntry>("install_curseforge_file", { instanceId, projectId, file });
}

/** 列出指定 MC 版本可用的 Forge 版本 */
export function listForgeVersions(mcVersion: string): Promise<ForgeVersionInfo[]> {
  return invoke<ForgeVersionInfo[]>("list_forge_versions", { mcVersion });
}

/** 后台安装 Forge(下载 installer + 依赖 + 运行处理器),进度/结束事件与 install_loader 一致 */
export function installForge(mcVersion: string, forgeVersion: string): Promise<void> {
  return invoke("install_forge", { mcVersion, forgeVersion });
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

// ---------- 应用配置 ----------

export function getAppConfig(): Promise<AppConfig> {
  return invoke<AppConfig>("get_app_config");
}

export function updateAppConfig(config: AppConfig): Promise<AppConfig> {
  return invoke<AppConfig>("update_app_config", { config });
}

/** 检测系统 Java 版本,返回如 "21.0.5"(未安装返回 null) */
export function detectJava(): Promise<string | null> {
  return invoke<string | null>("detect_java");
}

// ---------- 下载镜像 ----------

/** 列出可用镜像源(官方 / BMCLAPI / MCBBS / 自定义) */
export function listMirrors(): Promise<MirrorSpec[]> {
  return invoke<MirrorSpec[]>("list_mirrors");
}

/** 测速单个镜像源 */
export function testMirrorSpeed(id: string, base: string): Promise<SpeedResult> {
  return invoke<SpeedResult>("test_mirror_speed", { id, base });
}

/** 并发测速全部候选镜像,按延迟升序返回 */
export function testAllMirrorSpeed(): Promise<SpeedResult[]> {
  return invoke<SpeedResult[]>("test_all_mirror_speed");
}

/** 切换镜像源(持久化到 config.toml 并更新运行态) */
export function setMirror(mirror: string, customBase?: string | null): Promise<Mirror> {
  return invoke<Mirror>("set_mirror", { mirror, customBase });
}

// ---------- 崩溃诊断 ----------

/** 分析指定实例(或共享游戏目录)最新的崩溃报告 */
export function analyzeCrashReport(instanceId?: string | null): Promise<CrashDiagnosis> {
  return invoke<CrashDiagnosis>("analyze_crash_report", { instanceId: instanceId ?? null });
}

/** 列出实例下全部崩溃报告路径 */
export function listCrashReports(instanceId?: string | null): Promise<string[]> {
  return invoke<string[]>("list_crash_reports", { instanceId: instanceId ?? null });
}

// ---------- JVM 内存推荐 ----------

/** 当前系统内存概况 */
export function getSystemMemory(): Promise<SystemMemory> {
  return invoke<SystemMemory>("get_system_memory");
}

/** 按系统内存 + 可选 mod 数量返回 JVM 推荐配置 */
export function recommendJvm(modCount?: number): Promise<JvmRecommendation> {
  return invoke<JvmRecommendation>("recommend_jvm", { modCount: modCount ?? null });
}

// ---------- 整合包 ----------

/** 导入整合包到实例(后台执行,进度/结果通过 modpack-progress / modpack-finished 事件) */
export function importModpack(filePath: string, instanceId: string): Promise<void> {
  return invoke("import_modpack", { filePath, instanceId });
}

/** 导出实例为 .mrpack 到指定路径 */
export function exportModpack(instanceId: string, destPath: string): Promise<void> {
  return invoke("export_modpack", { instanceId, destPath });
}

// ---------- 服务器 ----------

export function addServer(
  name: string,
  address: string,
  port: number,
  favorite?: boolean,
): Promise<ServerEntry> {
  return invoke<ServerEntry>("add_server", { name, address, port, favorite: favorite ?? false });
}

export function removeServer(id: string): Promise<void> {
  return invoke("remove_server", { id });
}

export function listServers(): Promise<ServerEntry[]> {
  return invoke<ServerEntry[]>("list_servers");
}

export function updateServer(
  id: string,
  name?: string,
  favorite?: boolean,
  sortOrder?: number,
): Promise<void> {
  return invoke("update_server", { id, name, favorite, sortOrder });
}

export function pingServer(id: string): Promise<ServerStatus> {
  return invoke<ServerStatus>("ping_server", { id });
}

/** 一键加入服务器:用指定实例启动并传入 --server/--port */
export function joinServer(serverId: string, instanceId: string): Promise<void> {
  return invoke("join_server", { serverId, instanceId });
}

/** 从 Minecraft 原生 servers.dat 批量导入服务器 */
export function importServers(datPath: string): Promise<ImportedServer[]> {
  return invoke<ImportedServer[]>("import_servers", { datPath });
}

// ---------- 资源包 / 光影包 ----------

/** 扫描实例下的资源包/光影包目录并同步 DB */
export function scanResourcePacks(instanceId: string): Promise<ResourcePackEntry[]> {
  return invoke<ResourcePackEntry[]>("scan_resource_packs", { instanceId });
}

export function setResourcePackEnabled(id: string, enabled: boolean): Promise<void> {
  return invoke("set_resource_pack_enabled", { id, enabled });
}

export function removeResourcePack(id: string): Promise<void> {
  return invoke("remove_resource_pack", { id });
}

/** 从 Modrinth 搜索资源包(type: resourcepack / shader) */
export function searchResourcePacks(query: string, packType: string): Promise<ModrinthHit[]> {
  return invoke<ModrinthHit[]>("search_resource_packs", { query, packType });
}

/** 检测实例是否已安装光影加载器(Iris/OptiFine) */
export function checkShaderSupport(instanceId: string): Promise<ShaderSupportInfo> {
  return invoke<ShaderSupportInfo>("check_shader_support", { instanceId });
}

// ---------- 存档 / 截图 ----------

export function listSaves(instanceId: string): Promise<SaveInfo[]> {
  return invoke<SaveInfo[]>("list_saves", { instanceId });
}

export function backupSave(instanceId: string, saveName: string): Promise<BackupInfo> {
  return invoke<BackupInfo>("backup_save", { instanceId, saveName });
}

export function listBackups(instanceId: string): Promise<BackupInfo[]> {
  return invoke<BackupInfo[]>("list_backups", { instanceId });
}

export function restoreBackup(
  instanceId: string,
  backupName: string,
  targetName: string,
): Promise<void> {
  return invoke("restore_backup", { instanceId, backupName, targetName });
}

export function deleteSave(instanceId: string, saveName: string): Promise<void> {
  return invoke("delete_save", { instanceId, saveName });
}

export function listScreenshots(instanceId: string): Promise<ScreenshotInfo[]> {
  return invoke<ScreenshotInfo[]>("list_screenshots", { instanceId });
}

export function deleteScreenshot(instanceId: string, name: string): Promise<void> {
  return invoke("delete_screenshot", { instanceId, name });
}

/** 读取截图图片的 data URL(供画廊按需懒加载) */
export function getScreenshotImage(instanceId: string, name: string): Promise<string | null> {
  return invoke<string | null>("get_screenshot_image", { instanceId, name });
}

// ---------- 更新 ----------

/** 检查更新(未配置更新源时返回提示) */
export function checkForUpdate(): Promise<UpdateInfo> {
  return invoke<UpdateInfo>("check_for_update");
}

/** 下载并安装最新更新,成功后应用重启 */
export function installUpdate(): Promise<string> {
  return invoke<string>("install_update");
}

// ---------- 皮肤 ----------

/** 列出本地皮肤库 */
export function listSkins(): Promise<SkinEntry[]> {
  return invoke<SkinEntry[]>("list_skins");
}

/** 导入本地皮肤(PNG,需 64x64/64x32) */
export function importSkin(srcPath: string, name: string, model: string): Promise<SkinEntry> {
  return invoke<SkinEntry>("import_skin", { srcPath, name, model });
}

/** 删除本地皮肤 */
export function removeSkin(id: string): Promise<void> {
  return invoke("remove_skin", { id });
}

/** 将本地皮肤上传到当前活跃微软账号 */
export function uploadSkin(id: string): Promise<void> {
  return invoke("upload_skin", { id });
}

/** 读取皮肤图片的 data URL(供 3D 预览加载) */
export function getSkinImage(id: string): Promise<string | null> {
  return invoke<string | null>("get_skin_image", { id });
}

/** 读取离线账号当前关联的皮肤 id */
export function getOfflineSkin(accountId: string): Promise<string | null> {
  return invoke<string | null>("get_offline_skin", { accountId });
}

/** 设置/清除离线账号的皮肤关联(skinId 传空则清除) */
export function setOfflineSkin(accountId: string, skinId: string | null): Promise<void> {
  return invoke("set_offline_skin", { accountId, skinId });
}
