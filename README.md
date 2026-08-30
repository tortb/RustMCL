# RustMCL

> 一款现代化、轻量级、跨平台的 Minecraft: Java Edition 启动器。

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
![Languages](https://img.shields.io/badge/Rust-Tauri%20%2B%20React-blue)

> **English:** Looking for an English overview? Jump to the [project intro section](#english-overview).

---

**RustMCL**(简称 **rmcl**,曾用名 **Runa**)是一款使用 **Rust + Tauri** 构建的 Minecraft: Java Edition 启动器。它提供原版与社区加载器(Forge/Fabric/Quilt)的实例管理、微软与离线两种账号登录方式,以及 Mod、整合包、服务器等资源的统一管理,同时通过引入 Tauri 大幅降低了同类 Electron 启动器的资源占用。

完全开源,遵循 [GPL-3.0](LICENSE)。仓库地址:<https://github.com/tortb/RustMCL.git>。

---

## 项目截图 / 预览

TODO: 补充实际截图(请在本仓库 `docs/screenshots/` 或 README 同级图片目录添加界面上图,并在此处引用)。

---

## 核心特性

### 游戏与加载器

- **跨 Minecraft 版本**:支持从官方元数据拉取正式版 / 快照版本清单。
- **多加载器支持**:原版、**Fabric / Quilt**(通过 meta API 拉取 profile 合并 version.json)、**Forge**(版本清单 / installer 解压 / processors 执行引擎 / 客户端客户端处理器过滤)。
- **实例管理**:实例的创建、编辑、删除与多实例列表管理;启动实例时自动补齐缺失资源(幂等)。
- **下载器**:并发多线程、逐文件 SHA1 校验、失败重试、`.part` 临时文件原子改名;已存在且校验通过的文件自动跳过缓存,不重复下载。

### 账号

- **微软账号登录**:基于 OAuth 2.0 **设备代码流程(Device Code Flow)**,随后按标准链路完成 Xbox Live → XSTS → Minecraft Services 令牌交换。刷新令牌仅保存在本机系统凭据管理器(keyring)中。
- **离线账号登录**:本地校验用户名,生成稳定 UUID,可离线进入游戏。

### Mod / 整合包 / 资源

- **Mod 管理**:**Modrinth** 集成(搜索、版本解析、安装到实例、依赖检查),支持启用 / 禁用 / 删除;另支持 **CurseForge** 搜索与安装(需在设置页配置 API Key)。
- **整合包**:**mrpack** 导入与导出,支持 CurseForge 整合包解析(需 API Key);导入时校验 MC / 加载器兼容性,并带路径穿越(Zip-slip)防护。
- **资源包 / 光影包**:本地扫描、启用 / 禁用(重命名 `.disabled`)、删除与光影依赖检测(Iris / OptiFine)。

### 服务器

- **服务器列表**:添加、删除、收藏、拖拽排序、一键加入已启动实例,支持从经典版 `servers.dat` 导入。
- **延迟测速**:内置独立实现的 Minecraft 服务器列表协议(Handshake → Status)解析器,周期 ping 显示延迟与 MOTD。

### 系统集成

- **Java 环境**:自动检测本机 Java,并按系统内存档位 / Mod 数量**推荐 JVM 参数**。
- **皮肤管理**:本地皮肤库导入与校验,上传微软账号皮肤,离线账号皮肤本地关联,并提供 3D 预览。
- **崩溃日志分析**:规则库匹配(内存不足 / Java 版本不匹配 / mod 冲突 / 显卡驱动等),定位相关 mod 并给出修复建议。
- **存档与截图**:存档列出、备份 zip、恢复、删除;截图列出、删除、预览(均带路径防护)。
- **镜像源**:官方 / BMCLAPI / MCBBS / 自定义下载源切换,并可自动测速。
- **设置页**:下载源测速、Java 路径、CurseForge Key、主题与语言等持久化到 `config.toml`。

### 界面与性能

- **Apple 风格 UI**:毛玻璃、流畅动画、统一的设计令牌与组件。
- **轻量级**:基于 Tauri,较 Electron 类启动器资源占用更低。

---

## 安装 / 下载

目前**尚未发布正式 Release**,请直接**从源码构建**。

---

## 从源码构建

### 环境要求

| 依赖 | 版本要求 | 说明 |
|---|---|---|
| [Rust](https://www.rust-lang.org/) | stable(建议 1.75+) | 后端与 Tauri 壳 |
| [Node.js](https://nodejs.org/) | 18+ | 前端(Vite + React) |
| [Tauri CLI](https://tauri.app/) | 随 `npm` 安装即可 | 由 `@tauri-apps/cli` 提供 |
| 系统依赖 | — | Linux 需 `webkit2gtk`、`gtk` 等,见 [Tauri 官方 Prerequisites](https://tauri.app/start/prerequisites/) |

### 步骤

```bash
# 1. 克隆仓库
git clone https://github.com/tortb/RustMCL.git
cd RustMCL

# 2. 安装前端依赖
npm install

# 3. 开发模式运行(启动 Vite dev 服务 + 编译并运行 Tauri 应用)
npm run tauri dev

# 4. 构建生产版本
npm run tauri build
```

> 前端 `npm run build` 会先执行 `tsc && vite build` 产出 `dist/`,再由 Tauri 打包。

---

## 隐私与安全

本项目使用 **OAuth 2.0 设备代码流程(Device Code Flow)** 完成微软账号登录,随后按标准流程完成 **Xbox Live → XSTS → Minecraft Services** 的令牌交换,仅用于获取本机启动游戏所需的有效会话。

本项目**不会存储**用户的微软账号密码;仅在用户本机(通过操作系统凭据管理器)保存刷新令牌以维持登录状态,该令牌不会被传输至任何第三方服务器。启动器不会收集、记录或向开发者及任何外部方传输用户的认证数据。

所有认证相关代码均在本仓库中公开,欢迎审查。

---

## 贡献 (Contributing)

感谢你对 RustMCL 的兴趣!

- **Bug 报告 / 功能请求**:请先搜索 [Issues](https://github.com/tortb/RustMCL/issues) 是否已有相关讨论;若无,再新建 Issue,并附上复现步骤、环境信息(OS、Rust/Node 版本)与必要日志。
- **Pull Request**:请基于 `main` 分支的新分支工作,保持提交信息清晰(建议沿用 `type(scope): subject` 规范,如 `feat(mods): ...`、`fix(account): ...`);PR 描述请说明改动动机与验证情况。
- **Coding**:匹配现有代码风格;涉及 UI 改动请保持 Apple 风格设计令牌与动效约定。

---

## License

本项目基于 [GNU General Public License v3.0](LICENSE) 开源。

---

## 致谢

RustMCL 的微软账号认证与资源流程参考了社区内诸多开源启动器(HMCL、PrismLauncher 等)的既有实现思路,并借鉴了 [Minecraft Wiki / wiki.vg](https://wiki.vg/index.php?title=Microsoft_Authentication_Scheme) 的认证协议说明。在此一并致谢。

---

## English Overview

<a id="english-overview"></a>

**RustMCL** is a modern, lightweight, cross-platform Minecraft: Java Edition launcher built with **Rust + Tauri** (frontend: **React + Tailwind CSS**).

Key features:

- **Game & loaders**: vanilla, Fabric / Quilt / Forge; instance management (create / edit / delete / launch).
- **Accounts**: Microsoft (Device Code Flow, OAuth 2.0) and offline accounts.
- **Mods**: Modrinth (and CurseForge, API key required) search & install.
- **Servers**: list, favorites, ping, join slot, servers.dat import.
- **System**: Java detection & JVM tuning, skin management, crash log analysis, saves/screenshots, mirrors, theme & language.

See the Chinese sections above for the full detail. Licensed under **GPL-3.0**.

---

## Roadmap

以下为计划中的功能,尚未实现或仍在完善中:

- **资源包 / 光影包在线搜索并安装**(当前仅支持本地扫描与管理)。
- **自更新真实生效**(更新端点为`example.com`占位,需接入真实发布源)。
- **多实例并行运行**(当前前端一次仅允许一个实例在跑)。
- **主题 / 语言切换真正生效**(当前已持久化,运行时应用情况待确认)。
- **离线账号皮肤的游戏内渲染**(当前为本地关联 + 预览)。
- 服务器 `servers.dat` 之外的更多导入导出形态。
