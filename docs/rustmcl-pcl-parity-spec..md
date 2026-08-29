# RustMCL —— 对齐 PCL2/HMCL 体验 功能 Spec 合集

> 覆盖 11 个功能模块。每个模块独立成一节,包含:背景/目标 → 任务拆分(输入/产出/验收)→ Review Checklist → 提交节点。
> **执行原则**: 每个模块作为一个独立的开发阶段,完成后本地跑完 `cargo check` + `cargo test` + `tsc`,过 Review Checklist 后再 commit + push,不要多个模块混在一次提交里。
> 修正说明:此前提到的"外置登录(authlib-injector)"经实际核实 PCL/HMCL 并未提供该功能,故从计划中移除。

---

## 模块 1:服务器列表管理

**目标**: 添加/收藏服务器、显示 ping 延迟、一键加入,对齐原版"多人游戏"界面的启动器侧增强。

### 任务拆分
- **1.1 数据模型**: SQLite 新增 `servers` 表(`id, name, address, port, is_favorite, icon_base64, last_ping_ms, sort_order`)
- **1.2 Server Ping 协议实现**: 实现 Minecraft Server List Ping 协议(Handshake → Status Request → Status Response),Rust 端用 `tokio::net::TcpStream` 手写协议帧,不依赖游戏本体
- **1.3 Tauri Commands**: `add_server` / `remove_server` / `list_servers` / `ping_server(id) -> ServerStatus{motd, players_online, players_max, latency_ms, favicon}`
- **1.4 一键加入**: 启动参数追加 `--server {address} --port {port}`,复用已有的 `launch_instance` 逻辑
- **1.5 前端界面**: 服务器卡片列表(图标+MOTD+延迟颜色分级:绿/黄/红),支持拖拽排序、收藏星标,动效遵循 Apple 设计令牌

### Review Checklist
- [ ] ping 请求有超时机制(建议 3-5s),不阻塞 UI
- [ ] 离线/不可达服务器有明确的错误态展示,不是无限转圈
- [ ] favicon(base64 PNG)解析失败时有默认占位图
- [ ] 服务器列表支持批量导入(读取 Minecraft 本身的 `servers.dat` 做迁移,提升老用户迁移体验)
- [ ] 单元测试覆盖协议帧编解码

### 提交节点
`feat(servers): 服务器列表管理(添加/收藏/延迟/一键加入)` → 本地全量测试通过后 push

---

## 模块 2:CurseForge 集成

**目标**: 补齐 Modrinth 之外的第二大 mod 源。

### 任务拆分
- **2.1 API 接入**: CurseForge API 需要 API Key(需自行申请,注意不要硬编码进仓库,走环境变量/配置文件)
- **2.2 搜索与详情**: `search_curseforge_mods(query, mc_version, loader) -> Vec<ModSearchResult>`,复用模块 6(mod依赖冲突检测)会用到的元数据结构,建议统一 `ModSource` trait 抽象 Modrinth/CurseForge 差异
- **2.3 安装流程**: 下载 mod jar + 写入 `mods` 表(`source = 'curseforge'`)
- **2.4 前端**: Mod 浏览页增加来源筛选 tab(Modrinth / CurseForge / 全部)

### Review Checklist
- [ ] API Key 走配置注入,`.gitignore` 确认不会误提交密钥
- [ ] CurseForge 部分 mod 禁止第三方启动器分发(`allowModDistribution: false`),需要检测该字段并做降级处理(引导用户手动下载而非直接拉取)
- [ ] 两个 mod 源的搜索结果在 UI 上视觉统一,不因数据结构差异导致卡片错位
- [ ] 速率限制处理(CurseForge API 有请求频率限制,需要节流/缓存)

### 提交节点
`feat(mods): CurseForge 集成(搜索/安装/来源筛选)` → push

---

## 模块 3:整合包导入/导出

**目标**: 降低用户从其他启动器迁移的门槛,这是决定"能不能吸引老玩家"的关键功能。

### 任务拆分
- **3.1 Modrinth `.mrpack` 导入**: 解析 `modrinth.index.json`,批量下载依赖 mod、复制 overrides 文件夹
- **3.2 CurseForge 整合包导入**: 解析 `manifest.json` + `overrides/`,结构与 mrpack 类似但字段不同,建议抽象统一的 `ModpackManifest` 中间结构
- **3.3 导出功能**: 将现有实例打包为 `.mrpack`(反向操作,收集已装 mod 的来源信息生成 index)
- **3.4 前端**: "导入整合包"入口(拖拽 zip/mrpack 文件到窗口 或 文件选择器),导入进度展示(复用模块 8 下载器的进度事件机制)

### Review Checklist
- [ ] 导入时校验 loader/mc_version 兼容性,不匹配时提前提示而非装到一半失败
- [ ] overrides 文件夹里的配置文件(config/、resourcepacks/ 等)完整覆盖,不遗漏
- [ ] 导入失败(某个 mod 在源站下架)时,清单式展示"哪些 mod 装失败了",允许用户手动补
- [ ] 导出的 mrpack 能在 Modrinth 官方启动器或其他兼容启动器打开验证互通性

### 提交节点
`feat(modpack): 整合包导入导出(mrpack + CurseForge)` → push

---

## 模块 4:资源包/光影包管理

**目标**: 与 mod 管理逻辑高度复用,补齐 `resourcepacks/` 和 `shaderpacks/` 目录管理。

### 任务拆分
- **4.1 数据模型**: 复用 `mods` 表结构思路,新增 `resource_packs` 表(区分 type: resourcepack / shaderpack)
- **4.2 本地扫描**: 启动实例前扫描对应目录,同步文件系统实际状态到 DB(避免用户手动拖文件导致数据不一致)
- **4.3 启用/禁用**: 通过重命名(加 `.disabled` 后缀)或维护 `options.txt` / `optionsshaders.txt` 关联字段实现
- **4.4 Modrinth 资源包/光影搜索**: 复用模块 2/已有 Modrinth 集成的搜索能力,只是过滤 project_type
- **4.5 前端**: 类似 mod 管理页的卡片列表 + 预览图(资源包/光影包在 Modrinth 上通常带缩略图)

### Review Checklist
- [ ] 光影包依赖 OptiFine/Iris,需要检测当前实例是否装了对应 mod/loader 支持,没装时给出明确提示而非静默失败
- [ ] 文件系统扫描与 DB 状态双向同步逻辑要有测试(用户手动删文件后重启启动器,列表要正确刷新)
- [ ] 大文件(高清材质包可能上百 MB)下载要走已有的并发下载器 + 进度展示

### 提交节点
`feat(resourcepacks): 资源包与光影包管理` → push

---

## 模块 5:Mod 依赖冲突检测

**目标**: 安装 mod 时提前发现依赖缺失/版本冲突,这是 PCL/HMCL 用户体验里"显得聪明"的关键点。

### 任务拆分
- **5.1 依赖元数据获取**: Modrinth/CurseForge 的 mod 版本详情里通常带 `dependencies` 字段(required/optional/incompatible),需要在安装时一并拉取
- **5.2 依赖图构建**: 内存中构建当前实例已装 mod 的依赖图,新增 mod 时做冲突检测(版本区间不重叠、incompatible 声明冲突)
- **5.3 自动补全依赖**: 检测到 required 依赖缺失时,提示"是否自动安装依赖 mod X"
- **5.4 前端**: 安装冲突时用非阻断式提示(banner 而非强制弹窗),列出具体冲突的 mod 对

### Review Checklist
- [ ] 依赖版本号解析要处理好 semver 之外的非标准版本号格式(mod 生态版本号很不规范,建议做宽松匹配 + 兜底不报错策略,避免误报阻塞用户正常安装)
- [ ] 循环依赖场景不会导致自动安装死循环
- [ ] 冲突检测是"建议性"而非"强制阻断",用户仍可选择忽略提示继续安装(mod 生态里误报难以完全避免)

### 提交节点
`feat(mods): mod 依赖冲突检测与自动补全` → push

---

## 模块 6:崩溃日志分析

**目标**: 游戏崩溃后自动分析 crash report,给出中文可读的诊断建议,而不是甩 stacktrace。

### 任务拆分
- **6.1 Crash Report 定位**: 游戏进程异常退出时,自动定位最新的 `crash-reports/crash-*.txt`
- **6.2 规则引擎**: 维护一份"常见崩溃特征 → 诊断建议"的规则库(可以先用简单的关键字/正则匹配,后续再考虑更复杂的模式):
  - `OutOfMemoryError` → 建议调大 Xmx
  - Java 版本不匹配报错特征 → 提示应使用的 Java 版本
  - 特定 mod 的报错堆栈(`at net.minecraftforge...` 附近的 mod id)→ 定位到具体是哪个 mod 引发
  - 显卡驱动相关(`OpenGL`, `GLFW` 报错)→ 提示更新显卡驱动
- **6.3 Tauri Command**: `analyze_crash_report(instance_id) -> CrashDiagnosis{summary, suggestions: Vec<String>, raw_log_path}`
- **6.4 前端**: 崩溃后弹出诊断卡片(而非直接甩用户去看 crash-reports 文件夹),附"复制完整日志"和"打开日志文件夹"按钮

### Review Checklist
- [ ] 规则库要设计成易扩展的数据结构(比如 TOML/JSON 规则文件而非硬编码在 Rust match 里),方便后续持续补充新的崩溃特征
- [ ] 匹配不到已知规则时,展示"未识别的崩溃类型" + 原始日志入口,而不是假装分析出了原因
- [ ] 诊断建议措辞遵循"解释发生了什么 + 具体可执行的下一步",不是模糊的"请检查你的配置"

### 提交节点
`feat(diagnostics): 崩溃日志分析与诊断建议` → push

---

## 模块 7:JVM 参数自动调优

**目标**: 根据系统内存自动推荐 Xmx/Xms,减少用户手动填写门槛。

### 任务拆分
- **7.1 系统信息采集**: 用 `sysinfo` crate 获取总内存/可用内存
- **7.2 推荐算法**: 简单规则起步(如:总内存 8G 建议 Xmx=4G,16G 建议 Xmx=6-8G,并预留系统余量),后续可结合 mod 数量(mod 越多建议内存越大)细化
- **7.3 GC 参数建议**: 针对不同内存档位给出合适的 GC 参数组合(比如小内存用默认 GC,大内存推荐 G1GC 参数,可以参考社区常见的"G1GC 优化参数"组合)
- **7.4 前端**: 实例设置页的内存滑块旁边显示"推荐值"标记,一键应用推荐配置,同时保留用户手动覆盖的自由度

### Review Checklist
- [ ] 推荐值不超过系统可用内存的合理比例(避免推荐过高导致系统卡死)
- [ ] 用户手动改过的配置,不会被"自动调优"逻辑静默覆盖(自动推荐只在用户主动点击"应用"时生效)
- [ ] 32 位系统/低内存设备(<4G)有单独的保守推荐档位

### 提交节点
`feat(jvm): JVM 参数自动调优建议` → push

---

## 模块 8:下载镜像切换 + 测速

**目标**: 接入国内镜像源(如 BMCLAPI),自动测速选择最快节点,这对国内用户体验影响很大。

### 任务拆分
- **8.1 镜像源配置**: 扩展此前 `config.toml` 里预留的 `mirror` 字段,支持多个镜像源(官方源 + BMCLAPI 等)配置列表
- **8.2 URL 替换层**: 在下载器(`core/downloader/`)里增加一层 URL 映射,官方域名 → 镜像域名的替换规则(注意:不同类型资源—— version manifest / assets / libraries / forge installer —— 的镜像映射规则可能不同,需要分别处理)
- **8.3 测速机制**: 应用启动或用户手动触发时,对候选镜像做小文件下载测速(记录延迟+吞吐),自动选出最优
- **8.4 前端**: 设置页展示各镜像源的实时测速结果(延迟条形图),支持手动锁定某个源

### Review Checklist
- [ ] 测速请求要轻量(小文件),不要每次启动都拉大文件浪费带宽
- [ ] 镜像源不可用时要有自动降级到官方源的兜底逻辑,不能卡死下载流程
- [ ] URL 映射规则要写测试覆盖,避免镜像规则出错导致下载到错误资源(校验环节的 SHA1 检查是最后一道防线,但映射错误应该更早发现)

### 提交节点
`feat(download): 镜像源切换与自动测速` → push

---

## 模块 9:皮肤管理

**目标**: 皮肤上传/切换 + 预览,微软账号场景下调用 Mojang API 完成变更。

### 任务拆分
- **9.1 皮肤上传**: 调用 Minecraft Services API 的皮肤上传接口(需要有效的账号 token),支持 classic/slim 两种模型
- **9.2 本地皮肤库**: 允许用户保存多套皮肤文件(PNG)在本地,快速切换而不用每次重新上传相同文件
- **9.3 3D 预览**: 前端用现成的 skin viewer 方案(如 `skinview3d` 这类基于 three.js 的库)渲染 3D 人物模型预览
- **9.4 离线账号皮肤**: 离线账号没有 Mojang 账号系统支撑,需要通过自定义皮肤渲染方案(游戏本体读取本地皮肤文件的机制因版本而异,需要调研)

### Review Checklist
- [ ] 皮肤上传接口的 token 权限与账号系统(模块外)复用已有的 keyring token 存储,不要另起一套
- [ ] 3D 预览组件的性能开销要控制,避免拖慢整体应用启动/页面切换的流畅度(和 Apple 风格的动效要求不冲突)
- [ ] 皮肤文件格式校验(64x64/64x32 PNG),上传前给出格式错误提示而非等 API 报错

### 提交节点
`feat(skin): 皮肤管理与 3D 预览` → push

---

## 模块 10:自更新机制

**目标**: 用 Tauri 自带 updater 插件,减少后续分发维护成本。

### 任务拆分
- **10.1 接入 `tauri-plugin-updater`**: 配置更新清单(update manifest)托管方式(GitHub Releases 是最简单的选择)
- **10.2 签名配置**: Tauri updater 要求更新包签名,需要生成密钥对并妥善保管私钥(私钥绝不能进仓库)
- **10.3 检查更新时机**: 启动时静默检查 + 设置页手动检查按钮
- **10.4 前端**: 发现新版本时的提示卡片(版本号+更新日志摘要),下载进度展示,更新完成后引导重启

### Review Checklist
- [ ] 更新失败(网络问题/签名校验失败)有清晰的错误提示和重试机制,不能让用户以为软件坏了
- [ ] CI 流程里更新包的构建与签名要自动化(GitHub Actions),避免手动操作出错
- [ ] 更新过程不影响用户正在进行的游戏会话(如果游戏正在运行,更新应延迟到下次启动)

### 提交节点
`feat(updater): 自更新机制接入` → push

---

## 模块 11:存档备份 + 截图管理

**目标**: 世界存档的备份/恢复,以及游戏截图的浏览管理。

### 任务拆分
- **11.1 存档管理**: 扫描实例 `saves/` 目录,列出世界(名称、图标、最后游玩时间、大小),支持打开文件夹/删除/重命名
- **11.2 存档备份**: 一键打包为 zip(可选自动定期备份,比如每次游戏正常退出后触发),备份历史列表 + 恢复功能
- **11.3 截图管理**: 扫描实例 `screenshots/` 目录,做成图片画廊(网格布局,懒加载缩略图),支持删除/导出/复制到剪贴板
- **11.4 前端**: 存档卡片(带世界图标预览)+ 截图画廊,两者可以放在实例详情页的不同 tab 下

### Review Checklist
- [ ] 大存档(几百 MB 到几 GB)打包备份不能阻塞 UI 线程,要走异步 + 进度展示
- [ ] 自动备份策略要有存储上限控制(比如只保留最近 N 份),避免无限占用磁盘空间
- [ ] 截图画廊在截图数量很多(几百张)时要做虚拟滚动/分页,避免卡顿

### 提交节点
`feat(saves): 存档备份与截图管理` → push

---

## 整体执行顺序建议

按"用户能直接感知到体验差距"优先,建议顺序:

1. 模块 8(下载镜像测速)—— 国内用户体感最强
2. 模块 6(崩溃日志分析)—— 口碑关键,遇到问题才会用到但印象深刻
3. 模块 7(JVM 自动调优)—— 和模块 6 关联度高,可以放一起开发
4. 模块 3(整合包导入导出)—— 决定能不能吸引老用户迁移
5. 模块 1(服务器列表)—— 高频日常使用功能
6. 模块 2(CurseForge 集成)+ 模块 5(mod 依赖检测)—— 一起做,依赖数据结构复用
7. 模块 4(资源包/光影包)—— 复用 mod 管理经验,成本较低
8. 模块 11(存档/截图)—— 相对独立,随时可插入
9. 模块 9(皮肤管理)—— 视觉效果好但优先级可以靠后
10. 模块 10(自更新)—— 建议放在功能基本稳定、准备正式对外分发前再接入

每个模块完成后按前面约定:本地测试通过 → 过 Review Checklist → commit + push,不要攒到一起提交。
