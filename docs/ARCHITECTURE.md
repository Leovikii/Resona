# 系统架构

## 原则

1. 成熟实现优先：播放使用 Rodio/CPAL/Symphonia，转码使用 FFmpeg，标签使用 lofty-rs。
2. Rust 后端是播放、临时/持久播放列表、最近历史、任务和持久状态的权威来源。
3. 音频库、外部进程、平台 API、数据库和 UI 框架均位于适配边界之外。
4. 边界用于控制变化和故障，不为了抽象本身增加层级。
5. 性能敏感路径以批量事件、虚拟列表和后台任务处理，避免高频跨 WebView IPC。

## 总体结构

```text
React windows
  main / desktop-lyrics / audio-compression
          |
   typed Tauri commands and events
          |
Rust application services
  playback / playlists / recent / lyrics / metadata / conversion
          |
domain contracts and models
          |
adapters
  Rodio audio / SQLite / filesystem / FFmpeg / platform windows
```

依赖方向只能从外向内：UI 和 adapter 依赖应用契约，领域层不依赖 Tauri、Rodio、CPAL、Symphonia、SQLite、Mantine 或操作系统 API。

应用服务按当前产品需求和变化边界拆分：播放协调、播放列表、最近历史、按需元数据、转换、歌词和设置相互独立。0.1.0 不建立独立产品播放队列、受管理文件夹或媒体库索引，也不建立没有明确需求的动态插件 ABI 或皮肤引擎。

## 前端边界

### 状态归属

- Rust：播放状态、当前播放列表来源、Rodio 内部执行序列、临时默认列表、持久化用户列表、最近历史、转换任务和持久设置。
- React hooks/context：窗口临时状态、选择状态、未提交表单和纯 UI 偏好；没有明确跨 feature 需求时不引入额外状态库。
- React component：生命周期很短且不跨组件共享的交互状态。

前端不得自行推断播放成功或长期维护一份与后端竞争的播放列表/执行序列。command 返回接受/拒绝结果，后端事件提供权威状态。

### Mantine 解耦

Mantine 是 UI 实现依赖，不是领域 API。采用三层策略：

1. `app/theme` 保存 Resona 设计令牌和 Mantine provider 配置。
2. `shared/ui` 保存具有统一行为或领域语义的组件，例如 `TrackRow`、`PlayerControls`、`AppDialog`。
3. `features` 组合项目组件，不传播 Mantine 类型到状态、服务或 Tauri 契约。

不包装纯布局和一次性展示控件。简单包装不能隔离真实变化，反而增加组件深度和维护成本。需要包装的判断条件至少满足一项：

- 统一默认行为、键盘或无障碍规则
- 统一错误、加载或空状态
- 屏蔽第三方类型或 API
- 承载 Resona 领域语义
- 隔离平台差异或已知缺陷

这些函数组件不会进入音频处理路径；避免深层 provider 和无意义包装即可保持 UI 开销可忽略。

### 应用壳、主题与本地化

- 根布局只有左侧导航、右侧主信息区和右侧底部播放器三个稳定区域，不建立永久第三列。
- 侧栏顶部为“最近播放”，软分隔后是带新增按钮的“播放列表”分组；固定“默认”子项位于用户列表之前，“工具”和“设置”固定在底部。
- 不再提供侧栏“正在播放”或“文件夹”路由。完整播放页是底部播放器触发的右侧主内容互斥视图，关闭后恢复原内容上下文；点击侧栏会退出完整播放器并显示目标页。该视图只消费权威 `PlaybackSnapshot`，不得重新创建播放器状态。
- 底部播放器稳定挂载。完整播放器只由左侧封面按钮展开/收起，封面 hover/focus 显示对应动作图标；右侧不保留重复入口。展开或退出时不能重新创建播放控制状态或改变队列；Mica 下完整播放器透出根窗口材质，底栏使用稳定的轻量 chrome tint 建立操作区层级，不以整宽 hairline 或白边切割页面；solid 回退使用对应的不透明 chrome/content token。
- 底栏右侧工具使用固定等距槽位，顺序为音质标识、播放模式、桌面歌词和音量；不再提供当前播放列表定位或桌面歌词锁定入口。音质分类由 Rust 元数据 DTO 给出，桌面歌词锁定状态或音量交互不得改变底栏网格，音量使用固定按钮和按需弹出的滑杆。
- 0.0.11 目标是将桌面歌词显示/隐藏作为播放运行时动作放到底部播放器；设置页只保存持久偏好。主窗口必须保留锁定状态下的解锁和隐藏恢复路径。
- Mantine 负责控件和主题渲染；业务组件使用语义 token，不直接依赖固定深色十六进制颜色。
- 普通窗口的系统材质由 Rust `WindowMaterial` platform adapter 决定。Windows 11 主窗口/音频压缩窗口可使用 Tauri 原生 Mica，Windows 10/Linux 回退实色；React 只消费 `mica`/`solid` 展示结果和共享表面 token，不读取系统版本或调用 Windows API。长期约束见 ADR 0022。
- Mica 是根窗口背景能力，不是第二套主题或组件库。主内容与完整播放器继续直接透出根材质；侧栏、窄屏顶部导航、底栏和普通辅助窗口命令区只允许一层低对比 chrome tint，以色调差异表达稳定区域，不使用整高/整宽 hairline 切割应用壳。工具入口等独立工作单元可使用一层 `subtle surface`，菜单、文字密集交互区、选中态和错误提示保持实体表面；禁止连续遮罩、全局 Acrylic、`backdrop-filter` 或逐行模糊层。设置分组使用标题和节间距，不使用分组外框、阴影或贯穿页面的分隔线。
- `UiPreferences` 只包含 `colorScheme`、`accentColor` 和 `locale` 等展示偏好。0.0.5 可用 localStorage 持久化，字段保持框架无关，以后可迁移到统一设置服务。
- 主窗口布局模式与窗口几何影响原生最小尺寸和首次可见帧，由 Rust/Tauri 在显示前恢复，不进入仅在 WebView 加载后可用的 localStorage。宽屏/窄屏复用同一 React 选择和领域 hooks，具体约束见 ADR 0020。
- 窄屏使用顶部全局导航和横向播放列表 Tabs；不创建 Drawer 或第二份 `Sidebar`。导航只改变既有页面选择，不复制路由、列表或播放器状态。底栏以两行直接展示曲目信息、进度与固定控制槽，不以“更多”菜单隐藏常用能力。
- 单行文本通过共享 `OverflowMarquee` 在实际溢出时滚动；底栏、侧栏等少量稳定元素可观察尺寸变化。播放列表曲目等大集合必须使用按需模式，只在当前文本收到指针交互时测量，不得为每行创建常驻观察器或持续动画。滚动动画只修改 `transform`/`opacity`，减少动态效果时退化为省略。
- 本地化使用 `i18next` + `react-i18next`。React 组件只读取翻译 key；Rust 返回稳定错误码和参数，不把固定语言文案当作前端契约。
- “工具”只是路由和能力入口；主窗口不直接承载压缩工作流。音频压缩使用按需创建的单实例普通 WebView 工作窗口，标签编辑后置。
- `audio-compression` 窗口只展示 Rust 权威扫描/任务快照并提交类型化操作；递归文件系统、FFmpeg 生命周期和文件写入均不得进入 React。该窗口使用独立最小 capability，不复用桌面歌词的透明、置顶、穿透或原生 helper 边界。
- 桌面歌词不复用普通窗口的 Mica/solid 表面实现。它继续使用专用透明 WebView 与 Win32 helper，只对齐字体、圆角、轻量按钮状态和动效语言，避免普通材质破坏穿透与透明度语义。
- 压缩目录树默认折叠子目录，每个展开节点最多增量渲染 200 个直属项，并使用浏览器 `content-visibility` 跳过离屏布局；这为大目录提供有界首帧成本，不为首版增加第二个列表组件库。实测不足时再评估成熟虚拟化依赖。
- 压缩扫描的 WAV 候选校验使用进程内 `hound` 读取 RIFF/WAVE 头；扫描阶段禁止按文件启动 FFmpeg/ffprobe。FFmpeg 只在用户开始转换后运行，ffprobe 只验证输入转换参数和临时 FLAC 结果；Windows 子进程统一使用无控制台标志。
- 所有用户可见的主窗口滚动轨道由 Mantine `ScrollArea` 提供；设置、侧栏、播放信息和窄屏内容不得回退为 WebView 原生滚动条。歌词可以隐藏轨道，但仍使用同一组件 viewport，以维持 ref 定位和键盘滚动。

## 播放边界

应用层依赖 `PlaybackEngine` 契约，首个且唯一的首版 adapter 为 `RodioPlaybackEngine`。UI、应用服务和 Tauri command/event 契约不得接触 Rodio、CPAL 或 Symphonia 类型。

核心职责：

- `PlaybackService`：内部执行序列、播放模式、命令序列、状态归一化和恢复策略
- `AudioEngineActor`：在专用 Rust 线程持有输出流、播放器和混音器，通过 typed channel 串行处理命令
- `RodioPlaybackEngine`：实现领域播放契约，配置 Rodio 播放、队列、定位、音量和结束通知
- `AudioDeviceManager`：通过 CPAL 枚举输出端点、选择默认/指定设备并在设备变化后重建输出
- `DecoderFactory`：限制并配置 Symphonia 解码能力，首版强制 MP3/WAV/FLAC，AAC/M4A 可选
- `PlaybackClock`：生成权威播放位置并按 UI 实际需要节流

0.0.2 的 `PlaybackSnapshot` 对外提供 `positionMs`、`durationMs`、`volume` 与 `seekable`；定位和音量仍通过 audio actor 串行执行。Tauri 错误边界使用稳定 `code` + `message`，UI 不依赖 Rodio/Symphonia 异常文本。

Slider 拖动、轨道点击和完整播放器歌词点击统一进入一个前端 seek transaction，再调用既有 `seek_playback` command。提交后本地目标值保持到对应权威快照接管；连续定位使用递增事务标识忽略迟到结果，不能让旧快照造成视觉回跳。该交互状态只限播放器区域，Rust 仍是最终位置权威来源。

音频回调不获取应用全局锁，不执行文件、数据库或 WebView IPC。解码、设备恢复和队列准备在 Rust 后台完成；音频样本不经过 Tauri IPC 或 WebView。前端在后端时间锚点之间做视觉插值。

Rodio 使用精确锁定的稳定版本和最小 feature 集。首版不实现 mpv fallback，也不同时维护第二套播放路径。

当前配置使用 Rodio 的 Symphonia `flac`、`mp3`、`wav` 与 CPAL `playback` features。WAV 的 32-bit integer/float 和 FLAC 的 16/24-bit 已进入自动矩阵；Rodio 0.22.2 的 FLAC adapter 对合法 32-bit FLAC 暂不产生样本，不能宣称该组合已支持。

### 0.0.3 队列策略（提案）

- Rust actor 继续独占一个 `Player` 和一个输出 mixer；切歌不重建输出流。
- 播放模式切换只更新下一曲选择策略，不停止、定位或重建当前 source。若顺序/列表循环已预载下一曲，actor 通过原子状态只取消尚未开始的预载 source；已经开始输出的 source 不可被模式切换中断，避免硬切波形导致爆音。
- 队列服务持有稳定的 `QueueItemId`、路径、当前索引和错误状态，UI 不维护第二份权威队列。
- 当前曲目开始播放时，至少提前解码并 append 下一曲；使用 Rodio 公开的 `source::Done` 与每曲独立原子标记识别结束，再推进当前索引和快照。
- 只在有预载曲目时宣称 lossless WAV/FLAC 的连续无插入静音；队列已经空后追加不承诺 Gapless，因为 Rodio 0.22.2 自身仍有对应 ignored 测试。
- `try_seek` 只作用于当前曲目；跨曲定位、总队列时间和拖拽排序不进入本版本。
- 单曲解码失败标记队列项并继续下一项，不终止 actor 或污染其他项。
- 结束检测不在音频线程发送 channel 或执行 UI 回调；Rodio `Done` 只递减原子计数，audio actor 在既有 tick 中读取标记。

### 当前播放列表与内部执行序列（长期约束）

- 0.1.0 的播放列表就是用户可见的待播放序列，不提供独立可编辑 `PlaybackQueue`。`DefaultPlaylist` 是未保存的临时列表，`UserPlaylist` 是 SQLite 持久化的已保存列表。
- 当前列表来源编码为 `Default` 或 `User(id)`。点击列表曲目激活对应列表；只浏览侧栏列表详情不改变当前来源。
- 当前列表的追加、插入、删除和排序由 Rust 服务完成，并立即同步 Rodio actor 的内部执行序列；编辑非当前列表只改变该列表自身。React 不提交完整路径数组来替换播放序列。
- Rodio actor 内部继续持有稳定运行时 ID、当前索引、预载 source、单曲错误和随机遍历状态。该序列服务于音频执行、SMTC 和诊断，不作为第二个产品集合暴露。
- 随机播放不改变播放列表顺序，只在 actor 内选择下一项并维护必要历史。未来只有在真实媒体库、专辑/搜索结果或临时插播需求出现时，才通过新 ADR 增加独立队列协调层。
- 删除当前持久化列表时，现有路径序列转移为默认临时列表。默认列表为纯会话内容，正常退出后清空；启动只恢复音量、播放模式与输出设备选择，详见 ADR 0019。
- `RecentPlay` 是有限长度播放事件历史，不是列表或媒体库。0.1.0 不保存受管理文件夹、媒体记录或封面索引；`metadata.rs` 只为当前曲目或明确列表展示按需解析。
- 文件路径、显示名和运行时序列 ID 通过类型化契约传递；Rodio、CPAL、Symphonia、SQLite 和 Tauri 类型不能泄漏到前端或领域模型。

初次进程参数、Tauri single-instance 参数和应用内打开必须映射到同一 `open_media_context` typed request，不能在平台回调中直接操作 Rodio。

## 平台能力

平台差异由能力接口表达，而不是假设所有平台都支持相同行为：

```text
AudioOutputCapabilities
DesktopLyricsCapabilities
GlobalShortcutCapabilities
```

Windows 首版通过 CPAL 实现 WASAPI Shared 和桌面歌词窗口。板载声卡与蓝牙均视为系统输出端点；设备断开、默认设备变化和休眠恢复触发输出流重建。未来 Linux adapter 可以报告 Wayland/PipeWire 实际能力并降级，通用 UI 根据 capability 显示可用功能。

`DesktopLyricsWindowService` 当前已经构成稳定 facade：通用 command 和 UI 只接触 snapshot、failure 和 capability，Windows 原生实现位于 `platform/desktop_lyrics/windows.rs`。在只有一个真实实现时不额外建立动态 backend trait；未来 Linux 开发开始后，在 facade 后增加 `linux.rs` 并根据 Wayland 实际能力实现或降级，`cfg` 只允许出现在 platform 组装边界。

### 桌面歌词窗口边界

`DesktopLyricsWindowService` 统一管理桌面歌词可见性、锁定状态和窗口生命周期。歌词内容由独立的轻量 Tauri WebView 展示，继续消费 Rust 权威 `LyricsSnapshot`；它不是 sidecar 进程，也不加载媒体库或建立第二套播放控制状态。

歌词 WebView 创建时保持隐藏，URL 在 HTML 预绘制阶段同步声明透明窗口；前端读取偏好并完成首个 `NowPlayingSnapshot` 后，通过幂等 ready command 请求原生窗口显示。逻辑可见性和首个物理显示分离，关闭或快速切换时 ready command 不得复活已隐藏窗口。

Windows WebView2 采用 renderer/GPU/utility 多进程模型，第二个歌词 WebView 可能增加子进程和内存，但不会复制 Rust `RodioPlaybackEngine`、队列或歌词服务。0.0.11 目标是让主窗口成为 Windows 版本的应用生命周期所有者：关闭主窗口协调停止播放、销毁歌词 WebView 和原生 helper 并退出应用；没有新 ADR 时不得隐式转为后台播放或关闭到托盘。重复启动计划通过 Tauri 官方单实例能力回到现有主窗口，避免真正创建第二套应用状态。

Windows 锁定通过 Tauri 官方整窗鼠标穿透实现。由于穿透窗口无法接收自身 hover，解锁热点由一个不含 WebView 的微型原生 owned tool window 承担；它与歌词窗口的所有权、位置、置顶和销毁顺序在 Windows platform adapter 内处理。辅助窗口创建或显示失败时不得进入锁定，主窗口始终保留解锁和隐藏入口。

0.0.11 在隐藏桌面歌词时停止窗口快照轮询并释放歌词 WebView/helper 资源，再次显示按需重建；解锁状态的 WebView 控制条显示在歌词上方，锁定状态的原生 helper 保持可命中但使用近透明 alpha=1 的空闲绘制，通过局部 Win32 mouse leave 消息在 hover 进入/离开时显示或隐藏，不引入全局监听。完全透明 alpha=0 在部分 Windows 环境下可能不产生首次命中事件。淡入淡出动画属于后续视觉微调，不是本阶段正确性条件。

桌面歌词的后续视觉收口不改变窗口输入边界：未锁定 WebView 可以用 CSS hover 把背景临时切换到统一交互透明度，离开后恢复偏好；锁定 WebView 不能接收 hover，整窗背景保持原值，只允许原生 helper 的局部 hover。文字透明度是独立偏好，不与背景透明度复用。歌词切换在稳定两行槽内只使用 `transform` 和 `opacity` 做一次上滚；跨多行 seek、revision 变化和 `prefers-reduced-motion` 直接定位，不能逐行追赶或让动态内容改变窗口尺寸。

歌词活动行采用音乐播放器时间轴：第一行开始前为空，之后最近已开始的歌词持续到下一行开始；SRT/WebVTT 的 cue 结束时间只作为源信息保留，不控制主播放器或桌面歌词消失。主播放器保持该行高亮并自动定位，桌面歌词只在新行开始时滚动当前/下一行槽。

0.0.10 实现确认 Tauri 2.11.5 的无 WebView `WindowBuilder` 仍要求 `unstable` feature。当前不启用该 feature，改由 Windows platform adapter 在 Tauri 主事件线程创建最小 raw Win32 owned tool window；通用服务和 UI 仍只接触类型化状态与 command，HWND、窗口类、GDI 绘制和消息处理不会越过 platform 边界。

不使用 WebView2 局部命中、第二个解锁 WebView、全局鼠标 hook、持续鼠标轮询、输入注入或向其他程序转发输入。0.0.10 先验证窗口组合和跨进程穿透；持久化、多显示器完整恢复与最终视觉只有在原型通过后实施。长期决定见 [ADR 0012](decisions/0012-desktop-lyrics-window-and-unlock-helper.md)。

### 系统媒体会话边界

`MediaSessionAdapter` 是平台 adapter：Windows 实现映射到 SMTC，未来 Linux 可以映射到 MPRIS。系统回调只产生 typed playback command，不直接接触 Rodio、队列容器或 WebView。播放 actor 通过可合并状态发布通道提供曲目、播放状态和低频时间轴更新；Windows 类型、句柄和回调不得进入领域模型。

0.0.6 已锁定 `souvlaki 0.8.3` 作为 Windows adapter；它使用 MIT 许可证并关闭默认 D-Bus feature。若后续实机发现 Windows 行为、许可证或维护性不符合要求，再以新的 ADR 评估 `windows` crate 实现同一 adapter。

## 歌词边界

`LyricsService` 独立于 Rodio 播放 actor，负责同名 sidecar 发现、文本解码、LRC/SRT/WebVTT 解析、统一时间行和当前行选择。`lrc`、`subtp` 与 `encoding_rs` 类型在 `lyrics.rs` 内终止；UI 只接收 `LyricsDocument`、revision、状态和当前行索引。

应用层组合 command 同时读取权威 `PlaybackSnapshot` 和歌词快照。前端沿用 750 ms 低频刷新，不自行推导歌词时钟；同一 revision 不重复传输歌词全文。音频线程不读取歌词文件、不解析文本，也不发送 WebView 事件。LRC 优先于 SRT，SRT 优先于 WebVTT；同格式内依次选择精确基础 stem、完整音频文件名、带格式/语种限定名的稳定排序候选。字幕布局和样式统一降级为纯文本歌词行。

## 数据与文件安全

- SQLite schema 通过版本化 migration 演进。
- 按需元数据读取、标签写入和音频压缩不得阻塞 Tauri 主线程。
- 压缩文件夹扫描在 Rust 后台执行，规范化并去重路径；Windows junction、符号链接和 reparse point 默认不跟随。单项权限或读取错误形成结构化警告，不能中止其他根路径。
- 扫描只把扩展名候选交给进程内 WAV 头验证边界，最终任务只接收验证通过的 PCM/32-bit float WAV 路径快照。目录结构 DTO 只描述导入根、相对路径、节点类型和诊断，不成为媒体库索引。
- 文件写入默认使用临时文件、验证后原子替换。
- 转换不得覆盖已有 FLAC，也不得把有损来源标记为无损提升。“成功后删除源 WAV”虽默认开启，但必须逐批确认，并且只在对应 FLAC 校验和原子提交成功后执行。
- 跨边界错误使用稳定错误码和可诊断上下文；内部日志保留原始原因。

## 故障隔离

- 播放 actor 意外退出时由服务监督并进入可诊断失败状态；可恢复时重建输出，不静默继续。
- 单曲解码失败只跳过或停止当前曲目，不污染队列、默认列表或用户播放列表。
- FFmpeg 异常退出只使对应转换任务失败，不应终止主应用。
- `CompressionService` 是 WAV 到 FLAC 任务状态的权威来源；WebView 只提交路径、档位和经确认的删除意图并轮询类型化快照，不拼接 FFmpeg 参数。固定 sidecar、临时文件、ffprobe 验证、原子提交和源文件删除顺序全部在 Rust 内终止。
- 关闭或重建 `audio-compression` WebView 不得丢失活动任务状态；重新打开从服务快照恢复。退出主应用时必须协调取消活动扫描/转换、终止 sidecar 并清理临时文件。
- 单个损坏标签只使当前按需元数据不可用，不阻止播放或污染其他曲目状态。
- 单个窗口渲染失败不改变播放服务状态。
- adapter 错误不得把第三方异常类型扩散到领域层和 UI。

## 播放依赖持续性

- Cargo.lock 是实际版本的权威记录；Rodio、CPAL、Symphonia 大版本不自动升级。
- 用固定音频 fixture 验证版本升级，覆盖格式、位深、采样率、Gapless、定位和坏文件。
- 上游停止维护本身不触发迁移；只有出现无法修补的阻塞缺陷或平台不兼容时才评估替换。
- 替换顺序为：固定/修补现有版本、项目 fork、直接 CPAL + Symphonia、GStreamer adapter、mpv adapter。
- 任何替换都通过新的 ADR 决定，产品中不预装未使用的备用引擎。
