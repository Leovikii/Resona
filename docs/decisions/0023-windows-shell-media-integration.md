# ADR 0023：Windows Shell 媒体与任务栏集成

- 状态：Accepted；rc.3 修订
- 日期：2026-07-23，修订于 2026-07-25

## 决定

- `NativePlaybackProjection` 是 SMTC、任务栏和托盘的唯一原生播放投影。它低频消费 Rust 权威 `PlaybackSnapshot` 与共享当前曲目封面，不经 WebView 中转，也不分别轮询 Rodio。
- SMTC adapter 继续使用 souvlaki。共享封面编码字节按内容哈希原子写入应用缓存，正规化 WinRT 不接受的扩展路径前缀后再以 `file://` 提交；切歌/退出清理旧文件。歌曲封面失败时先重试应用占位图，再退到纯文本，不能连带丢失曲名。
- `MetadataService` 为当前曲目生成有界共享 `Artwork`：编码字节供 SMTC，data URL 供 React，限制尺寸的 BGRA 供 DWM。无封面、过大或损坏时统一使用应用占位图。
- 任务栏 `ITaskbarList3` 固定上一首、播放/暂停、下一首三个按钮；事件只通过 typed command channel 进入播放服务。Explorer 重启后响应 `TaskbarButtonCreated` 重新注册。
- 任务栏进度只在时长确定且可定位时显示：播放 normal、暂停 paused、无有效进度时清除，真实播放失败才使用 error。
- 主窗口启用 DWM iconic representation，处理 thumbnail/live-preview 请求并提交只含封面或占位图的位图。画布采用系统请求/客户区协调尺寸，封面等比完整显示、不中裁；窗口消息回调只读缓存 BGRA，不做 I/O、标签解析或图片解码。
- 各子能力初始化或更新失败只记录诊断并降级，不阻止启动和播放。不引入第二套媒体会话实现。

## 理由与验证

单一投影避免多个平台消费者维护不同状态；共享封面缓存避免底栏、SMTC 和 DWM 重复读文件/解码。自动测试覆盖投影映射、缓存上限和失败回退；真实 SMTC、媒体键、Explorer 重启、任务栏按钮/进度/封面、DPI 与 Windows 10/11 必须由安装版验收。
