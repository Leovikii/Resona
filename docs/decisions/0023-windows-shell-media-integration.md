# ADR 0023：Windows Shell 媒体与任务栏集成

- 状态：Accepted；rc.4 修订
- 日期：2026-07-23，修订于 2026-07-25

## 决定

- `NativePlaybackProjection` 是 SMTC、任务栏和托盘的唯一原生播放投影。它低频消费 Rust 权威 `PlaybackSnapshot` 与共享当前曲目封面，不经 WebView 中转，也不分别轮询 Rodio。
- SMTC adapter 继续使用 souvlaki。归一化封面 PNG 直接以 `Artwork` 的内容指纹原子写入应用缓存，正规化 WinRT 不接受的扩展路径前缀后再以 `file://` 提交；最近使用的不同封面最多保留 16 个，相同指纹只保留一个文件。按代际延迟淘汰避免 `RandomAccessStreamReference` 与立即删除竞争，启动/退出仍清理应用自有目录。歌曲封面失败时先重试应用占位图，再退到纯文本，不能连带丢失曲名。
- `MetadataService` 为当前曲目生成有界共享 `Artwork`：以源字节 SHA-256 维护最多 8 个不同封面的 LRU，相同嵌入图片直接复用；源图只解码一次，并归一化为最大 512 px 的 PNG 和 BGRA。归一化 PNG 的稳定指纹供平台消费者比较，PNG 供 SMTC/data URL，BGRA 供 DWM。无封面、过大或损坏时统一使用应用占位图。
- 任务栏 `ITaskbarList3` 固定上一首、播放/暂停、下一首三个按钮；事件只通过 typed command channel 进入播放服务。Explorer 重启后响应 `TaskbarButtonCreated` 重新注册。
- 任务栏进度只在时长确定且可定位时显示：播放 normal、暂停 paused、无有效进度时清除，真实播放失败才使用 error。
- 主窗口启用 DWM iconic representation，处理 thumbnail/live-preview 请求并提交只含封面或占位图的位图。`resona-taskbar` 以归一化指纹识别封面；相同指纹切歌直接复用现有 `HBITMAP` 组，只有指纹变化时才由工作线程预生成 1–512 px 的有界固定尺寸组，新组完整后原子替换旧组并通知 DWM。窗口消息回调只选择不超过系统请求的最近尺寸并立即提交；回调内不缩放、不创建 GDI 对象、不读文件，也不返回空结果等待 DWM 再次请求。live preview 使用实际缓存尺寸在客户区居中，封面等比完整显示、不中裁。
- SMTC 工作线程显式初始化 Windows Runtime MTA。原生投影同曲目复用共享 `Arc<Artwork>`，不在每个 500 ms tick 重查文件 revision；SMTC 状态变化立即同步，播放中的时间轴最多每 5 秒进入一次 WinRT。
- 各子能力初始化或更新失败只记录诊断并降级，不阻止启动和播放。不引入第二套媒体会话实现。

## 理由与验证

单一投影避免多个平台消费者维护不同状态；两级内容指纹让同专辑封面跨曲目复用，同时以 8 个内存对象和 16 个 SMTC 文件限制资源上界。DWM 会同步等待窗口消息返回，因此消息到达时必须已有可提交位图；保留旧位图直到新组完整可避免任务栏加载转圈，也防止回调拖慢 Explorer。自动测试覆盖源封面 LRU、归一化指纹、位图尺寸选择、SMTC 文件上限/生命周期、时间轴节流和失败回退；rc.4 Windows Release 已完成 SMTC、Explorer、任务栏封面和连续同专辑切歌实机验收，后续系统版本与 DPI 差异仍按发行矩阵抽查。
