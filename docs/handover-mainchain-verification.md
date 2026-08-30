# RustMCL —— 主链路稳定性修复:代码级结论 + 人工端到端验证清单

> 本文件是「核心功能稳定性修复 Spec(P0)」各阶段推进后的**交接说明**。
> 结论分两类:已完成的**代码级接线**(本轮已验证),以及**必须在真实环境人工跑通**的端到端清单
> (本机无法启动 Minecraft / 访问 Azure,也不按提示运行 cargo check / test,故以下项需人工确认)。
> 前置:【任务0】见 `docs/task0-frontend-backend-findings.md`。

---

## ✅ 代码级结论(本轮逐一读码核实,均已接线)

### 任务 0 —— 前后端联调诊断
- `src/lib/api.ts` 的 `invoke("xxx")` 与 Rust `#[tauri::command]` 同名,camelCase 参数映射正确。
- `src-tauri/src/lib.rs` `generate_handler!` 无漏注册(instance/mods/resourcepacks/mods 等全部在场)。
- **P0(阻断)**:仓库由旧项目 Runa 改名而来,`src-tauri/target/` 缓存里的 `tauri-*` 插件权限文件记录了旧绝对路径,导致 `cargo check` 爆路径错误。**修复**:进入 `src-tauri` 后 `rm -rf target` 重检(上一轮已验证可行)。

### 任务 1 —— 登录门禁(已贯通三层)
- **后端门禁**:`src-tauri/src/commands/launch.rs` `spawn_instance_launch` 在 `spawn` 之前校验激活账号,
  `get_active_account().is_none()` 直接返回 `Err`,不再先下载资源后报错。
- **前端拦截**:`src/stores/instance.ts` `launch()` 检查 `useAccountStore.active`,未登录调用 `openLogin()` 并 `return`,
  不发起 invoke。
- **全局账号状态**:`src/stores/account.ts` 由 App 挂载时 `loadAccounts()` 填充 `active`;
  `LoginModal` 在 `App.tsx` 常驻渲染,登录/登出后 `reloadAccounts()` 更新 `active`。
- **仓库**:`repository.rs` `get_active_account` 为 `WHERE is_active = 1 LIMIT 1`;离线/微软账号入库均 `is_active = 1`。
- **结论**:未登录点启动 → 弹登录框;登录后无需刷新即可启动;离线/微软都算已登录。

### 任务 2 —— 下载→启动主链路(本轮补齐"进度可见"这一断点)
- 链路:创建实例(`create_instance` 写 DB + `instances/<id>/instance.toml`)→ 非原版自动装加载器
  (`install_loader`/`install_forge`)→ 点击启动 →
  `spawn_instance_launch` 先 `run_download`(自动补齐 client.jar + libraries + natives + assets,幂等)→ `run_launch`
  (拼参 + `launch_process` 拉起子进程,日志经 `game-log` 转发,退出经 `game-exit`)。
- **本轮修复(命中"卡住不动")**:首次启动会下载数 GB 资源,但此前 `download-progress` 事件被实例页监听器丢弃
  (仅在 `installingId` 有值时响应),界面只显示"启动中"旋转图标。现已:
  - 后端 `spawn_instance_launch` 资源下载成功后发 `download-finished`(ok)。
  - 实例 store 新增 `launchProgress`;实例页监听 `download-progress`(`runningId` 时)写入进度条,
    收到 `download-finished` 或 `game-exit` 时清空。
  - `Downloads` 页监听 `download-finished` 加"仅下载中才响应"守卫,避免实例启动路径的该事件误改状态。
- **设计取舍**:下载在"启动"时自动触发(非创建时),符合"启动即自检资源"的幂等模型;下载页仍提供单独的显式下载入口。

### 任务 3 —— Mod 页面(搜索/安装/管理已全接线)
- 前端 `src/pages/Mods.tsx` + `src/stores/mods.ts`:Modrinth / CurseForge 切换、搜索、版本选择弹窗、安装、
  已装列表(`list_instance_mods`)、启用/禁用(`set_mod_enabled`)、删除(`delete_mod`)、依赖检查(`check_mod_dependencies`)。
- 后端 `commands/mods.rs` 各 command 均已注册。CurseForge 需在设置页配置 API Key。
- **本轮修复(读码发现的真实缺口)**:
  - `set_mod_enabled` 原先**只翻 DB 标志**,mod jar 仍在 `mods/` 下 → 游戏仍加载被"禁用"的 mod。
    现已改为与资源包一致:禁用重命名 `<file_name>.disabled`、启用改回;加载器只加载 `*.jar`,故 `.jar.disabled` 不生效。
  - `delete_mod` 原先只删 active 文件,删除"已禁用"的 mod 会残留 `.disabled` 孤儿。现同时删除两种文件名。
  - 安装均落到实例专属 `instances/<id>/mods/`,与启动 `--gameDir` 一致,保证"下次启动该 mod 生效"。

### 任务 4 —— 资源包 / 光影包(本地管理已达成验收线)
- 前端 `src/pages/Packs.tsx` + `src/stores/packs.ts`:资源包/光影包两个 tab、实例选择、重新扫描、
  已装列表、启用/禁用、删除。
- 后端 `commands/resourcepacks.rs`:`scan_resource_packs`(扫本地目录并同步 DB)、`set_resource_pack_enabled`
  (重命名 `xxx.disabled`)、`remove_resource_pack`、`check_shader_support`(检测 Iris/OptiFine)。
- **在线搜索** `search_resource_packs` 存在但**只读,无安装 command** → 按 Spec 4.3 属后续迭代;
  本期以"本地管理(启用/禁用/删除)"为验收线。

---

## ⚠️ 需人工在真实环境端到端验证(此处无法执行)

> 每项都要求"真实跑通一次使用场景",而非编译/单测通过。

### A. 构建前置(一次性)
- [ ] 进入 `src-tauri` 执行 `rm -rf target && cargo check`,确认编译通过(清除 Runa 旧路径残留)。
- [ ] `npm run tauri dev` 能拉起窗口且后端正常初始化(数据目录 `~/.rustmcl`)。

### B. Azure AD 配置(只能由你操作,不涉代码)
- [ ] 应用注册 → 身份验证 → 高级设置 → **Allow public client flows = 是**(否则必现 `unauthorized_client`)。
- [ ] 若用自建 client id:确认类型为公共客户端、API 权限含 `XboxLive.signin`。
- [ ] 代码层已使用官方 `00000000402b5328` 与 `consumers` 租户的 devicecode 端点,一般无需上述配置。

### C. 主链路端到端(任务 2 验收)
- [ ] 左下角创建**离线账号** → 弹窗关闭 → 实例页点"启动" → **看到下载资源进度条** → 完成后进入"启动中" → 游戏窗口弹出并能进主菜单。
- [ ] 从 Home 英雄区"启动" → 自动跳转实例页并出现进度与日志。
- [ ] 未登录(退出账号)时点"启动" → **不触发下载/进程**,只弹登录框。
- [ ] 微软账号 device code 登录:拿到 `user_code` → 浏览器授权 → 回到启动器显示已登录 → 启动成功。

### D. Mod / 资源包(任务 3/4 验收)
- [ ] Mod 页搜索 Sodium → 选版本安装 → `~/rustmcl/instances/<id>/mods/` 下出现 jar;游戏内生效。
- [ ] 已装 Mod 列表切页面后仍在(证明落库到 SQLite)。
- [ ] 资源包页放入本地 .zip → 扫描 → 启用/禁用 → 磁盘实际产生/移除 `.disabled` 后缀;删除后文件消失。

---

## 提交节点对照
1. `fix(auth): 修复 Azure AD 设备码流程配置` —— 需人工完成 B 后视改动提交(Git/代码层未变则仅记结论)。
2. `chore(debug): 前后端接口联调排查与问题清单` —— `docs/task0-frontend-backend-findings.md`(已有)。
3. `fix(core): 打通下载-启动主链路` —— 本轮改动(启动进度可见 + download-finished 事件)。
4. `feat(account): 登录门禁与账号状态管理` —— 前一组改动(launch.rs 门禁 + instance.ts 拦截 + Home 反馈)。
5. `feat(mods): Mod 下载页面与实例集成` —— 代码已在,待人工 D 验证。
6. `feat(resourcepacks): 资源包/光影包管理页面` —— 代码已在,待人工 D 验证。
