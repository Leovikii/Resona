# 开发状态台账

最后更新：2026-07-23

### 2026-07-23 0.1.0-rc.1 正式更新密钥与首次发布准备

- 项目所有者已生成、离线备份正式 Tauri updater 密钥，并在 GitHub `release` Environment 配置 `RESONA_UPDATER_PUBLIC_KEY`、`TAURI_SIGNING_PRIVATE_KEY` 与 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`；预览版和后续正式版沿用同一信任链，密钥未进入仓库。
- 修复桌面歌词干净初始化未采用约定默认值的问题。根因是缺失的 localStorage 数值被 `Number(null)` 解析为 `0`，再归一化成 `16px / 10% / 0%`；现明确把缺失、空白和非法值回退为 `30px / 100% / 20%`，已有合法用户偏好保持不变。CSS 首帧字号兜底同步为 `30px`。
- 主窗口系统标题统一为不含版本号的 `Resona`。前端预览/“关于”版本兜底改为直接读取 `package.json`，FFmpeg 下载 User-Agent 改读 Cargo 包版本；运行时代码中的 `0.0.19` 残留已清理，版本继续保持 `0.1.0-rc.1`。
- 发布判定允许“当前版本尚无 `v<version>` tag”时由首个合并到 `main` 的 PR 建立该版本 Release；tag 建立后，同版本后续合并只运行 CI、不重复发布。这为现有未发布的 `0.1.0-rc.1` 提供一次性自动启动路径，不增加 push、tag 或手工发布旁路。发布端新增完整 SemVer 优先级门禁，覆盖 alpha、beta、rc、数字标识和正式版，拒绝版本回退及仅改变 build metadata 的伪升级。workflow 以 `cancel-in-progress: false` 和 `queue: max` 对 `main` 合并交付串行化并保留等待任务；`actions/checkout v7.0.1` 与 `actions/setup-node v6.4.0` 均固定到当前稳定版完整 SHA，并运行在 Node 24。
- 自动验证通过：11 个前端偏好/发行规则测试、TypeScript/Vite production build、许可证生成、Rust format、82 个普通 Rust 测试（9 个设备/FFmpeg 测试按约定忽略）、Clippy `-D warnings` 和差异检查。浏览器干净来源实渲染确认设置页首次值为 `30px / 100% / 20%`，“关于”为 `0.1.0-rc.1`，文档标题为 `Resona`。

### 2026-07-23 0.1.0-rc.1 自动开发完成，进入项目所有者原生验收

- npm、Cargo 和 Tauri 版本统一为 `0.1.0-rc.1`；主窗口标题现统一为不含版本号的 `Resona`。设置“关于”现使用 GitHub 原生 Releases API 主动检查更新，稳定通道排除所有 prerelease，预览通道按完整 SemVer 接受 alpha、beta、rc、其他合法 prerelease 及更高正式版；下载、签名验证、进度、取消和安装由 Tauri 官方 updater 完成。
- `.github/workflows/main-merge-delivery.yml` 只响应已合并到 `main` 的 PR。工作流检出精确 merge commit，Action 固定到 Node 24 版本的完整 SHA，运行 Node 26、前后端门禁和签名 NSIS 构建，再由 SemVer 自动创建 GitHub prerelease 或 stable Release；不提供 push、tag 或手工发布旁路。
- 隐藏 WebView 现暂停播放、桌面歌词、压缩和 FFmpeg 依赖状态轮询，重新可见时立即同步 Rust 权威状态；详见[性能与音频审计](performance/0.1.0-rc.1.md)。Rodio/CPAL/Symphonia、`150ms` actor tick 和 `500ms` 原生播放投影保持不变，没有加入 DSP、额外增益或未经证据的重采样。
- 新增 Rust 本地诊断日志：只接受 `resona`/`resona_lib` 自身模块 target，Info 及以上，`256 KiB` + `KeepOne` 轮转，位于系统规范的应用日志目录；不上传、不转发 WebView 控制台、不记录媒体完整路径。普通卸载沿既有钩子清理日志，更新安装保留。
- 英文根 README、中文 README、精简根 `AGENTS.md` 和 `docs/AGENT_GUIDE.md` 已完成。过期根 `CHANGELOG.md` 与 Git 跟踪的 Mantine 快照被移除；完整 `llms-full.txt` 已保留在本机 `.local-docs/` 并由 Git 忽略，Agent 指南明确其检索入口。
- 签名 NSIS 测试产物、`.sig`、版本元数据、`latest.json`、分发审计和原生主窗口门禁已通过。当前测试安装包为 `Resona_0.1.0-rc.1_windows_x64-setup.exe`，6,007,296 bytes，SHA-256 `6D6922330989F977D32BA78F133FF87958B7E57E839FF3A89C08C5FA82DB6E94`；它只使用一次性本地 updater 密钥，不得作为正式 Release。浏览器视觉回归覆盖深浅色、`1080×700`、`800×600`、更新卡片和安装确认弹窗，控制台无 warning/error。
- 自动门禁结果：前端 production build、许可证清单、Rust format、Clippy `-D warnings`、82 个普通 Rust 测试、9 个真实音频设备/FFmpeg 测试和 3 个发行通道测试全部通过。10 次 release 主窗口就绪中位数 `259.0ms`；60 秒空闲父进程 CPU `0.016s`，Working Set 平均/峰值 `30.26/30.27 MiB`。
- 正式 updater 密钥及 GitHub `release` Environment 已由项目所有者配置。剩余工作为完成一次 GitHub Release 托管的真实覆盖更新，以及播放/托盘/桌面歌词资源采样和多显示器/DPI 原生矩阵；当前本地测试密钥不会进入仓库或正式发布。

### 2026-07-23 0.0.19 Windows 验收完成与阶段切换

- 项目所有者确认 SMTC 曲名与品牌身份、任务栏控件、托盘/退出、设置“关于”、FFmpeg GitHub 下载、安装分发和其他 0.0.19 功能正常；最终浅色/深色任务栏图标修订也已通过，0.0.19 正式完成。
- 桌面歌词初始化默认值明确为字号 `30px`、文字透明度 `100%`、背景透明度 `20%`；已有用户偏好保持不变，不进行强制迁移。
- 项目所有者实机确认 Windows 会为任务栏缩略图控件绘制灰色交互表面，因此移除图标内重复的灰色底板。上一曲、播放/暂停、下一曲现使用透明背景、最大化白色符号和细深灰外轮廓，在黑白背景下保持可读且不会出现双层底板。
- SMTC 媒体标题改读 Rust 权威队列项显示名；封面元数据提交失败时自动降级重试无封面元数据，只有成功提交才缓存，避免错误显示 AppUserModelID 或因封面失败而不再刷新曲名。
- 托盘原生菜单移除检查更新、缩短入口文字，并把过长曲名按 Unicode 安全截断到 10 个字符。Windows 原生菜单宽度由最长菜单项决定，不能设置固定像素宽度；严格限制标题长度可使菜单保持稳定窄宽。独立媒体浮窗仍是后续可选平台窗口。
- 设置新增“关于”分区，显示运行时应用版本、检查更新、GitHub 仓库和版权；两个外链经窄 Rust command 使用官方 Tauri opener 打开。检查更新当前打开 GitHub Releases，自动下载与签名安装归入 0.1.0-rc.1。
- MP3/WAV/FLAC 文件图标彻底移除小尺寸不可读的格式文字和底部标签，只用蓝/金/紫三种主体色区分类型，并重生成 ICO；工具页依赖提示保持“需下载 FFmpeg”。
- FFmpeg 运行时、真实转换测试和许可记录统一使用不可变 GitHub Release 资源 `GyanD/codexffmpeg`，彻底移除 gyan.dev 直链；仍固定版本并校验归档及双二进制 SHA-256。FFmpeg.org 只发布源代码，其 Windows 下载页列出该构建提供方。
- 第二轮反馈修正版安装包为 `Resona_0.0.19_windows_x64-setup.exe`，5,403,972 bytes，SHA-256 `3600976E20BCD347843C46AEDADE1F87FC425DF93C4B560672FB9A1A4DBAE98B`，未签名。
- 当前自动验证：76 个普通 Rust 测试通过，9 个真实音频设备/显式 FFmpeg 转换测试按约定忽略；TypeScript/Vite、许可证、Rust format 和 Clippy `-D warnings` 通过。浏览器实渲染确认 `1080×700` 英文工具/关于和 `360×600` 中文工具/关于无横向溢出，控制台无 warning/error。NSIS Release、Windows 分发审计和原生主窗口可见性门禁通过；安装版 Shell/SMTC 与最终任务栏图标已由项目所有者验收。
- 下一阶段为已接受的 [0.1.0-rc.1 Windows 发布候选收尾](plans/0.1.0-rc.1.md)：原 0.0.20 更新器与发行加固范围完整归并到 RC，只允许更新/签名/发布链、诊断与性能审计、遗留原生矩阵和缺陷修复，不新增产品功能。

### 2026-07-23 0.0.19 Windows 深度集成、托盘与 NSIS 实现完成

- [0.0.19 计划](plans/0.0.19.md)、[ADR 0023](decisions/0023-windows-shell-media-integration.md) 和 [ADR 0024](decisions/0024-windows-tray-distribution-and-update.md) 已接受、实现并通过项目所有者 Windows 安装版验收。
- 修复默认/用户播放列表双击切歌的整页和底栏闪烁：即时播放不再修改全局 `pending`，最后一次请求胜出；800ms 人工延迟浏览器回归确认请求期间底栏按钮可用、几何固定、无入场动画，完成后只局部更新曲名。`360×600` 紧凑工具/设置页横向溢出为 0。
- SMTC 补入品牌封面，并通过进程、NSIS 快捷方式和文件关联统一 `io.github.vki.resona` 身份。Windows 任务栏新增上一曲、播放/暂停、下一曲缩略图按钮、正常/暂停/失败进度映射和 `TaskbarButtonCreated` 恢复；托盘提供显示、曲目、播放控制、桌面歌词和完整退出。三者只订阅同一个 500ms Rust 原生播放投影。
- 关闭主窗口默认询问“退出 / 隐藏到托盘”，带“不再询问”；设置提供“每次询问 / 隐藏到托盘 / 退出应用”三态 Rust 持久偏好。最小化仍是普通最小化。活动扫描或压缩退出会显示主窗口二次确认，完整退出统一保存几何/播放偏好并协调关闭转换和桌面歌词。
- FFmpeg/ffprobe 不进入安装包。工具页显式下载固定 FFmpeg 8.1.2，Rust 服务执行流式进度、取消、归档/双二进制 SHA-256、安全解压和同卷原子启用，依赖就绪前不能打开压缩窗口。数据库、窗口状态和依赖统一使用 Local AppData；旧 Roaming 文件执行无覆盖迁移。
- 完成品牌化 Tauri 官方 NSIS `currentUser` 一键安装器，安装到 `%LOCALAPPDATA%\Programs\Resona`，注册 MP3/WAV/FLAC 和格式图标。更新覆盖保留状态；非更新卸载精确清理 Resona 安装数据、Roaming/Local AppData 和注册表，不遍历或删除用户音频。产物名包含平台/架构。
- 首轮内部测试安装包：`Resona_0.0.19_windows_x64-setup.exe`，5,323,432 bytes，SHA-256 `AF23F65AEABB24E1EB11C6C5BFB5B1F1D703B784A0D39C19F3475FB39E3543EE`，未签名。该记录已由顶部的验收反馈修正版产物取代；更新下载/安装归入 0.1.0-rc.1，采用 Tauri updater、GitHub Releases 静态清单和独立 updater 密钥。

### 2026-07-23 0.0.19 播放列表激活导航修复

- 修复默认列表非空时，从任意用户播放列表双击曲目都会把详情页强制切回默认列表的问题。根因是主界面曾以“默认列表已有修订且当前路径存在”推断默认列表正在播放；该条件在用户列表开始播放时同样成立，与 Rust 返回的权威 `activePlaylist` 状态冲突。
- 删除上述重复推断。外部打开媒体和双击默认列表曲目仍通过既有 `openSequence` 导航到默认列表；双击用户列表曲目只切换权威当前播放列表和 Rodio 执行序列，不再改变正在浏览的用户列表页面。
- Mantine `ScrollArea` 的可见滚动条在停止滚动 `700ms` 后改为通过 `180ms` 透明度过渡淡出，而非直接移除节点；隐藏状态禁用指针命中，不改变滚动容器或内容几何。
- `npm run build`、`npm run licenses`、Rust format、Clippy 和差异检查通过；`npm run release:windows` 完整通过并验证主窗口可见。产物为 `src-tauri/target/release/resona.exe`（18,003,456 bytes，SHA-256 `0EE6BB15052480697EB2F982FC3366B5C7E677229990619B4F2F1AF92D894383`）。当前内置浏览器无法连接本轮本地 Vite 端口，因此跨列表双击和淡出观感保留为项目所有者 Windows 原生验收项，未误记为浏览器通过。

### 2026-07-23 0.0.19 界面与播放状态一致性修复

- 工具页删除音频压缩入口的重复范围副标题及双语死文案；设置页输出设备组改为最大 `320px` 的弹性布局，设备选择框负责收缩，刷新按钮保持固定尺寸。宽屏、`360×600` 窄屏及中英文状态均确认页面横向溢出为 0，刷新按钮始终在内容区内。
- 主窗口、压缩窗口与窄屏播放列表所有可见 Mantine `ScrollArea` 改为仅滚动时显示轨道，停止滚动 `700ms` 后自动隐藏；明确不显示滚动条的歌词容器继续使用 `type="never"`。浏览器检查确认滚动中轨道为 `visible`、静止后为 `hidden`，不再常驻占用视觉空间。
- 完整播放器删除歌词列表顶部的半屏占位，首行距歌词面板顶部固定为 `8px`，后续行仍通过容器内 `scrollIntoView` 居中；播放信息标题列收紧为 `96px`、间距 `8px`，值列建立完整收缩边界。中英文宽屏均无横向溢出，中文输入、输出与源文件示例可完整显示。
- 修复顺序模式在单曲或队尾点击下一曲时的状态分裂：`NothingLoaded` 仍作为可诊断错误提示，但不再把仍在播放的权威快照改成 `Failed`；解码、输出等真实失败路径保持原语义。新增无音频设备依赖的 actor 回归测试覆盖当前曲目、队列状态和错误码。
- `npm run build`、`npm run licenses`、Rust format、Clippy 和完整测试通过；68 个普通 Rust 测试通过，8 个真实音频设备测试按约定忽略。浏览器回归覆盖 `1080×700` 中英文工具/设置/完整播放器、`360×600` 英文设置和 `720×500` 中文音频压缩窗口。

### 2026-07-23 0.0.19 音频格式文件图标资源预备

- 新增 `assets/resona-file-flac.svg`、`assets/resona-file-wav.svg` 与 `assets/resona-file-mp3.svg`，分别使用紫色、金色和蓝色主体，复用标准 `R` 路径、透明播放字腔和黑色投影；格式标签以统一的白字深色描边覆盖 `R` 的下半区，不增加边框或色块，并将包含描边的文字框收进前景 `R` 的安全轮廓，避免文字越过任一边缘。
- 新增 `assets/FILE_ICONS.md`，记录格式、颜色和后续多尺寸 `.ico` 栅格化约定。资源保持透明背景，标签作为 SVG 图稿的一部分，不依赖运行时字体或前端组件。
- 浏览器实渲染已覆盖深色/浅色背景下的大尺寸预览及 `64×64` 小尺寸；三种图标均无资源错误，字腔和格式标签可辨识，标签文字使用高对比度大字号。当前只准备资源，不注册 Windows/Linux 文件关联；安装器、ProgID/MIME 映射和图标缓存刷新留到下一个发行阶段。

### 2026-07-23 0.0.19 品牌图标定稿与应用接入

- 项目所有者确认根目录 `assets/resona-gothic-wordmark.svg` 为最终横向字标，宽屏侧栏与窄屏顶部导航均以该资源替换旧的图标加普通文本组合；`assets/resona-r-mark.svg` 的单字母 `R` 作为应用图标设计。两者均为项目内自绘路径，不依赖外部字体文件或新增字体许可证。
- 单字母图标使用紫色哥特体 `R` 主体、黑色错位投影和透明播放三角字腔；完整 `Resona` 字标复用完全相同的 `R` 路径、紫色主体与黑色投影。前景两段路径在内部连接处重叠 `2` 个设计单位，消除深色背景下露出黑色投影的抗锯齿接缝，不改变外轮廓或透明字腔。最终 SVG 已在深色背景和主界面宽屏布局中完成浏览器实渲染检查。
- `assets/resona-icon.svg` 已切换为最终单 `R` 设计，并以紧贴字形上下边界的正方形 `viewBox` 消除无效外圈留白，在不拉伸或裁切的前提下提高任务栏和小尺寸图标可读性。Tauri 官方图标生成器已更新 `src-tauri/icons/` 的 Windows、Linux/default、Appx、ICNS、Android 与 iOS 多尺寸资源；`32x32.png`、`128x128.png` 与 `icon.png` 均保留透明播放字腔。本条记录形成时 0.0.19 尚为 Proposed；其后续完成状态以本页顶部为准。
- `src-tauri/build.rs` 显式声明 `icons/icon.ico` 为 Cargo 构建输入，避免图标更新后 Windows 资源库继续复用旧图标。`npm run release:windows` 已重新生成 `src-tauri/target/release/resona.exe`（18,003,456 bytes，SHA-256 `59593BED37E222CB8886B0C305F21D6F86EB670B3AEC8FB04CA9C122EA2343BD`）；从 EXE 提取的嵌入图标已确认是放大后的最终 `R`。项目可见主窗口探针仍因当前命令环境无法显示有效桌面窗口而超时，未将该项记为通过。

### 2026-07-22 主题色预设收口与 0.0.18 完成

- 主题色预设收束为 Mantine 原生 `violet`、`blue`、`green`、`pink`、`yellow` 五项，默认改为 `violet`；旧的 `cyan`、`teal`、`orange` 偏好不在允许列表中时自然回退为默认色，不增加兼容迁移代码。
- Mantine provider 统一使用浅色 `primaryShade: 7`、暗色 `primaryShade: 5`、`autoContrast: true` 和 `luminanceThreshold: 0.3`。黄色 filled 控件自动使用深色前景，五种预设均通过中英文无障碍名称展示；语义警告/错误色不绑定主题色。
- 浏览器主题回归覆盖五种颜色、浅色/深色、宽屏/窄屏设置页和主要 filled 控件；`360x600` 与 `1280x720` 均无横向溢出，控制台无 warning/error。前端生产构建、许可证、Rust format、Clippy、67 个普通 Rust 测试与差异检查通过，8 个真实音频设备测试按约定忽略。
- 最终 Windows Release 已通过 Tauri 生产构建链生成：`src-tauri/target/release/resona.exe`，18,006,528 bytes，SHA-256 `961CB7D5F9E1225ECB18948CF85EC458003E69CBE4D04C370FB17DC3C14F5144`。当前命令环境无法访问有效的交互桌面，原生可见窗口探针只枚举到 Tauri single-instance IPC 隐藏窗口，因此本轮没有把该探针记为通过；此前同一 UI/Mica 基线的 Windows 原生截图验收仍有效，本次只改变主题预设、对比配置和版本信息。
- 0.0.18 现正式完成并冻结功能 UI；下一阶段为 Proposed 的 0.0.19 Windows 分发与应用身份，范围仍包括图标、安装包、文件关联、SMTC/Shell 身份及经 ADR 确认的托盘/关闭语义。

### 2026-07-22 纯 Mica 外壳与单一内容层收口

- 根据项目所有者对上一版原生截图的反馈，再次收敛 ADR 0022：标题栏、宽屏侧栏、窄屏顶部导航和底部播放器直接透出同一根 Mica，右侧只保留一个圆角 `content surface`。Mica 下 chrome/player token 改为透明；深色 content/subtle 为 `18%` 黑与 `6%` 白，浅色为 `72%` 白与 `52%` 白，solid 回退仍使用完整不透明主题表面。
- 播放列表标题区取消独立 subtle 卡片，普通曲目行移除 Mantine `withBorder` 和常驻背景，列表间距收紧；只有 hover、选中、播放中及拖放位置反馈上色。工具入口仍作为真正独立工作单元保留无描边 subtle surface，设置页与完整播放器继续复用单一内容层，不增加嵌套卡片。
- 浏览器实际验证覆盖 `1080×700` 深浅宽屏播放列表、工具、设置和完整播放器，以及 `360×600` 深浅窄屏列表、工具、设置和完整播放器。全部页面横纵溢出为 0，窄屏底栏全部按钮位于可视范围内，控制台无 warning/error；前端生产构建、许可证、Rust format 和差异检查通过。
- Windows Release 的 DPI-aware 原生截图确认侧栏与底栏连续使用 Mica，主内容为唯一上色大区域；当前桌面采样为标题栏 `#1E2025`、侧栏 `#241E1E`、主内容 `#1F1915`、底栏 `#251E1B`，侧栏/底栏的轻微差异来自 Mica 桌面采样而非 CSS tonal block。截图位于已忽略的 `src-tauri/target/release/resona-native.png`。
- `npm run release:windows` 已在可访问交互桌面的构建环境中完整通过，主窗口稳定可见门禁成功。产物为 `src-tauri/target/release/resona.exe`，18,006,528 bytes，SHA-256 `6EBD61A60A125A78B65B9B44AB073695F7DC227482877E0769AFFB9B02BCCA07`。

### 2026-07-22 WinUI tonal surface 与区域分隔收口

- 项目所有者的 Windows Release 截图推翻了上一轮仅依据浏览器计算样式作出的验收结论：旧版侧栏、主内容和底栏在同高度通常只有 `0–2 RGB` 差异，实际观感仍接近同一整块黑色。修订 [ADR 0022](decisions/0022-native-window-materials-and-surface-hierarchy.md)，明确浏览器只能验证布局、状态和溢出，Windows Mica 的可见层级必须由原生截图与像素采样证明。
- Mica 继续作为唯一根材质，但主内容改用一层低 alpha `content surface`；侧栏/窄屏导航使用 `chrome surface`，底栏新增独立 `player surface`，命令区与独立工作单元使用 `subtle surface`。深色四级值依次为 `12%` 黑、`5%` 白、`8%` 白、`10%` 白，浅色依次为 `18%`、`52%`、`70%`、`78%` 白。完整播放器和压缩工作区不会再重复叠加 content fill；仍不使用整高/整宽分隔线、全局模糊或嵌套材质卡片。
- 主内容增加有限内缩、圆角和裁切，以 WinUI `NavigationView` 式内容层与侧栏区分；播放列表/最近播放命令区使用无描边 subtle surface。设置页保持无卡片分组，宽屏收敛为 `760px`、`240px + 420px` 的稳定双列；空播放状态隐藏无意义的禁用进度轨道，有音频时轨道保持正常。音频压缩和桌面歌词窗口均完成回归，桌面歌词透明、置顶和穿透边界未改动。
- 浏览器回归覆盖 `1080×700` 深浅宽屏默认列表/设置/工具/完整播放器、`360×600` 与 `420×720` 深浅窄屏、`720×500` 深浅音频压缩窗口、长文本桌面歌词和暗色 solid 回退；全部页面横纵溢出为 0，控制台无 warning/error。Windows Release 的 Per-Monitor v2 DPI 原生截图采样为侧栏 `#2F2929`、主内容 `#221A17`、命令区 `#333236`、底栏 `#333131`，相比旧版已形成可感知层级且仍能看到 Mica 采样差异。
- `verify-release-webview.ps1` 新增可选 `-CapturePath`，等待有效尺寸和 UI 稳定帧，按进程枚举真实有标题主窗口，并在 DPI-aware 坐标系截图；默认 `npm run release:windows` 行为保持自动验证和清理。TypeScript/Vite build、许可证清单、Rust format、Clippy `-D warnings`、差异检查和 67 个普通 Rust 测试通过；8 个真实音频设备测试按约定忽略。产物为 `src-tauri/target/release/resona.exe`，18,006,528 bytes，SHA-256 `AA36E7400F840FEA9ABB4F247158F9F5C4BC01A05D85D64EB0A4D6A0BF31EEF1`。

### 2026-07-22 Mica 可见性与系统主题同步修复

- 确认 Windows 11 原生 Mica adapter 正常工作；主窗口材质不明显的根因是连续 content/chrome 表面叠加了深色遮罩。该阶段先恢复单层根材质与透明大区域，后续区域分隔策略已由同日的“WinUI tonal surface 与区域分隔收口”继续修订，当前以 ADR 0022 最新内容为准。桌面歌词透明/穿透边界不变。
- 项目所有者的 Release 截图揭示第一次修订仍显示完全一致的 `#101113`。原生调试确认 Windows 构建和主题同步均返回 `Mica`，实际覆盖源是 Mantine 全局 `body { background-color: var(--mantine-color-body) }`；应用壳透明后仍会露出该实色。现以窗口材质选择器明确清除 `html`、`body` 和 `#root` 背景，同时保留 `--mantine-color-body` 供组件实体表面使用。用于定位的红色探针和动态主窗口实验均已撤销，原有隐藏到首帧及 Windows 10/Linux solid 回退保持不变。
- 设置页移除卡片背景、外框、圆角和阴影；其后相邻分组细线也已在同日 tonal surface 收口中删除，当前只用标题与节间距。工具入口统一为无阴影轻量表面。播放进度 Slider 收敛为细轨道，thumb 只在 hover/focus 时出现，避免与内容滚动条混淆。
- 修复“手动浅色后无法恢复跟随系统”及原生标题栏不同步：前端不再把 `auto` 提前解析成受当前窗口覆盖影响的亮/暗值，`auto`/`light`/`dark` 原样传入 Rust；`auto` 清除 Tauri 原生主题覆盖并使用自适应 Mica，手动模式才固定标题栏和 Mica 变体。
- 浏览器回归覆盖 `1080×700` 宽屏完整播放器、`360×600` 窄屏设置/工具页和 `720×500` 音频压缩窗口；验证深色/浅色/跟随系统、Mica/窗口标识、大区域透明、局部表面无嵌套、设置分组和零横向溢出。TypeScript/Vite build、许可证、Rust format、Clippy `-D warnings` 和差异检查通过；67 个普通 Rust 测试通过，8 个真实音频设备测试按约定忽略。Windows Release 主窗口可见性门禁通过；额外原生像素门禁确认原 `#101113` 已被 Mica 采样色替代。产物为 `src-tauri/target/release/resona.exe`，18,006,528 bytes，SHA-256 `543A40ADD51A413E8C6C1E97BD3AA2647E51D32F31C9AA17B9CF334AE6B8A5A5`。

### 2026-07-22 原生窗口材质与跨窗口设计语言

- 接受并实现 [ADR 0022](decisions/0022-native-window-materials-and-surface-hierarchy.md)：Windows 11 客户端的主窗口与音频压缩窗口使用 Tauri 2 内置 Mica；Windows 10、Windows Server、Linux 和其他不支持路径保持 Mantine 实色回退。没有引入第三方材质插件、第二套 UI 组件库、全局 Acrylic/`backdrop-filter` 或自绘标题栏。
- 新增 `platform/window_material.rs` 统一 Windows 版本检测、Mica 应用和原生亮暗主题同步。主窗口透明创建属性隔离在 `tauri.windows.conf.json`，通用配置仍跨平台不透明；Mica 在主窗口可见前初始化，错误保持可诊断并回退实色。
- 主窗口、宽/窄导航、完整播放器、底栏、设置分组和音频压缩工作区改用共享语义表面 token。压缩窗口标题区收敛为紧凑桌面工具标题栏并删除重复说明。桌面歌词不叠加 Mica，只把字体、圆角、边界和轻量 ActionIcon 状态对齐同一设计语言，保留透明/穿透专用边界。
- 浏览器验证覆盖 Mica 预览下的 `1080×700` 深色宽屏、`360×600` 浅色窄屏、`720×500` 压缩窗口和长歌词窗口；页面零横向溢出、无越界控件或控制组重排，控制台无 warning/error。TypeScript/Vite build、Rust `cargo check --all-targets` 和差异检查通过；完整门禁与 Windows Release 结果见本阶段后续记录。
- 完整门禁通过：66 个普通 Rust 测试通过、8 个真实设备测试按约定忽略，TypeScript/Vite build、许可证、Rust format、Clippy `-D warnings` 和差异检查通过。Windows Release 主窗口可见性探针通过；新产物为 `src-tauri/target/release/resona.exe`，18,007,552 bytes，SHA-256 `1FC3D7251496FBAF64CBA5609DF5C41557FF769E44B3027046421B09F896D9CC`。

### 2026-07-22 窄屏导航与两行底栏收口

- 曲目列表将“播放中”与“已选中”拆为正交视觉状态：当前曲目使用主题浅色背景和三柱动画，选择使用高对比中性底色；边框交回 Mantine `Paper withBorder`，不绘制左右实心条或组合态浅色外框。曲目工作区占满滚动视口，单击任意非曲目、非控件空白区域清除全部选择。
- 窄屏底栏保持紧凑两行：第一行独占曲目信息、进度和时间，第二行稳定分布音质、上一曲/播放暂停/下一曲以及播放模式/桌面歌词/音量；总高度调整为 `120px`。播放器按钮统一使用 Mantine `xl`（实际 `44px`），封面为 `44px`，不再由 CSS 覆盖组件按钮尺寸。
- 窄屏播放列表标签改用 Mantine `Tabs variant="pills"` 表达激活状态；Tab、TabLabel 和内部文本建立完整收缩边界，长名称在标签内省略，不越过选项卡。
- 删除无实际作用的左上角折叠/导航按钮、Mantine Drawer、第二份 Sidebar 及其开关状态。窄屏只保留顶部全局导航和横向播放列表 Tabs，所有页面与列表均可直接到达。
- 窄屏歌词字号和歌词颜色均固定为“标签 + 控件”两列单行布局，避免颜色控件在窄屏落到标签下方。
- [ADR 0021](decisions/0021-compact-top-navigation-and-playlist-tabs.md) 已同步为无 Drawer、两行底栏的最终方案。
- `npm run build`、`npm run licenses`、`cargo fmt --all -- --check`、`cargo clippy --all-targets -- -D warnings` 通过；Rust 测试 64 通过，8 个真实音频设备测试按约定忽略。
- Windows Release 已生成并通过 `verify-release-webview.ps1` 主窗口可见性检查：`src-tauri/target/release/resona.exe`，17,966,080 bytes，SHA-256 `951BBBA0ECBE685A3001724057FF3BF77C6096569589A3B993937911493B66B1`。

## 当前阶段

0.0.1 最小技术验证：已完成。

0.0.2 播放能力闭环：已完成。

0.0.3 队列与连续播放：已完成并通过自动验收。

0.0.4 输出设备生命周期：已完成并通过 Windows 验收。

0.0.5 应用壳与文件夹播放闭环：已完成并通过自动验收与项目所有者功能验证。

0.0.6 Windows SMTC adapter：已完成，媒体控制功能已通过项目所有者 Windows 实机验收；浮层身份显示问题已在 0.0.19 的安装身份与元数据修订中关闭。

0.0.7 持久化播放列表与媒体库基础：已完成。

0.0.8 受管理文件夹与元数据索引：技术验证已完成；该产品范围已由 ADR 0014 淘汰，运行时代码已在 0.0.13 删除。

0.0.9 本地 LRC/SRT/WebVTT 解析与播放同步：已完成并通过项目所有者实机验收。

0.0.10 Windows 桌面歌词窗口技术原型：已完成；项目所有者确认显示、同步、拖动、整窗穿透和原生解锁基本功能正常，产品化缺口转入 0.0.11。

0.0.11 Windows 应用生命周期与桌面歌词交互闭环：已完成并通过项目所有者 Windows 实机功能验收。

0.0.12 播放列表与媒体库交互闭环：实现完成；用户播放列表基础保留，受管理媒体部分退出 0.1.0 范围。

0.0.13 播放器范围收束与列表信息架构：实现及自动验收完成；残余原生外部打开、拖入和 DPI 场景统一进入 0.1.0-rc.1 发行矩阵。

0.0.14 播放体验与完整播放页：已完成并通过自动验收。

0.0.15 交互与桌面歌词收口：已完成并通过自动验收；多显示器和原生拖动最终手感待项目所有者实机验收。

0.0.16 WAV 到 FLAC 音频压缩：已完成并通过自动验收。

0.0.17 发行前界面与音频压缩工作区收口：纠正性实现及自动验收完成；已修复扫描进程风暴、辅助窗口首帧、压缩窗口首开、播放器页布局和列表拖排回归。本阶段没有提前实现尚未确定的安装器、托盘、自更新和文件关联。

0.0.18 播放列表交互与 UI 冻结：已完成。已接受 Foobar2000/AIMP 风格的“播放列表即待播放序列”模型，移除产品层独立队列抽屉和公开队列编辑 commands；当前列表编辑同步 Rodio 内部序列，非当前列表保持隔离。播放列表排序使用带移动阈值的整行 Pointer Events 和扩展插入反馈，外部拖入缩放坐标和命中已修复；默认列表批量文件/直属文件夹追加、统一添加菜单、拖放热区、长路径控件布局和播放状态渲染隔离已完成自动验证。本批已实现默认/用户列表统一选择与编辑、Rust 权威 SVG 音质字标、播放中侧栏提示、桌面歌词三分区工具栏、稳定三段底栏和向上播放模式菜单；模式切换不再重建当前 source，已消除该路径的硬切爆音。ADR 0020 已接受并实现：进度条和歌词点击共用无回跳 seek transaction，主窗口可见滚动区统一为 Mantine `ScrollArea`，Rust 在首帧前恢复宽屏/窄屏各自几何；窄屏使用顶部全局导航与播放列表 Tabs、单列主内容和 `120px` 两行播放器，不再维护重复的侧栏 Drawer。最终主题色预设已收束为 Mantine 原生五色，默认紫色，并完成浅色/深色对比配置。

0.0.19 Windows 深度集成、托盘与分发设计：已完成并通过项目所有者 Windows 安装版验收。SMTC/任务栏、托盘和三态退出、活动转换确认、NSIS/文件关联/AppUserModelID、按需 FFmpeg、Local AppData 迁移、平台化产物名与更新协议均已落地或冻结。

项目所有者已反馈 0.0.6 的媒体控制功能实机测试通过；0.0.19 又通过安装身份、品牌资源和 Rust 权威媒体标题关闭了“未知应用”与 AppUserModelID 误显示问题。

项目所有者已确认 0.0.8 的添加、扫描和移除技术验证正常。2026-07-20 进一步收束 0.1.0 为纯播放器，决定删除受管理文件夹、媒体记录数据库和“文件夹”标签页；该技术验证保留为历史证据，不再形成发行范围。

项目所有者已确认 0.0.9 原有歌词功能正常，并确认 `1.wav.vtt` 等带音频格式或语种限定名的歌词可匹配 `1.wav` 或 `1.flac`。该修订已通过自动回归与实机验收。

项目所有者确认 0.0.10 基本功能正常，同时指出关闭主窗口后歌词与播放仍存活、运行时开关位于设置页、原生解锁按钮空闲时常驻三个严重交互问题。额外进程主要是 WebView2 renderer/GPU/utility 子进程，不代表复制了 Rust 播放核心；当前只有一份 `RodioPlaybackEngine`。真正的应用生命周期和闲置资源问题列为 0.0.11 首要工作。

当前交付状态：0.0.19 已完成并通过项目所有者 Windows 安装版验收；自动门禁的最终数量和产物记录见本页顶部。下一阶段为已接受的 0.1.0-rc.1，继续实现 updater、GitHub Releases/Actions、签名、诊断与发行加固，并完成仍开放的多显示器、DPI 和原生拖入矩阵。

2026-07-22 修复桌面歌词首次打开卡死：旧 `app_data_dir()/desktop-lyrics-window.json` 内容合法但已按项目所有者要求直接删除，未增加兼容分支。实际根因是 `DesktopLyricsWindowService::show` 持有服务锁创建/恢复窗口，原生 `set_size` 触发 `Resized` 后主线程回调重新进入 `enforce_height` 等待同一锁，同时 helper 创建等待主线程，形成循环等待。窗口构建现移到服务锁外，高度校正和 ready 显示也不再跨原生窗口调用持锁；helper 主线程派发增加 5 秒失败边界，异常不再无限阻塞。Rust 普通测试 59 通过、8 个真实设备测试按约定忽略，format、Clippy、TypeScript/Vite build 和 Windows Release 验证通过；测试音频启动后进程 `Responding=True`。新产物为 18,013,696 bytes，SHA-256 `88F5E5561D84DA9963FE743D3AF2612186BA2A110EC46434D73EF7C99B7DD81C`。

首次 GitHub 上传前仓库卫生审计已完成：本地 FFmpeg/ffprobe 测试工具各约 97 MiB，超过 GitHub 100 MB 十进制单文件限制，由 `.gitignore` 排除且只通过 `npm run prepare:test-tools` 显式准备；普通构建和发行不下载、不打包。运行时按需下载使用固定 URL、归档/双二进制 SHA-256。未发现环境文件、私钥、访问令牌、日志或本地数据库；测试音频、Mantine 文档快照、现有图标和 Tauri schema 均有明确用途并保留。

0.1.0-rc.1 开始建立最小 GitHub Actions 发布链；在其落地并通过干净检出验证前，本地自动检查仍是权威验收路径。CI、发布 Action、分支保护和依赖更新策略必须按实际发布流程重新建立并固定完整 commit SHA；FFmpeg 不是构建下载项，也不得进入安装包。

## 已完成

- [x] 明确 Windows-first，Linux 只预留 Wayland + PipeWire 结构
- [x] 确定 Tauri 2 + React + TypeScript + Mantine
- [x] 收束播放需求为 WASAPI Shared、板载声卡/蓝牙和 MP3/WAV/FLAC
- [x] 确定 Rodio + CPAL + Symphonia 为首版播放核心
- [x] 以 ADR 0005 替代 mpv sidecar 决策，首版不实现备用引擎
- [x] 确定 SQLite；lofty-rs 与 FFmpeg sidecar 分别留待按需元数据和音频压缩阶段准入
- [x] 定义 UI 依赖准入标准，排除运行时许可证注入机制
- [x] 建立产品、架构、目录、开发准则、路线图和 ADR 文档
- [x] 将仓库许可证切换为 GPL-3.0-only
- [x] 保存 Mantine 完整 LLM 文档并记录来源、大小和校验值
- [x] 初始化 Tauri 2 + React 19 + TypeScript + Vite + Mantine 应用
- [x] 锁定 Rodio 0.22.2 的最小 `playback`、`flac`、`mp3`、`wav` feature 集
- [x] 实现专用播放 actor、typed channel、`PlaybackEngine` 和 Tauri commands
- [x] 实现 WAV/FLAC 文件选择、播放、暂停/继续、停止及自然结束状态
- [x] 通过 TypeScript/Vite 构建、Rust 测试、Clippy、默认音频设备和 release 构建验证
- [x] 在 680×440 与 360×640 视口验证暗色最小界面，无溢出或控制台告警
- [x] Windows 实机真实音频试听通过，播放、暂停、继续和停止功能正常
- [x] 完成 0.0.1 验收记录并接受 0.0.2 计划
- [x] 生成 WAV/FLAC/MP3 矩阵与损坏文件 fixture，并生成依赖许可证清单
- [x] 启用 MP3，增加播放进度/时长、定位、音量和结构化错误边界
- [x] 自动验证 WAV/FLAC 16/24-bit 与 WAV 32-bit、MP3 矩阵
- [x] 通过 MP3、FLAC、192 kHz/32-bit WAV 默认设备 smoke test
- [x] 通过前端构建、Rust format/test/Clippy、Tauri release build 和桌面进程检查
- [x] Windows 实机验证 MP3/WAV/FLAC 播放、暂停/继续、停止、进度、定位和音量
- [x] 完成 0.0.7 SQLite persistence adapter、播放列表与最近播放基础闭环
- [x] 完成 0.0.7 前端页面接入、空/加载/错误状态和播放列表加载
- [x] 完成 0.0.7 Rust 单元测试、真实音频设备 smoke test、Clippy、前端 build 验证
- [x] 完成 0.0.8 受管理文件夹 CRUD、直属目录增量扫描和只读元数据索引
- [x] 完成 0.0.8 自动测试、Release 构建与项目所有者 Windows 实机验收
- [x] 完成 0.0.9 LRC/SRT/WebVTT sidecar 发现、编码、解析和 Rust 权威同步
- [x] 完成 0.0.9 正在播放页歌词滚动、空/失败状态与 revision 增量传输
- [x] 完成 0.0.9 普通/真实设备测试、Clippy、许可证、前端视觉与 Release 构建验证
- [x] 扩展 0.0.9 歌词限定名匹配，覆盖 `1.wav.vtt` 匹配 `1.wav`/`1.flac`、语种限定名、精确名优先和前缀碰撞
- [x] 以 ADR 0012 固定桌面歌词整窗穿透与无 WebView 原生解锁辅助窗口架构
- [x] 实现独立透明置顶桌面歌词 WebView、轻量前端入口、Rust 权威同步和解锁拖动
- [x] 实现 Tauri 官方整窗穿透与 Win32 owned tool 解锁辅助窗口，辅助窗口失败时拒绝锁定
- [x] 实现主窗口显示/隐藏、锁定/解锁兜底入口和独立最小 capability 权限
- [x] 完成 0.0.10 普通/真实设备测试、Clippy、许可证、前端视觉和 Tauri Release 构建验证
- [x] 完成 0.0.10 项目所有者 Windows 实机技术验证，确认双窗口与整窗穿透方案可行
- [x] 接受 0.0.11 生命周期、播放区入口、桌面歌词控件、偏好和歌词空白区定位范围
- [x] 实现桌面歌词上一曲/播放暂停/下一曲、设置、锁定解锁和关闭控件
- [x] 实现主界面歌词字号、颜色和背景透明度偏好共享
- [x] 修复第一条歌词前错误复用第一行，并统一 LRC/SRT/VTT 为“下一行开始前保留最近一行”的音乐播放器时间轴语义
- [x] 接入主窗口退出、Tauri 官方单实例和隐藏歌词资源释放路径
- [x] 完成 0.0.11 浏览器前端、内嵌 Release WebView、许可证、Clippy、Release 构建和代码差异检查
- [x] 修复锁定后原生 helper 使用 alpha=0 导致部分 Windows 环境不派发 hover、无法显示解锁按钮的问题
- [x] 完成播放列表项目事务化删除、重排、稠密位置维护和未知 ID 错误边界
- [x] 完成播放列表选中详情、重命名、载入、追加、移除和上下移动的最小功能流程
- [x] 完成受管理文件夹选择、直属媒体记录展示、missing 状态、播放和追加队列流程
- [x] 完成 0.0.12 浏览器前端、29 个普通/8 个真实设备 Rust 测试、许可证、Clippy 和 Release 构建检查
- [x] 以 ADR 0014 将 0.1.0 收束为纯播放器，取消受管理文件夹/媒体库索引，并定义临时默认列表和新侧栏结构
- [x] 确定桌面歌词后续视觉规则：文字透明度、未锁定 hover 固定交互背景、两行滚动切换及单行降级
- [x] 修复进度 Slider 浮层显示裸毫秒值，统一为格式化时间；音量浮层统一为百分比
- [x] 确认 32-bit FLAC 不纳入支持范围，并验证失败后可恢复播放普通 FLAC
- [x] 使用 Rodio `source::Done` 原子标记完成逐曲结束检测，移除自定义 source wrapper 和音频线程 channel
- [x] 文件选择对话框增加单实例互斥与失败边界，确认 Tauri/rfd 自动设置 Windows parent
- [x] 以 ADR 0006 固定侧栏/主信息区/底栏结构、正在播放展开行为、系统主题和中英文要求
- [x] 建立 0.0.6 SMTC 计划，区分系统媒体会话、通知 Toast、系统托盘和全局快捷键
- [x] 实现左侧分组导航、右侧主信息区、稳定底部播放器和响应式窄屏布局
- [x] 实现正在播放视图及空队列禁用规则，展开前后底栏尺寸和控制状态保持不变
- [x] 实现系统/浅色/深色模式、主题色和中文/英文界面偏好
- [x] 实现打开文件后同目录直属音频原子组队、文件夹浏览和按位置批量插入
- [x] 使用 Tauri 官方 WebView 拖放事件接收外部文件/文件夹，并返回结构化拒绝原因
- [x] 通过 13 个普通 Rust 测试、7 个真实音频设备测试、format、Clippy、前端构建、许可证报告和 Tauri release 构建
- [x] 在 1280×800、680×440 和 360×640 验证浅色/深色、中文/英文、空队列和正在播放布局；干净预览页无控制台 warning/error
- [x] 锁定 `souvlaki 0.8.3`（MIT，关闭默认 D-Bus feature）并生成更新后的依赖许可证报告
- [x] 实现 Windows `MediaSessionAdapter`，隔离 HWND、SMTC 类型和系统回调，不让其进入播放领域契约
- [x] 实现 SMTC 播放/暂停/切换、停止、上一曲、下一曲、前后跳转和定位命令映射
- [x] 通过 SMTC 命令映射设备测试；普通 Rust 测试 13/13，真实音频设备测试 8/8
- [x] 设置固定 Windows AppUserModelID，Release 使用 GUI subsystem 隐藏终端窗口；Debug 仍保留终端日志
- [x] 完成 0.0.6 Windows 实机媒体键和 SMTC 控制验收；浮层身份显示问题转入已知限制
- [x] 完成 schema v3 收缩 migration，保留用户播放列表/最近历史并删除受管理媒体表
- [x] 实现 Rust 权威临时默认列表、路径快照 revision 复用和统一 external-open 服务
- [x] 完成侧栏最近播放/播放列表树、默认与用户列表详情、外部拖放位置契约和工具/设置信息架构
- [x] 删除 managed-folder/media-library 前后端运行时代码，保留未编译的按需元数据源文件
- [x] 主窗口调整为 1080×700、最小 760×520，并以主题预绘制、隐藏首帧和实际可见 HWND 探针收束启动白屏风险
- [x] 完成 0.0.13 浏览器交互与冷启动控制台验证、29 个普通/7 个真实设备 Rust 测试、许可证、Clippy 和 Windows Release 检查

## 阶段记录

- [x] 完成 [0.0.3 队列与连续播放计划](plans/0.0.3.md)
- [x] 完成 [0.0.4 输出设备生命周期计划](plans/0.0.4.md)
- [x] 完成 [0.0.5 应用壳与文件夹播放闭环计划](plans/0.0.5.md)
- [x] 完成 [0.0.6 Windows 媒体会话与硬件控制计划](plans/0.0.6.md)
- [x] 完成 [0.0.7 持久化播放列表与媒体库基础计划](plans/0.0.7.md)
- [x] 完成 [0.0.8 受管理文件夹与元数据索引计划](plans/0.0.8.md)
- [x] 完成 [0.0.9 本地多格式歌词解析与播放同步计划](plans/0.0.9.md)
- [x] 完成 [0.0.10 Windows 桌面歌词窗口技术原型计划](plans/0.0.10.md)
- [x] 接受 [0.0.11 Windows 应用生命周期与桌面歌词交互闭环计划](plans/0.0.11.md)
- [x] 项目所有者验收 0.0.11 Windows 原生控件、生命周期和透明 helper
- [x] 确认并接受 0.0.12 播放列表与媒体库交互闭环计划
- [x] 接受 [ADR 0014 播放器单一范围与临时默认播放列表](decisions/0014-player-only-scope-and-transient-default-playlist.md)
- [x] 项目所有者确认 [0.0.13 播放器范围收束与列表信息架构计划](plans/0.0.13.md)
- [x] 实现 [0.0.13 播放器范围收束与列表信息架构计划](plans/0.0.13.md)，待项目所有者实机验收
- [x] 完成 [0.0.14 播放体验与完整播放页计划](plans/0.0.14.md)
- [x] 完成 [0.0.15 交互与桌面歌词收口计划](plans/0.0.15.md)
- [x] 完成 [0.0.16 WAV 到 FLAC 音频压缩计划](plans/0.0.16.md)
- [x] 接受 [ADR 0016 音频压缩独立工作窗口](decisions/0016-audio-compression-workspace-window.md)，实现纳入 0.0.17 发行前收口
- [x] 完成 [0.0.17 发行前界面与音频压缩工作区收口计划](plans/0.0.17.md)
- [x] 接受 [ADR 0017 完整播放器作为右侧主内容视图](decisions/0017-full-player-as-main-content-view.md)
- [x] 完成 [0.0.18 播放列表交互与 UI 冻结计划](plans/0.0.18.md)：Completed
- [x] 接受 [ADR 0018 播放列表即当前播放序列](decisions/0018-playlist-as-playback-sequence.md)，移除产品层独立队列并保留 Rodio 内部执行序列
- [x] 接受并实现 [ADR 0020 主窗口宽屏/窄屏模式与几何恢复](decisions/0020-main-window-layout-modes-and-geometry.md)
- [x] 完成 [0.0.19 Windows 深度集成、托盘与分发设计计划](plans/0.0.19.md)，项目所有者安装版原生验收通过
- [x] 接受并实现 [ADR 0023 Windows Shell 媒体与任务栏集成](decisions/0023-windows-shell-media-integration.md)
- [x] 接受并实现 [ADR 0024 Windows 托盘生命周期、分发与更新基线](decisions/0024-windows-tray-distribution-and-update.md)
- [x] 将 [0.0.20 更新器与发行加固计划](plans/0.0.20.md) 归并至 0.1.0-rc.1，不再发布独立 0.0.20
- [x] 接受 [0.1.0-rc.1 Windows 发布候选收尾计划](plans/0.1.0-rc.1.md)
- [ ] 确认 [0.1.0 Windows 首个正式版发行门禁](plans/0.1.0.md)：Proposed

## 下一步

1. 按 [0.1.0-rc.1 计划](plans/0.1.0-rc.1.md) 接入 Tauri updater、GitHub Releases/Actions 和两套签名边界。
2. 建立本地有限诊断日志并测量启动、空闲、播放、托盘、桌面歌词和音频压缩资源预算，只修复有证据的回归。
3. 完成宽屏/窄屏几何、最大化、多显示器/DPI、资源管理器拖入、当前列表播放中编辑和桌面歌词 helper 的 Windows 实机发行门禁；这些验证不再改变已冻结的功能结构。
4. 生成一致的 RC 安装包、更新产物、更新清单、许可证通知、版本说明和 SHA-256，项目所有者验收后进入 0.1.0 正式发布门禁。

## 阻塞项

0.0.10 技术原型无阻塞项并已关闭。0.0.11 无已知代码阻塞，锁定解锁热点已改为 alpha=1 的近透明命中方案并通过项目所有者验收。位置持久化、多显示器恢复和最终视觉仍后置。

Windows 原生文件选择窗口仍会偶发阻塞。应用侧单实例和 parent 边界已落实，当前暂缓进一步处理，不阻塞 0.0.10；重新调查条件见 [KNOWN_ISSUES.md](KNOWN_ISSUES.md)。

0.0.16 无已知代码阻塞。自动验证已完成；原生首次启动视觉感知、多显示器歌词恢复、系统文件选择器和实际用户文件删除确认属于项目所有者验收项。

0.0.17 已完成。后续使用发现的外部文件拖入与内部排序冲突已在 0.0.18 完成纠正性实现：原生路径事件与 Pointer Events 内部排序彻底分离，缩放坐标和整行插入命中已修复。Windows UI Automation 无法访问 WebView2 内的 React 控件，因此原生外部拖入与命中手感仍是 0.1.0 阻塞验收项。

0.0.18 的完整播放器详情排版、主题色和页面层级已经收口，功能 UI 已冻结。之后发现的阻断缺陷可以纠正，但新的视觉方向或页面结构变更必须进入后续版本计划。

## 尚未验证的关键风险

| 风险 | 验证阶段 | 状态 |
| --- | --- | --- |
| Rodio/CPAL/Symphonia 版本组合与 API 稳定性 | 0.0.2 | WAV/FLAC 16/24、WAV 32、MP3 已验证；FLAC 32-bit 明确不支持 |
| 24/192、混合采样率与 Gapless 正确性 | 0.0.3 | 已验证预载连续播放；动态补队列不宣称 Gapless |
| WASAPI Shared 设备切换与蓝牙断开恢复 | P1 | 未验证 |
| Rodio 上游停更后的固定、fork 或替换路径 | 持续检查 | 已设计，未触发 |
| 透明歌词窗口、跨进程穿透与原生解锁辅助窗口 | 0.0.10 | 技术方案已通过实机验证；退出生命周期、入口和 hover 显隐转入 0.0.11 |
| 主窗口退出、单实例与桌面歌词闲置资源 | 0.0.11 | 已实现并通过 Windows 原生验收 |
| 0.0.12 用户播放列表数据升级到收缩后的 schema | 0.0.13 | schema v3 migration 回归通过；真实应用数据迁移及 Release 启动通过 |
| 初次/第二实例外部媒体参数统一进入临时默认列表 | 0.0.13 | 参数解析与统一服务自动测试通过；原生第二实例待项目所有者验收 |
| 启动白屏与默认 680×440 窗口 | 0.0.16 | 已改为 1080×700；Rust 先恢复几何，React 同步权威布局后发送 ready，3 秒原生兜底防止无窗口；Release 可见探针通过 |
| Windows SMTC 系统媒体浮层和真实媒体键 | 0.0.19 | 控制功能、Resona 身份、实际媒体标题和任务栏集成已通过项目所有者安装版验收 |
| Windows 资源管理器外部文件/文件夹拖入默认或用户播放列表 | 0.0.18 | 默认列表批量追加及用户列表插入已接通；事件隔离、缩放坐标和命中已修复，待 100%/125%/150% Windows 实机验收 |
| 当前播放列表与 Rodio 内部执行序列同步 | 0.0.18 | 活动用户列表插入/移动/删除和非活动列表隔离已通过自动测试；Windows 实际播放中的编辑手感待项目所有者验收 |
| 完整播放器详情长文本与最终 UI 一致性 | 0.0.18 | 五行标签和值已统一对齐；宽窄屏、明暗、双语回归完成，功能 UI 已冻结 |
| 进度 Slider 点击/拖动后的旧快照回跳 | 0.0.18 | 已实现递增事务 ID 和目标保持；Slider/歌词点击复用同一路径，浏览器跳转回归通过 |
| 主窗口几何与显式窄屏模式 | 0.0.18 | ADR 0020 已实现；宽屏保存/再次启动恢复和 Release 可见探针通过，窄屏切换、多显示器/最大化/DPI 待项目所有者实机验收 |
| GPL-3.0-only 与 Rodio/CPAL/Symphonia/FFmpeg 依赖组合 | P6 前持续检查 | 依赖许可证报告通过；FFmpeg 8.1.2 GPLv3 固定构建、来源与 SHA-256 已记录 |

## 台账维护规则

- 每次有效开发后更新“已完成、进行中、下一步、风险”。
- 只记录已经验证的事实；计划不得写成完成。
- 测试失败或外部依赖阻塞必须明确记录，不能省略。
- 完成一个阶段后，把验收证据或命令摘要记录在本文件或对应阶段文档。
## 0.0.5 验收摘要

- 项目所有者确认播放、队列和控件功能正常。
- 13 个普通 Rust 测试与 7 个默认音频设备测试通过；前端 build、许可证报告、Rust format/Clippy 和 release 构建通过。
- 浏览器验证覆盖 1280×800、680×440、360×640、浅色/深色、中文/英文、空队列和正在播放展开态。
- release 可执行文件：`src-tauri/target/release/resona.exe`。

## 0.0.6 当前验收摘要

- `souvlaki 0.8.3` 已锁定并通过许可证报告，MIT，无运行时许可证或联网激活机制。
- SMTC adapter 已接入独立线程和窗口事件生命周期；初始化失败只降级，不阻塞普通播放。
- Windows Release 已设置 AppUserModelID `io.github.vki.resona`，并隐藏发布启动时的终端窗口。
- 修正版 Release：`src-tauri/target/release/resona.exe`；PE subsystem 已检查为 Windows GUI。
- 项目所有者已实机确认媒体控制正常；浮层身份显示问题已记录在 `KNOWN_ISSUES.md`，不阻塞后续开发。

## 0.0.8 验收摘要

- 受管理文件夹添加、扫描、重复扫描、移除和既有播放功能已由项目所有者确认正常。
- 17 个普通 Rust tests、8 个真实音频设备 tests、前端 build、许可证检查、Rust format/Clippy 和 Release 构建通过。
- 索引幂等、直属目录过滤和缺失记录保留由自动测试覆盖；媒体记录详情尚未在 0.0.8 UI 展示。
- 项目所有者指出文件夹行无法选择、展开或展示内容，当前交互不可用；该部分与播放列表交互统一推迟，0.0.8 只验收技术基础。
- 详细记录见 [releases/0.0.8.md](releases/0.0.8.md)。

## 0.0.9 验收摘要

- 项目所有者已确认 LRC/SRT/WebVTT 播放同步和限定名匹配修订功能正常。
- 24 个普通 Rust tests 与 8 个真实音频设备 tests 通过；歌词覆盖 LRC/SRT/WebVTT、UTF-8/UTF-16/GBK、坏 cue 隔离、格式优先级、限定名与前缀碰撞、seek/stop 和 revision 增量。
- 前端 build、许可证报告、Rust format/Clippy 和 Tauri Release 构建通过。
- 浏览器验证覆盖 1280×800、680×440、360×640、浅色/深色和窄屏滚动；无页面溢出或控制台 warning/error。
- Release：`src-tauri/target/release/resona.exe`；`1.wav.vtt` 等限定名匹配已通过项目所有者实机验收。

## 0.0.10 验收摘要

- 已实现第二个透明、置顶、无边框 Tauri 歌词 WebView；前端按窗口标签懒加载，独立 chunk 为 2.32 kB（gzip 1.06 kB），不加载媒体库或第二播放控制器。
- 锁定使用 Tauri 官方 `set_ignore_cursor_events(true)`；微型 Win32 owned tool window 使用 `WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE`，不含 WebView、全局 hook、轮询或输入注入。
- Tauri 2.11.5 稳定 API 的无 WebView `WindowBuilder` 仍要求 `unstable` feature，因此未启用该 API，原生辅助窗口严格隔离在 Windows platform adapter。
- 26 个普通 Rust tests 与 8 个真实默认音频设备 tests 通过；前端 build、许可证报告、Rust format/Clippy、`git diff --check` 和 Tauri Release 构建通过。
- 浏览器视觉验证覆盖桌面歌词正常/空状态、设置页中英文、暗色模式和 680x440/360x640 视口，无横向溢出或控制台 warning/error。
- 项目所有者确认桌面歌词显示、同步、拖动、锁定穿透和原生解锁按钮基本功能正常，技术原型通过。
- 项目所有者同时确认主窗口关闭后歌词与播放仍存活、运行时开关位置错误、原生解锁按钮空闲常驻；这些产品化阻塞项转入 0.0.11。
- Release：`src-tauri/target/release/resona.exe`。

## 0.0.11 当前实现摘要

- 桌面歌词顶部控件顺序为上一曲、播放/暂停、下一曲、设置、锁定/解锁、关闭；设置按钮通过 Rust command 激活主窗口并切换设置页。
- 主界面设置页提供歌词字号（16-64 px）、颜色和背景透明度偏好，两个 WebView 通过共享本地偏好同步。
- Rust 播放契约新增上一曲命令，超过 3 秒先回到当前曲目开头，否则切换到上一个有效曲目；第一首时重播当前曲目而不进入失败状态。
- 第一条歌词开始前桌面歌词不再回退显示第一行；第一条开始后，Rust 时间轴在下一行开始前持续返回最近一行，SRT/VTT cue 间隙和末行之后不再让歌词消失。
- 主窗口退出、官方单实例和隐藏歌词资源释放已接入；原生 helper 锁定空闲时使用 alpha=1 保留命中，鼠标进入后显示解锁图标。
- 版本已更新为 `0.0.11`。项目所有者发现交付 exe 错误访问 `http://localhost:1420`；根因是 Tauri CLI 构建产物随后被普通 `cargo build --release` 覆盖。已增加 `release:windows` 构建入口和无 Vite 服务时的 Release 主窗口启动检查；修复后已重新通过官方 Tauri 构建链验证。

## 0.0.12 当前实现摘要

- 版本已更新为 `0.0.12`；播放列表项目新增事务化 remove/move 和稳定错误边界，`replace_queue_and_play` 支持从持久化列表指定曲目开始播放。
- 播放列表页支持选中查看、重命名、载入、追加当前队列、移除和上下移动；点击列表本身不再立即覆盖当前播放队列。
- 受管理文件夹支持选中查看直属媒体记录，展示只读元数据、时长、封面存在性和 missing 状态，并可播放或追加队列。
- 29 个普通 Rust tests 与 8 个真实设备 tests、TypeScript/Vite build、许可证、format、Clippy、浏览器中英文/明暗/响应式交互和 Windows Release 构建/启动检查通过。
- Release：`src-tauri/target/release/resona.exe`；SHA-256 `3262A750F69802E8FBCDC4EBD9A8EDE5A9A3754F56063B1431528D342E33A2E6`。

## 0.0.13 当前实现摘要

- 版本已更新为 `0.0.13`；SQLite 收缩为用户播放列表、项目与最多 100 条最近历史，临时默认列表只保存在 Rust 会话内。
- 初次参数、第二实例与应用内打开复用 `open_media_context`；相同直属目录路径快照保持 revision，只在队列不匹配时重建。
- 侧栏收束为最近播放、默认/用户播放列表、工具和设置；文件夹与侧栏“正在播放”页面删除，播放详情保留给 0.0.14 覆盖页。
- 浏览器验证覆盖中英文、浅色/深色、1280×720、1080×700、760×520、长名称和播放列表创建/重命名/删除/添加/移除/重排；冷载入控制台无 warning/error。
- 29 个普通 Rust tests、7 个真实设备 tests、TypeScript/Vite build、许可证、format、Clippy `-D warnings`、`git diff --check` 和 Windows Release 实际可见窗口检查通过。
- Release：`src-tauri/target/release/resona.exe`；SHA-256 `59D21E67546EBB726B63918C68654C1380372898267AB1A44858A3EDCF8F89BE`。

## 0.0.16 当前实现摘要

- 版本已更新为 `0.0.16`；0.0.14-0.0.16 的播放入口收口、播放模式/会话恢复、完整播放页、列表拖排、桌面歌词收口和 WAV 到 FLAC 音频压缩均已实现。
- FFmpeg/ffprobe 固定为 Gyan.dev 8.1.2 GPLv3 static build，归档与两个二进制 SHA-256、构建选项和来源均已记录；这是 0.0.16 当时的随包 sidecar 状态，已由 0.0.19 的 Local AppData 按需下载、校验和原子启用方案替代。
- 真实转换 fixture 覆盖 44.1/48/96/192 kHz、16/24/32-bit integer 和 32-bit float；16/24-bit 保持，32-bit 输出 24-bit，采样率与声道保持。
- 后端强制验证删除确认；输出冲突、取消和失败保留源文件，成功路径只在 ffprobe 验证、同目录临时文件原子提交后删除源 WAV。
- 浏览器验证覆盖单一标题/状态、队列抽屉、完整播放页歌词/信息切换、760×520、暗色模式、压缩默认删除开关和控制台；发现并修复最小尺寸关闭按钮层叠。
- 42 个普通 Rust tests、7 个真实设备 tests（本轮未运行）、TypeScript/Vite build、许可证、format、Clippy `-D warnings` 和 `git diff --check` 通过。
- 主窗口由 Tauri `PageLoadEvent::Finished` 显示；沙箱外 Release 首次冷启动及连续三次重启均通过可见 HWND 探针。启动级 panic 会生成 `%TEMP%/resona-startup-error.log` 并显示错误，而不再静默退出。
- Release：`src-tauri/target/release/resona.exe`；SHA-256 `92D82FF965520C5E8F4DE880F23B8DC1FD63E24C9726E22C23CA65EC20C35000`。

## 0.0.17 当前实现摘要

- 版本已更新为 `0.0.17`。完整播放器改为右侧主内容互斥视图，点击侧栏自然退出；普通内容不会在播放器下方继续渲染。
- 完整播放器与底栏使用同一不透明主题表面，展开时取消分割白边；“同步歌词”收敛为“歌词”/“Lyrics”，设置页重复“输出设备”行标签改为“设备”。
- Tauri 外部拖放期间，侧栏播放列表间隔和曲目插入间隔从 5/6 px 展开到 14 px；内部曲目和播放队列只通过拖拽重排，右侧上下箭头已移除，私有拖拽标记阻止内部曲目触发侧栏外部导入交互。
- 主工具页只保留音频压缩入口和活动任务摘要。`audio-compression` 是按需创建的单实例普通 WebView，使用独立最小 capability、共享主题/语言，并恢复可见范围内的窗口位置和尺寸。
- Rust 压缩扫描工作区支持多文件/文件夹根、普通子目录递归、重叠路径去重、junction/符号链接/reparse point 跳过、进程内 `hound` WAV 头验证、取消、结构化警告、目录树、路径排除和关闭重开快照恢复；扫描不再启动 ffprobe 子进程，剩余 FFmpeg/ffprobe 在 Windows 使用无控制台标志。
- 压缩树默认折叠子目录，每个节点按 200 项渐进渲染并使用 `content-visibility`；已有 FLAC 以不可转换冲突项显示，最终转换只提交验证通过的 PCM WAV。
- 压缩参数、删除开关和转换命令移入顶部控制区，目录树占满剩余宽度；压缩窗口由 Tauri 页面加载完成事件显示。桌面歌词在透明预绘制和首个权威快照完成后才通过幂等 ready command 显示。
- 完整播放器歌词自动定位当前行并隐藏滚动条；歌词/播放信息使用固定面板和切换栏，输入音频与实际 Rodio/CPAL 系统输出配置各占一行。底栏以固定锁定/忙碌槽和音量弹层消除状态切换位移。
- LRC/SRT/WebVTT 活动行统一采用音乐播放器语义：第一行开始前不显示，之后持续保留最近已开始的歌词直到下一行开始；桌面歌词只在新行到来时滚动切换。
- 底栏右侧收束为五个 28px 等距槽位，播放队列固定在最右并删除重复的播放器展开按钮；完整播放器只由左侧封面切换，hover/focus 显示展开/收起图标。设置页分组间距收紧，八个设置行统一为 44px。
- 浏览器验证覆盖中英文、浅色/深色、1080×700、920×680、720×520、完整播放器固定几何、歌词居中、输入/输出信息、稳定底栏、播放列表箭头清理和透明歌词首帧；无页面溢出或控制台 warning/error。
- 49 个普通 Rust tests 通过，7 个真实音频设备 tests 按约定忽略；新增 45 文件扫描回归证明扫描不依赖 ffprobe，并覆盖 16/24/32-bit integer 与 32-bit float 头解析。TypeScript/Vite build、许可证、Rust format、Clippy `-D warnings`、`git diff --check` 和最终 Windows Release 可见主窗口检查通过。
- Windows UI Automation 不能穿透 WebView2 React 内容，未用坐标/按键次数伪造原生入口测试；压缩窗口入口、文件夹选择、外部拖放和再次聚焦留给项目所有者实机验收。
- Release：`src-tauri/target/release/resona.exe`；大小 17,550,848 bytes；SHA-256 `3CD6157015C258B346CAC28F4DB968626A2F006557286FF19D1F8FB107BE8AD3`。

## 0.0.18 当前实现摘要

- 已接受 ADR 0018：默认列表是临时待播放列表，用户列表是持久化待播放列表；点击曲目激活对应列表，当前列表的插入、追加、删除和排序同步 Rodio 内部执行序列，非当前列表编辑保持隔离。
- 用户列表播放入口只向 Rust 提交列表 ID 和项目 ID；Rust 读取权威顺序并激活列表。独立队列抽屉及 `replace_queue`、`replace_queue_and_play`、`append_to_queue`、`remove_queue_item`、`move_queue_item`、`clear_queue` Tauri commands 已删除。
- 删除当前持久化列表时，既有执行序列进入默认临时列表，不形成无 UI 来源的隐藏队列。ADR 0019 已取代默认列表的会话路径恢复；退出后只保留播放偏好。
- 默认列表外部拖放只命中空列表/曲目滚动区域，标题、来源路径和操作控件不命中；长来源路径使用可收缩 flex 轨道，统一添加按钮在默认桌面和 760×520 下保持完整可见。
- 已实现稳定默认项目 ID、单选/Ctrl/Shift、双击播放、可执行右键操作、批量删除、清空与整行阈值排序；拖动框选因命中不可靠且与排序手势冲突而删除，排序使用扩展间隔和插入线反馈。用户列表删除入口移至侧栏右侧，播放中的列表使用低成本 CSS 均衡器标识。底栏移除重复列表与锁定入口，Rust 元数据分类的 Hi-Res/SQ/HQ 改为固定宽度纯文本描边框；桌面歌词工具栏改为媒体名、播放控制和窗口控制三分区，锁定 helper 居中。
- 播放列表与曲目右键菜单已提升到页面级 Portal，修复父容器动画导致的鼠标坐标偏移；菜单按钮事件不再被全局外部点击监听提前卸载。侧栏整行负责右键入口，悬停删除按钮不再形成菜单死区；菜单收敛为“重命名 / 删除”纯文本，品牌版本副标题、重复的工具规划说明和一批空状态文案已精简。
- 本批 Release 已通过可见主窗口检查：`src-tauri/target/release/resona.exe`，大小 17,854,464 bytes，SHA-256 `A4AB0B3F342E68509FEDB200D12FA581044820FE920390C8012B46CB9926389C`。
- 播放/暂停等即时 command 不再切换全局 busy；普通列表、设置和完整播放器使用稳定 callback/对象引用与 memo 边界。浏览器确认暂停/继续前后用户列表 HTML 完全一致、`view-in` 动画未重新触发，冷加载控制台无 warning/error。
- 55 个普通 Rust tests 通过，7 个真实音频设备 tests 按约定忽略；TypeScript/Vite build、许可证、Rust format、Clippy `-D warnings`、`git diff --check` 和 Windows Release 可见主窗口检查通过。
- Release：`src-tauri/target/release/resona.exe`；大小 17,853,440 bytes；SHA-256 `31CF7E41F43F7C14754ED34E48EB1AD0B2571E08BEB3F5B5B845C44DB513AC77`。本次纯文本音质标识修订已通过浅色/暗色、空标识和 412px 窄屏浏览器回归，底栏无横向溢出。
- 播放模式切换不再停止并按位置重建当前 `Player`；预载 source 使用原子 queued/started/cancelled 状态，只撤销尚未开始的后续 source。新增 2 个无设备竞态测试和 1 个真实设备连续进度测试，后者已单独通过。
- 模式控件改为向上四项菜单；音质标识使用固定 viewBox SVG 文字框；底栏左右对称轨道保持核心控制居中。压缩窗口与播放列表复用 `AddMediaMenu`，删除头部重复计数并收紧到 `800×560` 默认尺寸、`700×500` 最小尺寸。
- 本批浏览器验证覆盖主窗口/压缩窗口暗色与浅色、`1080×700`、`800×560` 和 `700×500`，模式菜单选择、共享添加菜单和横向溢出均正常，控制台无 warning/error。
- 最新 Windows Release 已通过可见主窗口检查：`src-tauri/target/release/resona.exe`，大小 17,818,112 bytes，SHA-256 `B7D21788EDA4F968DBAEE7668E2F6D529EDB565D2E506F067205E2785538E869`。
- 项目所有者实机发现压缩窗口仍恢复历史 `1710×1860` 尺寸；确认代码默认值已更新但被本地几何文件覆盖。按要求仅清除本机 `audio-compression-window.json`，不增加一次性兼容迁移或额外运行时代码；下次打开直接采用 `800×560` 默认尺寸。
- 2026-07-22：接受并实现 ADR 0020 与 0.0.18 补充范围。前端新增共享 seek transaction、可点击/键盘激活歌词行、设置/侧栏/歌词/播放信息 Mantine `ScrollArea` 和显式窄屏应用壳；Rust 新增主窗口模式与两套几何服务、首帧 ready 和 3 秒显示兜底。浏览器在 `1080×700`、`760×520`、`420×720`、`360×600` 下验证中英文、明暗、Drawer、设置、完整播放器两个页签、辅助控制和歌词跳转，干净会话无 warning/error。普通 Rust 测试 59 通过、8 个设备测试忽略；Release 连续两次启动可见，保存的高 DPI 内容区尺寸连续保持 `1620×1050`，未出现外框/内容区混用导致的增长。最终产物 17,942,016 bytes，SHA-256 `1B9EE9E2991DB6415F0668DB62E54F9A7DEA09ACA396E2598455FF538FCD0630`。
- 2026-07-22：根据项目所有者窄屏反馈完成纠正性实现。左侧改为 `48px` 常驻单图标栏，顶部展开和播放列表入口打开复用完整 `Sidebar` 的显式侧面板；移除底栏“更多”弹层，在第二行直接展示音质、模式、上一曲、播放/暂停、下一曲、停止、桌面歌词和音量。`OverflowMarquee` 只覆盖底栏曲名与侧栏名称，并在减少动态效果时退化为省略。浏览器在 `360×600`/`420×720` 验证中英文、明暗、完整侧面板、默认列表 4 条曲目和全量控制，360px 下主区宽 312px、页面无横向溢出且控制台无 warning/error；`760×520` 宽屏回归通过。前端 build、许可证、Rust format/test/Clippy 和差异检查通过，59 个普通 Rust 测试通过、8 个设备测试忽略。Windows Release 可见性验证通过；`resona.exe` 为 17,943,040 bytes，SHA-256 `4233C6FBA0804BF4ED42230A60DB764A2BFF8C0B684795BAE4663EF869D30EC9`。
- 2026-07-22：修复播放列表曲目文本和底栏长标题的收缩边界。`420px` 以下隐藏序号时同步把曲目行从双列改为单列，避免内容误落入原 `24px` 序号列；曲目名称、路径与默认列表来源改用 `OverflowMarquee` 按需模式，仅在当前文本收到指针交互时测量。底栏标题改为悬停滚动，轨道容器显式占满并限制在左侧网格轨道内，禁止长元数据穿透窗口；大列表不创建逐行常驻 `ResizeObserver` 或持续动画。浏览器已覆盖 `320×600`、`360×600`、`420×720`、`760×520` 和 `1080×700`：360px 曲目文本列由错误的 24px 恢复为约 256px，320px 长路径正确进入按需滚动状态，760px 溢出曲名正确识别，所有视口横向溢出为 0 且控制台无 warning/error。
- 2026-07-22：修复桌面歌词偏好颜色覆盖工具栏控件的问题；颜色和文字透明度现在仅作用于歌词正文，工具栏使用独立中性色。正文使用固定两行总高度，当前歌词自然换行，实际换行时占满两行并隐藏下一行预览，单行时与预览各占一行；超过两行的极端长句才启用正文区域内的水平滚动，动画仅使用 `transform`。不缩小用户字号、不改变窗口几何，尺寸判断仅使用单个 `ResizeObserver`。
- 2026-07-22：完成本次修复后的 Windows Release 构建与原生窗口可见性验证。产物为 `src-tauri/target/release/resona.exe`，大小 17,943,552 bytes，SHA-256 `E4E649BBD7753534816D6213B531D6868DCF3F6651862DA6394776570A1E1DA1`。
- 2026-07-22：项目所有者实机发现上述 Release 存在阻断性桌面歌词回归：超过两行时，布局测量会在换行与 `nowrap` 水平滚动两种渲染状态之间形成反馈环路，导致歌词反复跳跃。此前浏览器验证只覆盖默认短句和静态计算样式，不能视为长歌词渲染通过；该 Release 不作为 0.0.18 桌面歌词验收产物。纠正计划已写入 `docs/plans/0.0.18.md`，待项目所有者审阅后实施。
- 2026-07-22：完成桌面歌词纠正实现。独立 wrapped/nowrap 测量层消除长歌词反馈环路，并修复 `scrollHeight` 舍入导致短句误判两行的问题；歌词开关持久化，后续播放自动显示；字号使用支持手动草稿提交的纯数字输入，不显示步进器、不响应滚轮且不保留预设菜单。Windows 窗口高度按字号固定计算、只记忆位置/宽度，左右点状握柄通过 Tauri 官方水平 resize capability 调整宽度，窗口 `Resized` 事件强制恢复目标高度。
- 2026-07-22：浏览器稳定性门禁覆盖 `short`、`two-lines`、`long-zh`、`long-latin`、`empty`，16/28/64px 与 480/760/1280px 宽度；每组连续 5 秒采样均保持布局模式、几何和滚动距离稳定，页面横向溢出为 0。前端 build、Rust format、`cargo test --all-targets`（59 通过、8 个设备测试忽略）和 Clippy `-D warnings` 通过。
- 2026-07-22：最终 Windows Release（含高 DPI 宽度恢复修正和中文预设文案修正）已通过 `verify-release-webview.ps1` 原生主窗口可见性检查。产物：`src-tauri/target/release/resona.exe`，大小 18,012,672 bytes，SHA-256 `D6B32DB8AAF67DDA79B00038F5AE994691357C06D792623DA1E816AEC6D91DF8`。左右 resize 手感、锁定穿透和多显示器位置恢复仍需项目所有者实机验收。
- 2026-07-22：按项目所有者反馈移除左右整条可见 resize 热区，改为未锁定 hover/focus 时出现的点状握柄；删除自定义 Win32 resize command，改用歌词窗口最小 capability 调用 Tauri 官方 `startResizeDragging`。顶部工具栏使用对称 `22px` 内边距和左右 `86px` 控制区，中央播放控制严格居中、右侧窗口控制贴近边界。浏览器在 480px/760px 验证工具栏对称、零横向溢出，以及纯数字字号输入的 Enter/失焦/Escape 行为；原生左右拖动手感仍需 Windows 实机验收。
- 2026-07-22：上述修订通过前端 build、许可证、Rust format/test/Clippy、差异检查及 Windows Release 主窗口可见性门禁；59 个普通 Rust 测试通过、8 个真实设备测试按约定忽略。最终产物为 `src-tauri/target/release/resona.exe`，大小 17,932,288 bytes，SHA-256 `BD94A1A84D2F223B202CF6C50D3710E7DE3DB1BB08B08571B7E9EF8B19942991`。
- 2026-07-22：项目所有者确认桌面歌词握柄、字号输入和工具栏功能正常，但多显示器位置恢复失败。根因是保存窗口横跨副屏与主屏时，旧算法选择枚举中的首个相交屏幕，并在移动前使用主屏 DPI 计算尺寸，最终将副屏负坐标夹到 `x=0`。现按最大可见交集面积选择目标屏，完全离屏时选最近屏幕，尺寸约束使用目标屏 DPI，并在跨屏 DPI 转换后重新设置物理原点。完整前端 build、Rust format/test/Clippy 通过；61 个普通测试通过、8 个设备测试按约定忽略，修复待 Windows 多显示器实机复验。
- 2026-07-22：多显示器修订的 Windows Release 已通过主窗口可见性检查。产物为 `src-tauri/target/release/resona.exe`，大小 17,944,064 bytes，SHA-256 `51157E90A3D3B93B5E135611B8FF678937A5B0AD186348AD159180DE608B970A`；仍需项目所有者执行“副屏定位、退出、重开播放”的原生位置恢复验收。
- 2026-07-22：项目所有者确认桌面歌词多显示器位置恢复正常。窄屏 Drawer 顶部已由重复“导航”标题改为复用宽屏 `Resona` 品牌锁定；歌词字号设置改为标签与 112px 数字输入同排，不再拉伸到整行。浏览器覆盖 360×600/420×720、中英文和浅色/深色，标题与关闭按钮保持 130px 以上间距，字号控件无跳动，页面横向溢出为 0，控制台无 warning/error。
- 2026-07-22：窄屏侧栏与字号修订的 Windows Release 已通过主窗口可见性检查。产物为 `src-tauri/target/release/resona.exe`，大小 17,944,064 bytes，SHA-256 `1597E8E2180940E59B8F19506E41D0EEC8AF539DB89F06D30649AAAF6EE28F00`。
- 2026-07-22：修复默认列表播放命令仍提交旧数组下标导致的失败，现与用户列表一样提交稳定项目 ID，并由 Rust 权威列表解析目标。默认列表非空时，资源管理器拖入多个文件/文件夹可按曲目行中线插入，使用与内部排序一致的扩展间隔和插入线；当前默认列表同步更新 Rodio 执行序列，标题和控件不进入拖放热区。
- 2026-07-22：音频压缩档位与“成功后删除源 WAV”开关通过现有 UI 偏好边界持久化，默认仍为“平衡 / 开启删除”；扫描根、候选文件和任务状态不持久化。浏览器重载验证通过。完整门禁为 63 个普通 Rust 测试通过、8 个真实设备测试按约定忽略，TypeScript/Vite build、许可证、Rust format、Clippy `-D warnings`、差异检查和 Windows Release 主窗口可见性检查通过。产物为 17,998,848 bytes，SHA-256 `D02E17C316969A1DB740D117812962E56F61E0ED39EA11BDE0B47A0E78D4E330`。
