# DayFlow

Windows 10/11 本地任务、工作记录与周期复盘应用。Tauri 2 + Rust + React 19 + TypeScript + SQLite。核心数据留在本机，运行不依赖网络、账号或 AI 服务，首次启动不写入示例任务。

## 启动

开发需要 Node.js 20、Rust MSVC 工具链、Visual Studio C++ 桌面开发工具和 WebView2。本次使用 Node.js 20.18.2、Rust 1.99.1、React 19.3.1、Tauri Rust/API 2.12.4、CLI 2.12.3、Vite 6.4.4。

```powershell
cd C:\Users\hp\Desktop\DayFlow
npm install
npm run tauri dev
```

本项目 `.tools` 若存在本次安装的 Rust 工具链，包装脚本会自动使用；其他电脑使用正常安装并位于 PATH 的 Rust。`.tools` 不属于交付源代码，不进入安装包。`npm run dev` 只是浏览器服务，不能当作完整应用启动方式。

参考 [Tauri 官方 Windows 环境要求](https://v2.tauri.app/start/prerequisites/)。首次安装时如果系统缺少 WebView2，默认安装器会下载该组件。已具备 WebView2 后，任务、日志和报表支持离线运行。

## 使用

- 小组件输入标题并按 Enter 创建任务，日历按钮设置日期与可选时刻；左侧圆圈完成，点击标题打开详情。
- 任务详情明确保存，支持描述、四种状态、三档重要程度、分类、截止日期/时刻。删除进入回收站，可恢复；完成、重开和删除事件保留。
- “记录今天做过的事”无需先创建任务，日志可补录历史日期，也可关联正在推进的任务。
- 周期复盘支持历史日报、周报、月报和复制总结；周报周一至周日，月报自然月。
- Ctrl+N 聚焦快速添加，Ctrl+K 搜索，Escape 关闭详情，未保存先确认。

## 窗口与退出

主窗口默认 1120×760，小组件 320×420，最小 260×300。小组件顶部拖动、边缘缩放。默认关闭主窗口后隐藏，后台与小组件继续运行；可在设置修改。主窗口左下角和系统托盘提供彻底退出入口。应用使用单实例插件。

置顶使用 Tauri 原生 always-on-top。桌面模式先由独立 Win32 模块检查 WorkerW 宿主与 DPI 上下文；只有实际 SetParent 成功才显示桌面宿主。失败准确显示“桌面兼容模式”及原因，实际行为是普通非置顶窗口。模式保存并在重启恢复。**普通非置顶不等于真正桌面嵌入。**

位置以物理坐标保存，拖动停止后合并写入，正常退出时再次保存；启动检查监视器并纠正不可见位置。真正桌面挂载的 Explorer 重启、混合 DPI 和多显示器行为需逐设备验收，见 [测试记录](docs/TESTING.md)。

## 数据与备份

实际数据目录在设置页显示，Windows 通常为：

```text
%APPDATA%\com.dayflow.desktop\
  dayflow.db
  dayflow.db-wal
  dayflow.db-shm
  backups\
  exports\
```

只使用一份 SQLite，由 Rust 单例连接管理。开启 WAL、外键和 FULL 同步；重要操作包含事务，表单校验版本以避免覆盖另一窗口修改。

- 每天首次启动自动创建一致性备份，也可手动备份；保存在 `backups/`，不会自动删除。
- JSON 导出保存在 `exports/`，包含任务、日志、分类、设置和历史事件。当前恢复只接受本应用 SQLite `.db` 备份。
- 恢复前检查完整性、版本、表结构、外键、事件快照；用户确认后，先保存 `before-restore-*` 原数据备份，再替换数据库。
- 运行中不要单独复制 `dayflow.db` 作为备份，使用内置在线备份以包含 WAL 中的已提交数据。

完成快照保留当时标题和分类名称。重开清除当前完成状态，保留历史事件。报表按周期内任务 ID 去重，同时给出完成事件次数；趋势按每天完成任务数汇总，同一任务跨日多次完成可出现在不同日柱。日志按用户指定的本地日期归档。日历边界分别换算到 UTC，正确处理夏令时，避免假设每天固定 24 小时。

具体截止时刻以录入时确定的 UTC 瞬间保存，界面保留录入的本地日期/时刻。仅修改描述或分类不会因系统时区变化而重算截止；明确修改日期/时刻时，才按当时本地时区重新确定。仅日期任务按所在本地日历日判断。

## 测试与打包

```powershell
npm test
npm run build
npm run check:rust
npm run test:rust
npm run tauri build
```

正常安装 Rust 后也可：

```powershell
cd src-tauri
cargo check
cargo test
```

NSIS 发布输出约定：

```text
src-tauri/target/release/dayflow.exe
src-tauri/target/release/bundle/nsis/DayFlow_0.1.0_x64-setup.exe
```

安装器使用当前用户安装。构建产物未代码签名。是否已生成与实际验收结果，以 [TESTING.md](docs/TESTING.md) 为准。

## 主要模块

```text
src/App.tsx                     主窗口与导航
src/widget/WidgetView.tsx        桌面小组件
src/components/                 快速添加、任务条目、详情和进展
src/views/                      工作日志、周期复盘、设置
src/services/                   类型化 IPC、排序、报告文本
src/hooks/useData.tsx           双窗口同步与数据状态
src-tauri/src/db.rs             迁移、备份、恢复、导出
src-tauri/src/repository.rs     任务、事件、日志、分类及校验
src-tauri/src/reports.rs        历史状态与周期汇总
src-tauri/src/windows/          窗口状态和独立桌面宿主
src-tauri/src/lib.rs            IPC、托盘、单实例、开机启动、提醒
src-tauri/migrations/           版本迁移，禁止改写已执行历史
src-tauri/src/tests.rs          SQLite 和报表自动化测试
tests/                         前端排序和日期测试
docs/                          需求、架构、交互、验证记录
```

## 当前边界与已知问题

- 原生桌面宿主依赖 Explorer 非公开结构，设有可用性和 DPI 检查。兼容模式是普通窗口行为，不能视为全部原生桌面验收通过。
- 已有宿主失效检查，但真正挂载时 Explorer 重启造成的子窗口销毁/恢复仍需专项验收，不承诺已完全解决。
- 多显示器热插拔、不同 DPI、安装/卸载、通知投递、开机登录启动需要在相应设备继续验收，不能只以编译通过认定通过。
- 通知默认关闭，每个截止时间只提醒一次，支持勿扰；仅日期任务默认 09:00 提醒。显示还受 Windows 权限和专注助手影响，开发与正式安装表现可能不同。
- 日志删除永久生效且需要确认；任务软删除。历史日志被用户编辑或删除后，报告随数据库变化。
- 没有云同步、账号、AI 总结、重复任务、自动更新、Markdown 文件导出，不显示这些能力的占位开关。
