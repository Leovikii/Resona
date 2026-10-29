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
        └─ platform adapters (SMTC / MPRIS / taskbar / tray / layer-shell)
```

Tauri commands 负责 DTO 转换和服务调用，不包含领域分支。长任务在 Rust 后台线程或 actor 中运行；WebView 隐藏时停止无意义轮询，恢复可见后立即读取权威快照。

## 前端边界

`src/app` 负责窗口壳、路由和跨 feature 编排；`src/features` 负责用户工作流；`src/shared` 负责模型、Tauri bridge、i18n 和无业务组件。Mantine 只能经应用层或 `shared/ui` 使用，领域 hook 不返回 Mantine 类型。

“跟随系统”通过原生 adapter 获取界面语言：Linux 使用 GLib，Windows 使用 GetUserPreferredUILanguages。各窗口在 React 首次渲染前读取语言并初始化 i18n；手动偏好优先。WebView 语言只作为原生读取失败或浏览器预览的回退，启动日志记录两者以诊断差异；不修改进程 locale 或强制某种语言。

主窗口有宽屏和窄屏两种信息架构，但共享同一状态。完整播放器是主内容互斥视图；底部播放器稳定挂载。设置、更新和压缩窗口不得创建第二份播放状态。

普通窗口材质由 Rust `window_material` adapter 决定。Windows 11 可用 Mica；KDE Wayland 探测 ext-background-effect/旧 KWin blur 协议，为外壳区域请求合成器模糊，能力不可用时使用实色 token；桌面歌词保持专用透明窗口，不复用普通窗口材质。

普通主/压缩窗口共享 `WindowTitleBar`，原生窗口状态由事件读取，关闭仍发出原生 CloseRequested 并进入既有生命周期。Win32 adapter 保留 HTMAXBUTTON/Snap hover 和系统菜单；Linux 从真实 GTK 按钮事件保留 Wayland 序列号，调用 compositor 窗口菜单。尺寸仍由 Rust 管理。

## 播放与列表

`PlaybackService` actor 独占 `RodioPlaybackEngine`、当前执行序列、索引、位置、音量和输出设备。React、SMTC、任务栏和托盘只消费 `PlaybackSnapshot`/原生投影并通过 typed command channel 发命令。

持久化用户播放列表是产品层播放序列。激活列表时按顺序把 `MediaSource` 投影给播放服务；普通来源只有物理路径，CUE 逻辑音轨还携带 CUE 路径、曲号、元信息和毫秒起止区间，不把区间编码进路径。播放服务内部可有执行结构，但不形成第二个用户可见队列。临时“默认”列表只存在于会话内，用于外部打开和手工添加。

文件夹导入由 Rust 文件系统边界递归解析为按路径稳定排序的音频项目，并为每项携带可选导入根目录；同名 CUE 在此边界展开为多个逻辑项目，同一物理路径不会合并这些曲目。扫描会规范化、去重且不跟随符号链接、junction 或 reparse point。用户列表把该根目录上下文随项目持久化，默认列表只在会话内保存。React 依据项目顺序和根目录生成可折叠目录段，并按列表维护会话级展开状态；目录节点不是可播放项目，折叠、展开或同一路径因手工排序形成多个连续段时都不得重排底层项目或播放序列。底栏定位动作读取 Rust 权威活动列表与当前逻辑曲目，只在用户明确触发时展开祖先目录、滚动并聚焦曲目行。

单个列表删除和“关闭其他列表”都由 Rust 协调持久化与当前播放序列；批量关闭在单个 SQLite 事务中保留目标用户列表或删除全部用户列表。若活动用户列表被删除，其现有执行序列一次性转移为默认列表，React 不逐项删除或推演中间状态。

## 元数据与封面

`MetadataService` 按路径、文件大小和修改时间缓存最多 4 首曲目，并按源字节指纹维护最多 8 个不同封面的 LRU。封面先取内嵌图片；缺失时有界扫描直属目录，依次选择 `cover`、唯一图片、多图中的小尺寸方图。候选只读 JPEG/PNG 图片头，选中后才完整解码；不同源图归一化为最大 512 px 的 PNG 与 BGRA，并携带归一化内容指纹。PNG 供 SMTC 和 React，BGRA 供 DWM。源字节、像素、目录条目和缓存均有上限；失败使用统一占位图。

播放列表通过 `MetadataService` 的独立轻量摘要分批读取标题、音轨号与时长；该路径关闭封面解析，批量请求有上限并使用小型 LRU，不占用当前曲目封面缓存。同一物理文件的 CUE 项共享摘要，React 只依据 Rust 已解析的逻辑起止区间形成曲目、目录段和整表合计；未知时长不得猜测。文件信息弹窗只在用户明确打开时读取完整标签与音频属性。文件管理器定位经 Tauri opener 的跨平台 reveal 能力封装在 bridge/command 边界，不向 UI 暴露 Explorer、Shell 或 Linux 文件管理器细节。

`NativePlaybackProjection` 合并权威播放快照、标签（CUE 逻辑曲目信息优先）和共享封面，低频发布给 SMTC、任务栏和托盘：

- SMTC 使用标题、艺术家、专辑、时长、时间轴、可用动作和真实封面；归一化指纹直接映射缓存文件，最近使用的不同封面最多保留 16 个，重复封面只占一个文件。带封面提交失败时重试占位图和无封面元数据。
- Linux MPRIS 在独立线程发布媒体属性，回调只排队操作；权威 actor 校验定位范围和 track ID，封面使用正确转义的本地 URI 与 16 项缓存。
- 任务栏工具栏固定上一首、播放/暂停、下一首；进度只在时长确定且可定位时显示。
- DWM iconic thumbnail/live preview 只绘制等比居中的封面或占位图。平台工作线程按归一化指纹复用当前 GDI 位图组；只有指纹变化时才预生成新的有界固定尺寸组，并在完整后替换旧组。窗口消息回调只选择不超过系统请求的最近缓存并立即提交，不在 Explorer 同步等待的回调内缩放、创建位图、读文件或投递补偿渲染。

## 歌词与窗口

歌词解析器只处理 LRC、SRT 和 VTT，返回平台无关的时间片/文本模型。桌面歌词极长行按当前行起止时间与播放位置从头到尾线性滚动一次，暂停冻结、定位同步。窗口由 Rust 服务管理位置、尺寸、置顶、穿透和 helper；Windows adapter 在显示、锁定、解锁、失焦、恢复和 DPI 变化时校正 topmost 与 helper 几何，并以 WinEvent 监听外部前台窗口、虚拟桌面和最小化恢复事件。回调只合并并投递无激活校正，资源隐藏时卸载 hook，不使用定时置顶。主窗口关闭、托盘隐藏与完整退出由 `ApplicationLifetimeService` 协调。

Win32/COM 回调必须快速返回，通过 channel 或缓存与服务交互。禁止全局鼠标 hook、持续轮询、输入注入或向其他程序转发输入。

## 压缩

`FfmpegDependencyService` 用同一 DTO/command 选择平台 adapter。Windows 管理固定归档的按需下载、SHA-256、安全解压、取消和同卷原子启用；Arch Linux 检查 `/usr/bin/ffmpeg` 和 `ffprobe`，缺失时提示系统包安装，应用不提权或下载。网络只由 Windows 用户明确下载依赖触发。

`CompressionService` 管理扫描与批次快照：

- 扫描用 `hound` 进程内验证 WAV 头，规范化/去重路径，不跟随链接类目录。
- 转换前用 `hound` 获取声道、采样率、位深和时长；转换后用 `lofty` 验证临时 FLAC，不启动 ffprobe 热路径。
- worker 数由可用并行度决定，保留响应余量并限制为 1–4；每个文件只有一个 FFmpeg 子进程和独立临时路径。
- React 依据 Rust 快照展开首次活动项目的祖先目录，当前批次内手动折叠优先；不自动滚动。固定汇总与主窗口工具入口共享处理进度投影，失败计入已处理但不计入成功体积；展示规则见 [ADR 0027](decisions/0027-compression-progress-and-close-dialog.md)。
- 终态批次保留醒目结果，前端和 Rust 均阻止重复启动。只有添加/移除/清空输入成功后重置任务；输入变更和 start 共用批次锁，阻止扫描/转换竞态，并重新识别已有输出冲突。结果中的删除数量只取实际 sourceDeleted。
- 每项维护单调进度、源体积、成功输出体积、状态与错误。取消会终止所有活动子进程并清理未提交临时文件。
- 已存在输出永不覆盖；验证后通过平台 adapter 原子提交（Windows `MoveFileExW` 不设置替换标志；Linux 同目录硬链接发布后清理临时文件），源删除只能发生在提交成功之后。部分失败不回滚已成功文件。
- 压缩窗口关闭时由服务在批次锁内区分当前任务与旧工作区：运行/取消中的批次保留并继续，其他状态连同扫描输入、统计、警告和错误恢复为初始值。窗口保持关闭时，当前批次进入终态后再清理；完成前重开会取消延迟清理。

## 持久化与更新

SQLite schema v7 只保存用户播放列表、列表项目（含可空导入根目录与 CUE 逻辑音轨字段）和应用状态。旧 `recent_plays`、`managed_folders` 和 `media_records` 会迁移删除；根目录上下文不建立媒体库或文件夹监视。偏好由 Rust 类型化存储；目录展开等窗口临时 UI 状态只留在前端会话。

普通窗口尺寸统一以逻辑内容区宽高保存，主窗口宽/窄模式各自记忆正常尺寸，切换期间过滤尚未到达目标的中间几何。Linux 通过 GTK 内容区 allocation 读取尺寸，避免 Tao 的装饰/阴影缓存反复累加；Windows 位置继续使用物理坐标。Arch Wayland 仅恢复尺寸、布局和主窗口最大化状态，全局位置归 compositor 管理。桌面歌词使用 layer-shell overlay 置顶、空输入区域穿透与原生 GTK 解锁按钮，KDE 媒体控制使用 MPRIS；托盘无宿主或创建失败时拒绝隐藏主窗口。具体能力边界见 [ADR 0028](decisions/0028-arch-linux-wayland-baseline.md) 和 [ADR 0029](decisions/0029-wayland-lyrics-and-mpris.md)。

更新检查由 `ApplicationUpdateService` 单飞协调，设置连接与总超时。支持的平台在应用首帧后每进程静默检查一次；只有发现更新才加载 Markdown 弹窗。仅接受更高 SemVer，tag 统一为 `v<version>`，使用 Tauri 默认版本比较器；完整清单用 Minisign 验证身份，安装请求绑定检查时的 tag、版本、URL 和签名，再由 Tauri updater 下载验签安装。Linux 复用相同检查/签名与 Tauri download 验签，通过 distribution/arch adapter 准备受限临时包和 Pacman 只读事务，用户确认后以固定 pkexec/Pacman 参数安装；安装前复核，完成后查数据库并提示重启。安装操作与转换/扫描互斥，生命周期拒绝活动安装期间退出。Arch 预览与正式版统一使用 `resona` 包名，按更高 SemVer 升级，不支持旧包名迁移；系统签名策略不变，详见 ADR 0032/0035。外链只允许 HTTP(S) 并经 Rust opener。

## 故障隔离

- 元数据、歌词、SMTC、任务栏、托盘、更新或材质失败不能阻止本地播放。
- 输出设备失效由播放服务显式报告并允许重选；不偷偷切换引擎或重采样策略。
- 数据库迁移在事务内执行并以旧 schema fixture 验证用户列表不丢失。
- 原生视觉与硬件行为必须在 Windows Release 中验证；浏览器只证明布局、状态、溢出和静态样式。

## 文件访问与窗口内快捷键

[ADR 0036](decisions/0036-native-file-access-and-focused-playback-controls.md) 将所选文件验证与可选目录发现分开；Linux 文件访问 adapter 解析 URI 并映射/刷新 KIO-FUSE 路径。KIO-FUSE 挂载禁止自动目录发现，避免枚举破坏文件映射；本地与内核挂载目录沿用正常 CUE、排序和外置封面逻辑。冷启动外部打开结果保留到前端监听完成后一次消费。

Linux 文件选择 adapter 直接使用锁定 ashpd 的标准 XDG portal，保留 URI、响应错误与真实 Wayland parent；Windows 使用 Tauri dialog 原生后端。选择与映射在后台边界执行。前端窗口内按键只提交有界意图，播放 actor 根据当前真实位置/音量处理相对操作并返回快照；压缩窗口不创建第二播放器。边界及验收见 [beta.1](plans/0.3.0-beta.1.md)。

长期决定见 [decisions/](decisions/README.md)。
