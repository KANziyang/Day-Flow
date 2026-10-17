# 验证记录

本文件随实际验证更新，未执行的项目不能标为通过。

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

日期：2026-10-17，真实 Windows x64 环境。

| 检查 | 实际结果 |
| --- | --- |
| npm install | 成功，已生成 package-lock.json |
| npm test | Vitest 4.1.12，6/6 通过 |
| npm run build | 严格 TypeScript + Vite 生产构建通过 |
| cargo check --locked（src-tauri 目录） | Rust 1.99.1 MSVC，通过 |
| npm run test:rust | 14/14 通过，真实 SQLite 临时文件与在线备份；含跨时区编辑保持原截止瞬间的回归测试 |
| cargo fmt -- --check | 通过 |
| npm audit / npm audit --omit=dev | 均为 0 个漏洞（仅 npm 依赖审计） |
| npm run tauri build | 最终跨时区保护改动构建成功；发布 EXE 与 1.95 MiB NSIS 安装器均已生成 |

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

最终发布文件（2026-10-17 12:30，Asia/Shanghai）：

```text
C:\Users\hp\Desktop\DayFlow\src-tauri\target\release\dayflow.exe
C:\Users\hp\Desktop\DayFlow\src-tauri\target\release\bundle\nsis\DayFlow_0.1.0_x64-setup.exe
C:\Users\hp\Desktop\DayFlow\src-tauri\target\release\bundle\nsis\SHA256SUMS.txt
```

安装器未签名；SHA256SUMS.txt 记录本次最终安装包的校验值。最终 npm build 已执行严格类型检查；Rust 最后修复后再次执行 cargo check 与 14 项测试，均通过。

当前运行的验收程序使用独立目录 `C:\Users\hp\Desktop\DayFlow\.tools\native-test-data`；其中含验收输入及用户手动输入的日志，予以保留。正式安装版使用正常应用数据目录。需要迁移本次记录时可在设置中备份并恢复到正式安装版。

### 原生桌面模式的真实结果

本机返回“桌面兼容模式 · 当前桌面结构不支持挂载”。因此本机已验证的是普通非置顶与置顶的 API 切换及持久化，**真正 WorkerW 挂载没有通过本机验收**。未修改 Explorer，没有重启 Explorer。Explorer 重启、混合 DPI、多显示器热插拔与长期挂载稳定性仍未验证。IDE 截图捕获不包含其他窗口遮挡，不能据此宣称跨应用 Z 序视觉验收通过。

### 尚未执行的系统验收

- NSIS 实际安装/卸载与干净 Windows 10/11 设备验收；安装器未代码签名。
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
