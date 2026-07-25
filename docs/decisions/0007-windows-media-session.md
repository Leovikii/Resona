# ADR 0007：Windows 系统媒体会话 adapter

- 状态：Accepted；rc.4 Windows 实机验收通过
- 日期：2026-07-18，修订于 2026-07-25

## 背景

0.0.6 需要接收键盘和耳机媒体键，并向 Windows 系统媒体界面发布播放状态、文件名、时长和位置。该能力属于平台集成，不应让 Windows 类型、窗口句柄或系统回调进入播放领域契约。

候选方案为 `souvlaki` 与直接使用 `windows` crate。项目要求优先成熟库、最少自有代码，并保留未来 Linux MPRIS 扩展空间。

## 决定

- 0.0.6 使用 `souvlaki 0.8.3`，Cargo 中关闭默认 feature，不引入 D-Bus 依赖。
- 进程启动时设置固定 AppUserModelID `io.github.vki.resona`，用于 Windows 媒体浮层和任务栏识别；该调用只存在于 Windows platform adapter。
- `souvlaki` 只存在于 `platform/media_session` adapter；`PlaybackEngine`、队列模型、Tauri command 和前端不依赖其类型。
- adapter 在主窗口首个可用窗口事件后取得 HWND，在独立线程以 MTA 初始化 Windows Runtime，再注册 `MediaControls`；退出时在同一线程释放 controls 并反初始化。
- 系统回调只发送内部 `MediaSessionCommand`；adapter 线程再调用现有 `PlaybackEngine`，不在系统回调中持有 UI 或播放 actor 锁。
- 原生投影以 500 ms 读取 Rust 权威快照。SMTC 在状态变化时立即同步，播放中的时间轴按 5 秒位置桶更新，暂停位置保持精确；元数据仅在曲目、时长或共享封面变化时更新。共享封面是最大 512 px、带稳定指纹的归一化 PNG；SMTC 按指纹复用文件并保留最近 16 个不同封面，避免 WinRT 延迟读取与立即清理竞争，同时限制长会话磁盘占用。
- SMTC 初始化失败只记录可诊断错误并降级为普通播放器，不影响音频播放。
- 0.0.6 不实现 Toast、系统托盘、Jump List、全局快捷键或 Linux MPRIS。

## 理由

- `souvlaki` 0.8.3 为 MIT，最低 Rust 1.67，与 GPL-3.0-only 项目分发兼容，不要求许可证注入或联网激活。
- Windows 后端直接封装 `SystemMediaTransportControls`，能覆盖当前需求，且保留未来 MPRIS adapter 方向。
- 显式 AppUserModelID 避免 Windows 将未打包的本地 Release 程序显示为“未知应用”。
- 独立线程和 adapter 边界避免系统回调阻塞 Tauri 主线程或音频 actor。
- 低频快照同步实现简单、可诊断，且不增加高频 WebView IPC；时间轴节流避免每个投影 tick 都进入 WinRT。

## 后果与验证

- Cargo.lock 增加 `souvlaki 0.8.3` 及其 Windows 依赖；依赖报告确认无未知许可证。
- 自动测试覆盖 SMTC 命令、封面文件路径、指纹复用、文件上限、缓存生命周期与时间轴节流键；format、Clippy 和 Rust 全量测试通过。
- rc.4 Windows Release 实机已验证 SMTC 封面、连续同专辑切歌、媒体控制与任务栏协同正常；后续 Windows 版本、DPI、媒体键和蓝牙设备差异仍按发行矩阵抽查。
- 若实机暴露 souvlaki 的平台缺陷，再新增 ADR 评估 `windows` crate；不预装第二套实现。
