# DayFlow 架构

## 数据流

React → 类型化 Service → Tauri IPC → Rust Repository → 单例 SQLite Connection。

两个 WebView 共享 Rust 管理的一个 `Database`。连接由 Mutex 串行化，WAL、外键、5 秒 busy timeout 与 FULL 同步模式开启。数据库存放在 Tauri 的 `app_data_dir/com.dayflow.desktop` 对应目录（Windows 通常为 `%APPDATA%\com.dayflow.desktop\dayflow.db`，以设置页显示为准）。

写入提交成功后广播 `data-changed`，两窗口刷新快照；焦点恢复时再次读取。无逐秒数据库轮询。30 秒的本地时钟更新只重新计算 UI 到期状态。后台提醒每 30 秒检查一次。

## 持久化与历史

`PRAGMA user_version` 管理按版本逐步执行的 SQL 迁移；已执行的迁移不得修改。新增版本应添加新迁移和升级代码。任务与事件使用同一个事务。状态重复写入是幂等操作；表单更新带 `expected_updated_at`，拒绝覆盖另一窗口的新版本。设置只修改指定键，避免用旧快照覆盖其他设置。

事件快照包含任务当时的全部字段以及分类名称。报告读取事件 UTC 时间，使用查询时本机时区分别计算每个日历边界，不以固定 24 小时推算日/月/周，也不使用截止日期推断完成日期。完成事件按任务 ID 去重，每日趋势按每天的完成任务数计算，因此跨日反复完成的同一任务可能出现在多个日柱中。

SQLite 在线 Backup API 用于手动与每日启动备份。恢复流程：只读打开 → integrity_check → 版本/表结构/外键/快照反序列化检查 → 原库安全备份 → 在线 Backup 替换 → 广播刷新。恢复过程持有数据库锁，防止并发写入。

## 原生窗口

`windows/mod.rs`：Tauri 窗口显示、模式、物理位置恢复。恢复时检查当前监视器，修正不可见位置。`desktop_host.rs`：唯一包含 Explorer 特殊实现的模块；检查 DPI 上下文一致后才调用 SetParent，保留并恢复原父级和窗口样式。无输入穿透全屏罩层，也不修改 Explorer 图标窗口。

桌面适配目前依赖 Explorer 非公开的 WorkerW 行为，不能承诺所有 Windows 版本可用。兼容模式是正式可用行为；真实挂载的恢复和混合 DPI 验收另见 TESTING.md。

## 前端

严格 TypeScript，React Hooks 与 Context 管理本地快照。任务排序、报表文本格式化在 Service；视图负责筛选和交互。CSS 使用系统字体与 Lucide 图标，无远程字体、CDN、网络数据请求。浏览器直接访问开发地址会明确告知需要 Tauri，不提供伪装成数据库的 localStorage 副本。
