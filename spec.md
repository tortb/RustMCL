# Runa 项目开发规格 (Project Spec)

> 本文档用于指导 AI 编程助手(Claude Code / Cursor 等)进行开发,包含项目定位、技术栈、架构约束、数据模型与开发阶段划分。

## 1. 项目定位

**名称**: Runa
**类型**: Minecraft: Java Edition 启动器
**目标**: 现代化、轻量级、跨平台(Windows / macOS / Linux)

**核心原则**:
- 启动速度快,内存占用低(相比 Electron 类启动器)
- UI 简洁现代,交互流畅
- 支持原版 + Forge/Fabric/Quilt 多加载器
- 支持微软账号登录(正版验证)

## 2. 技术栈

| 层 | 技术 |
|---|---|
| 应用框架 | Tauri 2.x |
| 后端语言 | Rust (stable) |
| 前端框架 | React 18+ (TypeScript) |
| 样式方案 | Tailwind CSS |
| 本地数据库 | SQLite (通过 `rusqlite` 或 `sqlx`) |
| 配置文件 | TOML (通过 `toml` + `serde`) |
| 异步运行时 | `tokio` |
| HTTP 客户端 | `reqwest` |
| 状态管理(前端) | zustand |

## 3. 目录结构

```
runa/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── commands/          # #[tauri::command] 暴露给前端的接口
│   │   │   ├── account.rs
│   │   │   ├── instance.rs
│   │   │   ├── launch.rs
│   │   │   ├── download.rs
│   │   │   └── mods.rs
│   │   ├── core/               # 纯业务逻辑,不依赖 Tauri
│   │   │   ├── account/
│   │   │   │   ├── microsoft_auth.rs
│   │   │   │   └── mod.rs
│   │   │   ├── version/
│   │   │   │   ├── manifest.rs
│   │   │   │   └── rules.rs    # OS/arch 条件规则解析
│   │   │   ├── launcher/
│   │   │   │   ├── args_builder.rs
│   │   │   │   └── process.rs
│   │   │   ├── downloader/
│   │   │   │   ├── asset.rs
│   │   │   │   └── library.rs
│   │   │   └── mods/
│   │   │       ├── forge.rs
│   │   │       ├── fabric.rs
│   │   │       └── quilt.rs
│   │   ├── db/
│   │   │   ├── schema.rs
│   │   │   ├── migrations/
│   │   │   └── repository.rs   # CRUD 封装
│   │   ├── config/
│   │   │   ├── app_config.rs   # 全局 TOML
│   │   │   └── instance_config.rs
│   │   └── error.rs            # 统一错误类型
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                         # React 前端
│   ├── pages/
│   ├── components/
│   ├── hooks/
│   ├── stores/                  # zustand stores
│   ├── lib/
│   │   ├── api.ts               # invoke() 统一封装 + 类型
│   │   └── types.ts             # 与 Rust struct 对应的 TS 类型
│   └── main.tsx
└── package.json
```

## 4. 数据模型

### 4.1 SQLite Schema

```sql
CREATE TABLE instances (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    mc_version TEXT NOT NULL,
    loader TEXT,                  -- vanilla | forge | fabric | quilt
    loader_version TEXT,
    game_dir TEXT NOT NULL,
    icon_path TEXT,
    created_at INTEGER NOT NULL,
    last_played INTEGER
);

CREATE TABLE accounts (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL,
    uuid TEXT NOT NULL,
    account_type TEXT NOT NULL,   -- microsoft | offline
    is_active INTEGER DEFAULT 0,
    refreshed_at INTEGER
);

CREATE TABLE mods (
    id TEXT PRIMARY KEY,
    instance_id TEXT NOT NULL REFERENCES instances(id) ON DELETE CASCADE,
    file_name TEXT NOT NULL,
    source TEXT,                  -- modrinth | curseforge | local
    project_id TEXT,
    version_id TEXT,
    enabled INTEGER DEFAULT 1
);

CREATE TABLE asset_cache (
    sha1 TEXT PRIMARY KEY,
    path TEXT NOT NULL,
    size INTEGER NOT NULL
);
```

### 4.2 TOML 配置

**应用级** `~/.runa/config.toml`:
```toml
[general]
data_dir = "~/.runa"
theme = "dark"
language = "zh-CN"

[java]
auto_detect = true
default_java_path = ""

[download]
max_concurrent = 8
mirror = "official"
retry_times = 3
```

**实例级** `~/.runa/instances/<id>/instance.toml`:
```toml
[meta]
name = "My Survival"
mc_version = "1.21.1"
loader = "fabric"
loader_version = "0.16.5"

[jvm]
min_memory = 2048
max_memory = 4096
extra_args = []

[game]
resolution = { width = 1280, height = 720 }
fullscreen = false
```

**数据存储原则**: TOML 存人类可读/可编辑的实例与应用设置;SQLite 存需要关联查询的结构化数据(账号列表、mod 关联、下载缓存索引)。两者不重复存同一字段。

## 5. 核心模块接口约定(Tauri Commands)

以下为前端可调用的核心命令,供 AI 生成代码时对齐签名:

```rust
// account.rs
#[tauri::command]
async fn start_microsoft_login() -> Result<(), String>;
#[tauri::command]
async fn get_active_account() -> Result<Option<Account>, String>;
#[tauri::command]
async fn list_accounts() -> Result<Vec<Account>, String>;

// instance.rs
#[tauri::command]
async fn create_instance(config: InstanceConfig) -> Result<String, String>; // 返回 instance id
#[tauri::command]
async fn list_instances() -> Result<Vec<Instance>, String>;
#[tauri::command]
async fn delete_instance(id: String) -> Result<(), String>;

// launch.rs
#[tauri::command]
async fn launch_instance(id: String) -> Result<(), String>;
// 通过 app.emit("game-log", line) 转发游戏日志
// 通过 app.emit("game-exit", code) 通知退出

// download.rs
#[tauri::command]
async fn download_version_assets(mc_version: String) -> Result<(), String>;
// 通过 app.emit("download-progress", { current, total, file }) 汇报进度
```

## 6. 开发阶段划分(建议按此顺序交给 AI 分批实现)

| 阶段 | 目标 | 关键产出 |
|---|---|---|
| M0 | 项目骨架 | Tauri + React + Tailwind 初始化,SQLite migration 跑通 |
| M1 | 版本清单 | 拉取 `version_manifest_v2.json`,解析并展示版本列表 |
| M2 | 离线启动 MVP | 下载原版 assets/libraries,拼装启动参数,离线账号启动游戏 |
| M3 | 微软登录 | OAuth Device Code Flow → Xbox Live → XSTS → MC Token |
| M4 | 实例管理 | 多实例 CRUD,TOML 读写,前端实例列表页 |
| M5 | Mod Loader | Fabric/Quilt 安装器(相对简单),再做 Forge(复杂) |
| M6 | Mod 浏览 | 接入 Modrinth API 搜索/安装 mod |
| M7 | 打磨 | 下载并发优化、错误处理、日志系统、自动更新 |

## 7. 已知技术难点(AI 编码时需重点关注)

1. **微软登录链路**: Device Code Flow → Xbox Live token → XSTS token → Minecraft token,每一步都有独立的错误码需要处理
2. **版本规则解析**: `version.json` 中 `rules` 数组基于 OS/arch 的条件判断(natives 下载、JVM 参数是否生效)
3. **Java 版本匹配**: 不同 MC 版本要求不同 Java 主版本(如 1.20.5+ 需要 Java 21)
4. **并发下载与断点续传**: 需要限速、失败重试、SHA1 校验
5. **子进程管理**: 游戏进程的 stdout/stderr 实时转发到前端,以及异常退出检测

## 8. 给 AI 助手的编码约束

- Rust 端错误统一使用自定义 `RunaError` 枚举 + `thiserror`,不要到处 `unwrap()`
- 所有网络请求需要超时与重试机制
- Tauri command 与 core 业务逻辑分离,core 层不依赖 `tauri::AppHandle`(便于单元测试),需要发事件时通过参数传入 handle 或用 channel
- 前端 API 调用统一走 `src/lib/api.ts`,禁止在组件里直接写 `invoke(...)`
- TypeScript 类型与 Rust struct 保持同步(可考虑 `ts-rs` 自动生成)
# Runa 开发任务拆分(AI 可执行版)

> 配合 `runa-project-spec.md` 使用。每个任务粒度控制在"AI 一次会话可完成"的范围,包含明确输入/输出/验收标准,可直接作为 Claude Code 的任务描述。

---

## 前端设计方向:Apple 风格设计令牌

在拆任务前先定design token,所有前端任务都要遵循这套规范,保证风格统一。

### 视觉语言参考
- **参考对象**: macOS Sonoma/Sequoia 系统设置、iOS 17/18 原生 App、SF Symbols 的克制感
- **核心特征**: 毛玻璃(frosted glass / vibrancy)、大圆角、克制的阴影层次、内容优先于装饰、Spring 动画曲线(而非线性 ease)

### 设计令牌

```
颜色(浅色模式基准,深色模式做语义映射):
- 背景层级: #FFFFFF(卡片) / #F5F5F7(页面底色,苹果官网同款) / rgba(255,255,255,0.72)+blur(毛玻璃面板)
- 主强调色: #0A84FF(iOS 系统蓝) 或根据 Runa 品牌色替换,建议保留一个可自定义主题色的机制
- 文字: #1D1D1F(主文字,苹果标准深灰黑) / #86868B(次要文字) / #6E6E73(说明文字)
- 分割线: rgba(0,0,0,0.08),不用纯黑

圆角:
- 卡片: 16-20px
- 按钮: 10-12px(不用 full-rounded,苹果很少用胶囊按钮除非是标签/开关)
- 输入框: 8-10px

阴影(极克制,只在需要"悬浮"语义时用):
- 卡片静置: 0 1px 3px rgba(0,0,0,0.06)
- 卡片 hover/active: 0 8px 24px rgba(0,0,0,0.12)
- 不用夸张的彩色发光阴影

字体:
- 系统字体栈优先: -apple-system, "SF Pro Display", "PingFang SC", sans-serif
- 标题用偏重字重(600-700),正文常规(400),说明文字次要色 + 400

动效(这是本项目的重点,苹果感主要靠这个体现):
- 缓动函数: cubic-bezier(0.32, 0.72, 0, 1) —— 苹果常用的"快出慢入"曲线,替代默认 ease
- 页面切换: 用位移 + 透明度组合(translateY 8px → 0 + opacity 0→1),时长 250-350ms
- 列表项进入: 轻微 stagger(逐项延迟 20-40ms),避免所有元素同时跳出
- 弹窗/Modal: 缩放 + 透明度(scale 0.95→1 + opacity),配合背景毛玻璃遮罩
- 按钮反馈: active 状态 scale(0.97),时长 100ms,模拟物理按压
- 侧边栏/详情页展开: 用 Framer Motion 的 layout animation 做流畅的元素位置过渡
- 涉及技术: 前端用 `framer-motion`(React 生态里做 Apple 风格动效最顺手),配合 Tailwind 做静态样式
```

### 前端任务通用要求(所有 UI 任务都要带上这段 prompt)

```
设计参考 macOS/iOS 原生应用的视觉语言:大圆角卡片、毛玻璃背景、克制的阴影、
系统字体。动画用 framer-motion,缓动曲线用 cubic-bezier(0.32, 0.72, 0, 1),
页面/列表进入要有轻微的位移+淡入,不要用生硬的线性动画或者默认的 ease-in-out。
颜色以 #F5F5F7 页面底色 + 白色卡片 + #0A84FF 强调色为基准,文字用 #1D1D1F/#86868B
两级灰度。避免看起来像 Bootstrap/Material 默认组件库风格。
```

---

## 任务列表

### M0. 项目骨架

**T0.1 初始化 Tauri + React + Tailwind 项目**
- 输入: 无
- 产出: `npm create tauri-app` 脚手架,接入 Tailwind CSS,配置 `framer-motion` 依赖
- 验收: `tauri dev` 能跑起一个空白窗口

**T0.2 SQLite 接入与 migration 系统**
- 输入: 第 4.1 节 schema
- 产出: `src-tauri/src/db/` 模块,使用 `rusqlite` 或 `sqlx`,启动时自动跑 migration
- 验收: 首次启动在 `~/.runa/runa.db` 生成建好表的数据库文件

**T0.3 TOML 配置读写模块**
- 输入: 第 4.2 节配置示例
- 产出: `config/app_config.rs`,支持默认值 + 反序列化容错(字段缺失不崩溃)
- 验收: 首次启动生成默认 `config.toml`,能读写往返一致

---

### M1. 版本清单

**T1.1 拉取并解析 Mojang 版本清单**
- 输入: `https://piston-meta.mojang.com/mc/game/version_manifest_v2.json`
- 产出: `core/version/manifest.rs`,拉取并缓存清单,提供 `list_versions(filter: release|snapshot|all)` 
- 验收: 单元测试能解析出至少最近 5 个正式版

**T1.2 前端版本列表页(应用设计令牌)**
- 输入: T1.1 的 Tauri command
- 产出: `pages/VersionPicker.tsx`,列表带 stagger 进入动画、搜索过滤
- 验收: 视觉与动效符合"前端任务通用要求"

---

### M2. 离线启动 MVP

**T2.1 资源下载器(assets + libraries)**
- 输入: 单个版本的 `version.json`
- 产出: `core/downloader/`,支持并发下载(可配置并发数)、SHA1 校验、失败重试
- 验收: 能完整下载一个版本所需全部文件且校验通过

**T2.2 启动参数拼装器**
- 输入: `version.json` 的 rules 规则
- 产出: `core/launcher/args_builder.rs`,正确处理 OS/arch 条件规则
- 验收: 生成的 JVM + 游戏参数能在本机跑通一次离线启动

**T2.3 进程管理与日志转发**
- 输入: T2.2 的启动参数
- 产出: `core/launcher/process.rs`,子进程 stdout/stderr 通过 `emit` 转发到前端
- 验收: 前端能实时看到游戏日志滚动,退出码正确捕获

**T2.4 下载进度 + 启动界面(应用设计令牌)**
- 输入: T2.1/T2.3 的事件
- 产出: 进度条组件(毛玻璃卡片 + 平滑进度动画),启动中状态的加载动效
- 验收: 视觉与动效符合通用要求,进度数字变化有平滑过渡而非跳变

---

### M3. 微软登录

**T3.1 OAuth Device Code Flow**
- 产出: `core/account/microsoft_auth.rs`,实现 MSA → Xbox Live → XSTS → MC Token 全链路
- 验收: 能拿到有效的 Minecraft access token 并成功请求 profile 接口

**T3.2 Token 安全存储**
- 产出: 用 `keyring` crate 存 refresh token,DB 只存账号元数据
- 验收: 重启应用后能用 refresh token 静默续期

**T3.3 登录界面(应用设计令牌)**
- 产出: 登录弹窗(毛玻璃遮罩 + 缩放淡入),Device Code 展示与轮询状态动画
- 验收: 符合通用视觉要求

---

### M4. 实例管理

**T4.1 实例 CRUD(Rust + SQLite + TOML 双写)**
- 产出: `commands/instance.rs` 全套增删改查
- 验收: 创建实例后 DB 记录与 `instance.toml` 均正确生成

**T4.2 实例列表/详情页(应用设计令牌)**
- 产出: 卡片式实例列表,详情页用 `framer-motion` layout animation 做展开过渡
- 验收: 卡片 hover/点击有物理反馈感,符合通用视觉要求

---

### M5-M7

后续 Mod Loader、Modrinth 集成、打磨阶段任务待 M0-M4 完成后再细化(依赖前面阶段的实际接口稳定后拆分更准确)。

---

## 给 AI 编码助手的执行建议

- 按 M0 → M1 → M2 → M3 → M4 顺序逐个任务提交,不要跳阶段并行,因为后面阶段依赖前面的接口稳定
- 每个任务开始前把对应的 "输入" 一并贴给 AI,减少上下文缺失导致的返工
- 涉及前端的任务,务必附上"前端任务通用要求"那段 prompt,否则容易做成默认的 Tailwind 组件库风格
