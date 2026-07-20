# ADR 0012：桌面歌词窗口与原生解锁辅助窗口

- 状态：Accepted
- 日期：2026-07-19

## 背景

Resona 需要提供透明、置顶、可锁定穿透且可直接恢复交互的 Windows 桌面歌词。Tauri 官方 `set_ignore_cursor_events(true)` 在 Windows 上对整个窗口启用穿透；窗口进入该状态后无法依靠自身 hover 事件显示解锁按钮。

在同一 WebView 上实现局部命中需要介入 WebView2/Win32 HWND 命中链，对跨进程穿透、DPI 和 Tauri/Wry 升级较敏感。全局鼠标轮询、鼠标 hook、输入注入或为单个按钮创建额外 WebView 都会增加不必要的常驻成本和故障面。

## 决定

- 桌面歌词使用同进程的独立 Tauri WebView 窗口，复用 Rust 权威播放与歌词快照，不创建 sidecar 进程或第二套歌词服务。
- 锁定时由 Tauri 官方整窗穿透能力处理歌词 WebView，不修改 WebView2 子窗口的局部命中行为。
- 锁定后保留一个独立、微型、不含 WebView 的 Windows 原生 owned tool window 作为解锁辅助窗口。它负责 hover、点击和最小视觉反馈，不进入任务栏、不主动抢焦点。
- 解锁辅助窗口由 `DesktopLyricsWindowService` 与 Windows platform adapter 管理。点击时先恢复歌词窗口交互，再隐藏辅助窗口。
- 主窗口始终提供解锁和隐藏入口。辅助窗口未成功创建或显示前不得提交锁定状态；失败必须拒绝或回滚锁定。
- 除桌面歌词自身外，不为解锁辅助窗口增加额外 WebView；不使用持续全局鼠标轮询、全局鼠标 hook、输入注入或向后方程序转发鼠标消息。
- Windows 原生类型和窗口消息在 platform adapter 内终止；通用 UI 只接收类型化 capability、状态和 command 结果。
- 先通过 0.0.10 最小技术原型验证该组合，再决定持久化、多显示器恢复和最终视觉实现。

## 理由

- 歌词窗口沿用 Tauri 官方整窗穿透路径，避免依赖 WebView2 内部 HWND 结构。
- 原生辅助窗口始终是正常命中目标，不需要让一个已经穿透的窗口反向检测鼠标进入。
- 辅助窗口不含 WebView，常驻资源和启动成本远低于第二个 WebView，职责也足够单一。
- 两个窗口的故障边界清楚：歌词展示失败不影响播放；辅助窗口失败不能进入锁定；主窗口保留恢复路径。
- 该方案贴近原生桌面播放器常用的分层窗口模式，同时保留未来平台通过 capability 采用不同实现的空间。

## 后果

- Windows adapter 需要管理两个 HWND 的所有权、置顶顺序、位置、DPI 和销毁顺序，并处理少量原生绘制与鼠标消息。
- 原型必须实测跨进程点击、滚轮、拖动穿透，以及辅助窗口不抢焦点；浏览器预览和单元测试不能替代。
- Linux/Wayland 不承诺相同窗口组合。未来实现必须报告实际 capability，不照搬 Win32 方案。
- 如果原型失败，必须回到架构决策层选择全窗穿透加主窗口解锁，或保留整窗交互；不能静默引入被排除的输入方案。

## 0.0.10 实现说明

Tauri 2.11.5 稳定 API 的无 WebView `WindowBuilder` 仍要求 `unstable` feature。0.0.10 不为这一微型窗口启用不稳定能力，而是在 Windows platform adapter 内通过 raw Win32 创建 owned tool window，并由 Tauri 主事件线程持有窗口创建过程。该调整不改变本 ADR 的窗口分层、恢复顺序、故障回滚和禁止输入注入等约束。
