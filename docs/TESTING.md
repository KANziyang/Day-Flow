# 验证记录

本文件随实际验证更新，未执行的项目不能标为通过。

## V1.0.1 桌面小组件修复（2026-10-17）

- 用户在 Windows 10 安装版反馈：主窗口和托盘正常，小组件位于桌面图标之下，无法点击或拖动；通过托盘或应用内退出后仍看到残留。旧版代码选择了图标宿主后方的壁纸 WorkerW，且启动、隐藏、恢复和退出时的 Tauri 窗口样式重建会丢掉外部设置的 `WS_CHILD`。未在用户设备上直接观察进程和窗口句柄，退出后残留是活窗口还是绘制残影尚未确认。
- 已改为挂到包含 `SHELLDLL_DefView` 的父宿主，将小组件置于图标视图之上；移除创建壁纸 WorkerW 的消息。挂载时用原生接口显示/隐藏，解除后保留当前可见状态并同步框架，退出前先隐藏并解除挂载。
- `npm test` 6/6 通过。沙箱临时目录最初报 EPERM，本次将测试进程的 TEMP/TMP 指向 `.tools/test-temp` 后通过，未改动测试或应用逻辑。
- `npm run test:rust` 17/17 通过，其中 3 项新增原生回归使用自身创建的隐藏 Win32 窗口，覆盖宿主发现、图标同级 Z 序、挂载/解除前后的屏幕坐标、样式与当前可见位。测试不操作 Explorer，也不等同于 Win10 实机通过。
- `npm run check:rust`、严格 TypeScript/Vite 构建、Rust 格式检查与差异空白检查通过。`node scripts/tauri.mjs build --debug --no-bundle` 成功。
- 发布 Rust 优化构建通过。隔离环境中的 NSIS 工具启动报 DLL 重定位错误 623，在正常 Windows 环境重新执行 `node scripts/tauri.mjs build` 后完整构建和 NSIS 打包成功。输出 `src-tauri/target/release/bundle/nsis/DayFlow_1.0.1_x64-setup.exe`，2,045,079 字节；SHA256：`6447d5b623de0fe9d63baa1bcc25560f670a10746d68d2829508d7e4cd8cd9b5`。发布 EXE 的 PE 产品版本为 `1.0.1`；本轮未安装该安装包。
- 使用 `.tools/widget-1.0.1-test` 独立数据库启动调试版，实际加载本地嵌入资源，主窗口可访问性树显示 `V1.0.1`。后续界面操作分别收到 `window bounds changed`、`user input was detected in this window`，未完成交互验收；已仅关闭本轮启动的调试进程，测试数据保留。
- **Win10 待复测**：小组件与桌面图标重叠时可点击、输入中文、拖动和缩放；点击桌面图标后层级稳定；隐藏后从主窗口/托盘恢复；桌面与置顶双向切换保持位置、尺寸及可见性；从托盘和应用内分别真正退出后无小组件残留且进程结束；重新启动后模式和位置恢复。多显示器、混合 DPI、Explorer 重启仍未验收。

## V1.0.0 发布与覆盖升级（2026-10-17）

- npm、Cargo、Tauri 发布版本及锁文件统一为 `1.0.0`。主窗口侧栏和设置页显示 `V1.0.0`，共同读取 `tauri.conf.json` 的构建版本。
- 本轮 `npm test` 6/6 通过，`npm run tauri build` 成功，包含严格 TypeScript、Vite 和 Rust 发布构建；版本一致性检查通过。
- 对已安装的 `0.1.0` 创建应用内在线备份并正常退出，再通过资源管理器执行新版 NSIS 的 `/S /UPDATE /R`，没有先卸载旧程序。
- 覆盖后的程序仍位于 `C:\Users\hp\AppData\Local\DayFlow\dayflow.exe`，PE 产品版本为 `1.0.0`；自动启动后只有一个进程，主窗口与小组件均显示。
- 已通过真实窗口确认主窗口左侧的 `V1.0.0` 和设置页底部的 `DayFlow V1.0.0`。升级前的完成任务及独立日志正常显示。
- 升级前后均通过应用备份 API 创建一致性快照，只读比较所有业务记录：1 个任务、2 条历史事件、1 条日志、4 个分类逐条完全一致，SQLite 完整性检查与外键检查通过，迁移版本仍为 1。
- 本次没有额外写入的用户偏好设置；窗口位置键在退出时正常保存，不计入业务记录一致性比较。
- 快照副本为 `.tools/upgrade-0.1.0-before.db` 和 `.tools/upgrade-1.0.0-after.db`；检查结果为 `.tools/upgrade-1.0.0-result.json` 和 `.tools/upgrade-data-comparison.json`。
- 新版安装包 `C:\Users\hp\Desktop\DayFlow\src-tauri\target\release\bundle\nsis\DayFlow_1.0.0_x64-setup.exe`，2,044,187 字节；SHA256：`5fd9b34d0737fe8e440d1b98514d080a1932fcf68886e1423762c89b77e6a154`，未代码签名。旧版安装包与其校验记录保留。
- 最后保留安装后的 V1.0.0 运行。以下 0.1.0 记录是此前的安装/卸载验收历史；V1.0.0 本轮验证的是不卸载的覆盖升级。

## 自动化覆盖

- TypeScript：逾期/今天/明天/下周/无日期排序，日期任务当天不逾期，同截止时间重要性及创建时间排序，已完成/取消排除逾期，本地日期显示。
- Rust：任务生命周期、完整输入、校验、超长标题、非法日期/时刻、重复完成幂等、乐观并发、外键事务回滚、软删恢复、独立/历史工作日志、日志更新冲突。
- 报表：当日/昨日、完成后重开改名删除、去重与重复完成事件、进行中任务日志、空数据、周日/周一、跨年、闰年与自然月。
- 数据：真实文件数据库重开、迁移重复运行、在线备份、恢复前安全备份、拒绝损坏数据库、JSON 导出、分类与设置持久化。

## 命令

```powershell
npm install
npm test
npm run build
npm run check:rust
npm run test:rust
npm run tauri build
```

`check:rust` 和 `test:rust` 是对 cargo 的薄封装，若本项目 `.tools` 中存在工具链会自动使用，否则使用系统 PATH；也可进入 src-tauri 执行 cargo check / cargo test。

## 必须在实际 Windows 设备确认

- 双窗口任务更新与排序同步、小组件日期创建、详情定位。
- 普通应用上的置顶效果、切回兼容桌面、重启恢复。
- 托盘显示/隐藏/退出、单实例、主窗口关闭行为。
- NSIS 安装/卸载、Windows 系统通知与开机启动。
- 不同 DPI、多显示器插拔、Explorer 重启后的宿主恢复。

## 本次执行结果

日期：2026-10-17，真实 Windows 11 家庭版中文版 x64，版本 10.0.26300。

| 检查 | 实际结果 |
| --- | --- |
| npm install | 成功，已生成 package-lock.json |
| npm test | Vitest 4.1.12，6/6 通过 |
| npm run build | 严格 TypeScript + Vite 生产构建通过 |
| cargo check --locked（src-tauri 目录） | Rust 1.99.1 MSVC，通过 |
| npm run test:rust | 14/14 通过，真实 SQLite 临时文件与在线备份；含跨时区编辑保持原截止瞬间的回归测试 |
| cargo fmt -- --check | 通过 |
| npm audit / npm audit --omit=dev | 均为 0 个漏洞（仅 npm 依赖审计） |
| npm run tauri build | 本轮重新完整执行成功（12:41）；发布 EXE 与 1.95 MiB NSIS 安装器均已生成 |

### Windows 11 正式安装版验收已通过

- 从资源管理器打开简体中文 NSIS 安装向导并完成安装；实际安装的程序为 `C:\Users\hp\AppData\Local\DayFlow\dayflow.exe`，不是工作区中的调试程序。
- 安装器创建开始菜单和桌面快捷方式；已从桌面快捷方式通过资源管理器实际启动。Windows“程序和功能”中显示 DayFlow，版本 0.1.0，发布者 dayflow，大小 5.07 MB。
- 正式安装版同时显示主窗口和小组件，页面加载嵌入的 `http://tauri.localhost/` 资源，不需要 Vite 开发服务器。
- 在小组件输入并保存“安装验收：关闭重启后保留任务”；关闭主窗口后小组件继续运行，点击展开可重新打开主窗口。
- 使用“退出应用”彻底退出，检查 DayFlow 进程数为 0；从桌面快捷方式重新启动后，小组件仍显示此前保存的任务。
- 小组件完成该任务后，主窗口同步显示完成数量为 1；保存独立的安装验收日志后，日报正确包含 1 个完成任务、1 次完成事件和 1 条工作日志。
- 正常退出后，通过资源管理器环境执行正式 NSIS 卸载器的 `/S` 静默卸载。检查程序 EXE、卸载器、开始菜单和桌面快捷方式均已移除，DayFlow 进程数为 0；默认保留数据库。
- 再次通过资源管理器环境静默安装同一个 NSIS 包，从桌面快捷方式启动后，原任务的完成状态和独立工作日志均恢复。再次在 Windows“程序和功能”中确认可见的卸载项。
- 最后保留已重新安装并启动的应用。正式数据库中的任务和日志明确标注“安装验收”，均为实际测试输入；安装包不包含这些数据。此前 `.tools/native-test-data` 中的开发验收和用户手动输入记录也完整保留。
- 安装后 EXE 与发布 EXE 仅有 Tauri 官方包类型标记 `__TAURI_BUNDLE_TYPE_VAR_UNK` → `__TAURI_BUNDLE_TYPE_VAR_NSS` 的 3 字节差异；按官方 NSIS 标记规范归一化后逐字节一致。安装 EXE SHA256 为 `528fef915e93de614790ac71e2db82f581a518b0e6d03c43dadbb5a86c26abd6`。

安装和卸载的机器检查结果保存在 `.tools/nsis-install-result.json`、`.tools/nsis-uninstall-result.json`、`.tools/nsis-binary-verification.json`。首次从 Codex 子进程直接安装受到 MSIX 路径重定向影响；最终正式验收通过资源管理器启动，窗口进程路径已确认不在 Codex 的 LocalCache 中。

### 真实窗口验收已通过

- 同时启动 `main` 与 `widget`，简体中文页面及 320px 小组件正常显示。
- 在小组件创建“完成项目代码优化”，用真实日期/时间输入控件选择 2026-10-17 18:00；SQLite 提交后主窗口未切换焦点即显示任务与截止时刻。
- 小组件完成任务，显示完成反馈；主窗口反映已完成，并可从小组件点击标题定位详情。
- 独立日志不关联任务保存，出现成功反馈；日报包含真实日志与完成快照，复制按钮反馈“已复制”。
- 完成后重开再完成，日报完成任务仍按 ID 去重，并显示两次完成事件；当前状态和历史事实分别展示。
- 周报范围为 2026-10-12 至 2026-10-18，每日趋势正确；切换上周显示 2026-10-05 至 2026-10-11 的空记录。
- 月报范围为 2026-10-01 至 2026-10-31，显示每周完成趋势、完成快照和分类汇总。
- 主窗口设置可切换始终置顶，小组件同步图钉与模式状态；重启后恢复置顶，再次点击图钉切回明确标注的桌面兼容模式。
- 关闭主窗口只隐藏，保留小组件；再次启动第二个进程会唤回主窗口，第二个进程退出，实际仅保留一个实例。
- 使用“退出应用”正常退出，进程结束；关闭 Vite 服务后，重新运行嵌入资源的调试 EXE，URL 为 `http://tauri.localhost/`，任务、完成状态、两条日志与窗口模式均保留。
- 上述为无开发服务器的本地资源/数据库验证；主机网络仍连接，未执行关闭网卡的物理断网测试。
- 切换深色主题后，主窗口和小组件同步渲染深色；设置保存反馈正常。
- 从设置页实际创建手动 SQLite 备份和 JSON 导出，导出反序列化验证：1 个任务、2 条日志、4 个历史事件，格式为 DayFlow。

验收数据备份：`C:\Users\hp\Desktop\DayFlow\.tools\native-test-data\backups\manual-20261017-122719-f7d6f99c.db`。

验收 JSON：`C:\Users\hp\Desktop\DayFlow\.tools\native-test-data\exports\dayflow-20261017-122728-70927337.json`。

最终发布文件（2026-10-17 12:41，Asia/Shanghai）：

```text
C:\Users\hp\Desktop\DayFlow\src-tauri\target\release\dayflow.exe
C:\Users\hp\Desktop\DayFlow\src-tauri\target\release\bundle\nsis\DayFlow_0.1.0_x64-setup.exe
C:\Users\hp\Desktop\DayFlow\src-tauri\target\release\bundle\nsis\SHA256SUMS.txt
```

安装器未签名；SHA256SUMS.txt 记录本次最终安装包的校验值：`c0da973bb1614dc0e9133f116792343c0cbce44ac6c2e0d0bd94f9c5ffb76d5e`。最终 npm build 已执行严格类型检查；Rust 最后修复后再次执行 cargo check 与 14 项测试，均通过。本轮前端测试再次执行，6/6 通过。

早先开发验收使用独立目录 `C:\Users\hp\Desktop\DayFlow\.tools\native-test-data`；其中含验收输入及用户手动输入的日志，予以保留。当前运行的正式安装版使用正常应用数据目录。需要迁移开发验收记录时可在设置中备份并恢复到正式安装版。

### 原生桌面模式的真实结果

本机返回“桌面兼容模式 · 当前桌面结构不支持挂载”。因此本机已验证的是普通非置顶与置顶的 API 切换及持久化，**真正 WorkerW 挂载没有通过本机验收**。未修改 Explorer，没有重启 Explorer。Explorer 重启、混合 DPI、多显示器热插拔与长期挂载稳定性仍未验证。IDE 截图捕获不包含其他窗口遮挡，不能据此宣称跨应用 Z 序视觉验收通过。

### 尚未执行的系统验收

- 其他干净 Windows 10/11 设备验收；本机 Windows 11 安装/卸载已执行，安装器未代码签名。
- 重启 Windows 后的开机启动投递与正式安装后的系统通知、休眠/勿扰实机矩阵。
- 托盘菜单的逐项 UI 点击、拖动/缩放和窗口位置的多显示器矩阵。

### 开发中遇到并处理的问题

- 初始仅有 README、无 Rust；已从官方源安装工作区内 Rust 1.99.1，并验证下载 SHA256。
- CLI 2.12.4 未发布，改为实际可用的 2.12.3，Rust/API 2.12.4；锁文件固定本次依赖。
- npm 审计发现旧测试依赖问题，升级 Vitest 后审计为 0；沙箱临时目录重命名 EPERM 通过在正常 Windows 环境运行测试解决。
- Windows PATH 大小写问题导致构建子进程找不到 npm，已修复命令包装脚本。
- GitHub NSIS 下载超时，使用同一官方发布的 GitHub API 下载 NSIS 与辅助 DLL，核对发布 SHA256 及 Tauri 官方 SHA1，再缓存到 `src-tauri/target/.tauri/`；未关闭哈希检查。
- 开发服务器起初位于隔离网络中，原生 WebView 无法连接；重启服务到正常 Windows 环境后成功。最终离线验收完全关闭了该服务。
- 在运行 EXE 时重建同一路径被 Windows 文件锁拒绝；正常退出后构建成功。调试增量缓存曾提示硬链接失败并改为复制，不影响测试结果。
- 本轮隔离环境中的 makensis.exe 启动报系统 DLL 重定位错误（623）；在正常 Windows 环境重新执行完整 `npm run tauri build` 后成功，未更改官方 NSIS 工具或关闭校验。
