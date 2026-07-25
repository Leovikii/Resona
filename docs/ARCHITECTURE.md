# 架构

## 原则

- Rust 拥有播放、播放列表、偏好、压缩、更新、生命周期和平台能力的权威状态。
- React 只展示类型化快照并提交用户意图，不复制领域状态，不直接调用 Win32 或拼接 FFmpeg 参数。
- 核心领域不依赖 Windows；平台能力通过 adapter 隔离并允许逐能力降级。
- 文件写入使用验证、临时文件、原子替换和明确删除顺序；外部回调不等待播放锁或执行重 I/O。

## 总体结构

```text
React windows
  main / desktop lyrics / audio compression
        │ typed Tauri commands + polling where needed
        ▼
Rust application services
  playback / playlists / preferences / metadata / lyrics
  compression / dependency / update / lifecycle
        │
        ├─ persistence (SQLite + JSON preferences)
        ├─ playback engine (Rodio)
        └─ platform adapters (SMTC / taskbar / tray / windows)
```

Tauri commands 负责 DTO 转换和服务调用，不包含领域分支。长任务在 Rust 后台线程或 actor 中运行；WebView 隐藏时停止无意义轮询，恢复可见后立即读取权威快照。

## 前端边界

`src/app` 负责窗口壳、路由和跨 feature 编排；`src/features` 负责用户工作流；`src/shared` 负责模型、Tauri bridge、i18n 和无业务组件。Mantine 只能经应用层或 `shared/ui` 使用，领域 hook 不返回 Mantine 类型。

主窗口有宽屏和窄屏两种信息架构，但共享同一状态。完整播放器是主内容互斥视图；底部播放器稳定挂载。设置、更新和压缩窗口不得创建第二份播放状态。

普通窗口材质由 Rust `window_material` adapter 决定。Windows 11 可用 Mica，其他环境使用实色 token；桌面歌词保持专用透明窗口，不复用普通窗口材质。

## 播放与列表

`PlaybackService` actor 独占 `RodioPlaybackEngine`、当前执行序列、索引、位置、音量和输出设备。React、SMTC、任务栏和托盘只消费 `PlaybackSnapshot`/原生投影并通过 typed command channel 发命令。

持久化用户播放列表是产品层播放序列。激活列表时按顺序把路径投影给播放服务；播放服务内部可有执行结构，但不形成第二个用户可见队列。临时“默认”列表只存在于会话内，用于外部打开和手工添加。

## 元数据与封面

`MetadataService` 按路径、文件大小和修改时间缓存最多 4 首曲目，并按源字节指纹维护最多 8 个不同封面的 LRU。相同嵌入图片直接复用共享 `Artwork`，不重复解码、缩放和 PNG 编码；不同源图归一化为最大 512 px 的 PNG 与 BGRA，并携带归一化内容指纹。PNG 供 SMTC 和 React，BGRA 供 DWM。源封面、解码内存和缓存条目均有上限；失败返回无封面元数据并使用统一占位图。

`NativePlaybackProjection` 合并权威播放快照和共享封面，低频发布给 SMTC、任务栏和托盘：

- SMTC 使用标题、艺术家、专辑、时长、时间轴、可用动作和真实封面；归一化指纹直接映射缓存文件，最近使用的不同封面最多保留 16 个，重复封面只占一个文件。带封面提交失败时重试占位图和无封面元数据。
- 任务栏工具栏固定上一首、播放/暂停、下一首；进度只在时长确定且可定位时显示。
- DWM iconic thumbnail/live preview 只绘制等比居中的封面或占位图。平台工作线程按归一化指纹复用当前 GDI 位图组；只有指纹变化时才预生成新的有界固定尺寸组，并在完整后替换旧组。窗口消息回调只选择不超过系统请求的最近缓存并立即提交，不在 Explorer 同步等待的回调内缩放、创建位图、读文件或投递补偿渲染。

## 歌词与窗口

歌词解析器只处理 LRC、SRT 和文本，返回平台无关的时间片/文本模型。桌面歌词窗口由 Rust 服务管理位置、尺寸、置顶、穿透和 helper；主窗口关闭、托盘隐藏与完整退出由 `ApplicationLifecycleService` 协调。

Win32/COM 回调必须快速返回，通过 channel 或缓存与服务交互。禁止全局鼠标 hook、持续轮询、输入注入或向其他程序转发输入。

## 压缩

`FfmpegDependencyService` 管理固定归档的按需下载、SHA-256、安全解压、取消和同卷原子启用。网络只由用户明确下载依赖触发。

`CompressionService` 管理扫描与批次快照：

- 扫描用 `hound` 进程内验证 WAV 头，规范化/去重路径，不跟随链接类目录。
- 转换前用 `hound` 获取声道、采样率、位深和时长；转换后用 `lofty` 验证临时 FLAC，不启动 ffprobe 热路径。
- worker 数由可用并行度决定，保留响应余量并限制为 1–4；每个文件只有一个 FFmpeg 子进程和独立临时路径。
- 每项维护单调进度、源体积、成功输出体积、状态与错误。取消会终止所有活动子进程并清理未提交临时文件。
- 已存在输出永不覆盖；验证后原子 rename，源删除只能发生在提交成功之后。部分失败不回滚已成功文件。

## 持久化与更新

SQLite schema v5 只保存用户播放列表、列表项目和应用状态。旧 `recent_plays`、`managed_folders` 和 `media_records` 会迁移删除。偏好由 Rust 类型化存储；窗口临时 UI 状态可留在前端。

更新检查由 `ApplicationUpdateService` 单飞协调，设置连接与总超时。应用首帧后每进程静默检查一次；只有发现更新才加载 Markdown 弹窗。下载、签名验证和安装使用 Tauri updater。外链只允许 HTTP(S) 并经 Rust opener。

## 故障隔离

- 元数据、歌词、SMTC、任务栏、托盘、更新或材质失败不能阻止本地播放。
- 输出设备失效由播放服务显式报告并允许重选；不偷偷切换引擎或重采样策略。
- 数据库迁移在事务内执行并以旧 schema fixture 验证用户列表不丢失。
- 原生视觉与硬件行为必须在 Windows Release 中验证；浏览器只证明布局、状态、溢出和静态样式。

长期决定见 [decisions/](decisions/README.md)。
