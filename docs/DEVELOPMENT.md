# 开发准则

## Windows 环境

0.0.1 使用以下本机工具链：

- Node.js 26 与 npm 11
- Rust 1.97.1（项目最低版本见 `src-tauri/Cargo.toml`）
- Visual Studio Build Tools 2022，包含 MSVC/C++ 桌面工具
- Microsoft Edge WebView2 Runtime

首次安装依赖使用 `npm ci`，随后运行 `npm run prepare:sidecars` 下载并校验固定 FFmpeg/ffprobe。两个约 97 MiB 的可执行文件不进入 Git；`npm run tauri dev`、`npm run tauri build` 和 `npm run release:windows` 也会自动执行该准备步骤。Rust 命令应在已加载 MSVC 环境的 Developer PowerShell/Command Prompt 中运行。

常用验证命令：

```powershell
npm run build
npm run licenses
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run release:windows
```

fixture 重新生成需要 FFmpeg 8.1.2+ 与 Xiph FLAC 1.5.0+，详见 `tests/fixtures/audio/README.md`。日常测试不依赖编码器，直接使用已提交的二进制样本。

需要真实默认音频设备的 smoke test 默认忽略，显式执行：

```powershell
cargo test --manifest-path src-tauri/Cargo.toml opens_default_output_and_accepts_a_flac -- --ignored
```

开发运行使用 `npm run tauri dev`。单独执行 `npm run dev` 只预览 Web 前端，没有 Tauri 文件对话框或 Rust 播放能力。

Windows 交付版本只能通过 `npm run release:windows` 生成。不得在 Tauri 构建后用 `cargo build --release` 重建或覆盖 `src-tauri/target/release/resona.exe`：普通 Cargo 构建不会注入 Tauri 生产协议，生成的 WebView 会错误访问 `devUrl`。`release:windows` 在构建后先确认 Vite 开发服务器未运行，再启动可执行文件检查主窗口存活，并自动关闭测试实例；交付前仍需浏览器或人工确认窗口内容。

`scripts/prepare-ffmpeg-sidecars.ps1` 固定下载 FFmpeg 8.1.2 essentials archive，并分别校验归档、ffmpeg 和 ffprobe 的 SHA-256。下载文件只写入已忽略的 `src-tauri/binaries/*.exe`；修改版本、来源或任一哈希前必须重新审查许可证、构建选项和转换回归。自动化构建应调用 `npm run tauri -- build` 使用同一准备路径，不依赖仓库内二进制。

0.1.0 功能开发完成前不维护 GitHub Actions；当前由本地自动检查和阶段验收提供反馈。CI、分支保护和 Action 依赖更新策略在 0.0.20 发布加固阶段按实际发布渠道建立，启用时必须重新核对所有 Action 的最新稳定版本、运行时和权限，并优先固定完整 commit SHA。

涉及前端的改动至少要在独立浏览器中检查一次页面加载、控制台错误和主要布局状态。浏览器预览不替代 Tauri command、窗口、媒体键、透明穿透等原生验收，但必须先拦截白屏、根组件崩溃、资源路径和明显布局问题。

## Windows 原生材质

- 普通窗口材质只通过 `src-tauri/src/platform/window_material.rs` 接入。Windows 11 主窗口和音频压缩窗口使用 Tauri 内置 Mica；不得再加入 window-vibrancy 插件、全局 CSS `backdrop-filter`、Acrylic 全窗口模糊或自绘标题栏。
- Windows 主窗口透明创建配置只写入 `tauri.windows.conf.json`；通用配置保持实色默认。新增普通辅助窗口必须通过同一 adapter 决定透明背景、Mica 和查询参数，不能在 React 中判断操作系统。
- 页面使用 `--resona-window-background`、`--resona-chrome-surface`、`--resona-content-surface`、`--resona-subtle-surface` 与 `--resona-hairline`。不得在 feature 中复制带 alpha 的平台色值。
- Mica 页面遵循单层材质模型：根窗口承载唯一的大面积材质，侧栏、导航、主内容、完整播放器、底栏和压缩工作区不得再覆盖连续半透明遮罩；区域分隔优先使用 `--resona-hairline`。每个区域最多增加一层有明确交互或状态用途的 `subtle surface`，不得用嵌套半透明背景制造卡片层级。
- Mantine 全局 `body` 背景不消费 Resona 的窗口材质 token。Mica 路径必须通过 `html[data-window-material="mica"]` 同时覆盖 `html`、`body` 和 `#root` 为透明；不要把 `--mantine-color-body` 改为透明，否则组件实体表面会一起丢失。调整 CSS import 顺序或 Provider 时必须复测该覆盖优先级。
- 设置页分组默认透明、无阴影、无外框，通过标题、间距和相邻分组细线建立层级。只有独立重复项目、菜单、弹层、错误/选中状态等确需边界的内容才使用实体表面；不为模仿 WinUI 引入第二套组件库或自绘控件体系。
- Mica 初始化和保存的原生窗口主题必须在窗口首次可见前就绪；失败日志必须可诊断并回退 `solid`。主题偏好必须以 `auto`/`light`/`dark` 传到原生边界，`auto` 使用 `set_theme(None)`，不得用受当前窗口覆盖影响的 `prefers-color-scheme` 结果代替。浅色、深色和跟随系统都要检查 WebView 表面与原生标题栏是否一致。
- 桌面歌词是例外：保持透明、置顶、穿透和 helper 架构，不启用 Mica。设计语言只同步字体、圆角、按钮状态、边界和动效，歌词正文透明度仍由独立偏好控制。

## Windows 文件选择对话框

Tauri dialog 2.7.1 在 Windows 通过 rfd 0.16.0 调用系统 `IFileOpenDialog`，并自动把当前 Tauri 窗口设为 parent。前端必须保证同一窗口最多存在一个文件选择请求；对话框完成或失败后再解除互斥。

不要给原生对话框增加仅在 JavaScript 层生效的超时。该超时无法关闭 Windows COM 对话框，只会让应用错误地认为窗口已经关闭并允许叠加第二个模态窗口。若单个对话框偶发长时间无响应，应分别检查 Windows Explorer、快速访问中的不可达网络路径和第三方 Shell 扩展，并记录复现时的目录位置。

## Windows 拖放

Windows 主 WebView 保持 Tauri 原生拖放处理器启用，用它接收资源管理器提供的真实文件系统路径。Tauri 返回物理像素坐标，命中 DOM 前必须使用当前 Tauri 窗口的 `scaleFactor()` 转为 WebView 客户区逻辑坐标；不得假定 `devicePixelRatio` 在所有系统缩放和显示器切换场景下始终等价。

Tauri 原生拖放处理器与 WebView2 HTML5 drag-and-drop 在 Windows 上存在明确冲突。播放列表内部排序不得使用 HTML `draggable` 与 `dataTransfer`；使用 Pointer Events、指针捕获和列表几何位置表达内部移动，且不得产生可拖出应用的文件或曲目对象。外部路径解析、格式校验、批量插入和持久化仍由 Rust 服务负责，React 只传递目标列表与插入位置。列表详情的外部拖放热区只覆盖空列表或曲目滚动区域，标题、重命名输入和右侧操作控件不得成为导入目标。

## 优先级

发生取舍时按以下顺序判断：

1. 数据与文件安全
2. 播放正确性和可诊断性
3. 用户体验完整性
4. 长期维护成本
5. 性能
6. 初始实现速度

“少写代码”指减少自有维护面，不意味着省略错误处理、测试或迁移。

## 依赖准入

允许商业生态，但免费核心必须满足：

- 不要求注册、申请许可证或注入密钥
- 不要求联网激活
- 不显示水印、遮罩或授权提示
- 不把许可证校验或强制遥测注入最终应用
- 免费能力足以承担选定职责
- 许可证与 GPL-3.0-only 项目分发兼容

引入核心依赖前记录许可证、维护活跃度、替代方案、二进制体积和退出成本。大版本升级不得自动合并，必须重新检查运行时行为和许可边界。

## 依赖使用

- 首先使用标准库、现有依赖或外部工具已提供的能力。
- 不同时引入两套完整 UI 组件库、状态库或数据库层。
- adapter 解决真实的变化点；禁止为“以后可能替换”预建空泛接口。
- FFmpeg sidecar 版本固定并记录来源、构建选项、许可证和校验值。
- Rodio、CPAL、Symphonia 使用稳定发布版和最小 feature 集；Cargo.lock 记录实际版本。
- 不因上游发布新版本自动升级音频栈，升级必须通过完整播放 fixture 回归。
- 外部参考快照放在 `docs/vendor`，必须带来源与获取日期。

## 性能预算原则

- 音频流不进入 WebView，也不跨 Tauri command/event 传输。
- 音频输出由专用 Rust actor 持有；音频回调不等待应用锁、不访问数据库、不发送 UI 事件。
- 长播放列表和 Rodio 内部执行序列必须采用有界渲染/处理；达到实测阈值时使用成熟虚拟化方案。
- 高频进度事件按视觉需要节流，避免每个音频 tick 触发全局 React 更新。
- 播放/暂停、进度和歌词更新只允许重渲染对应播放器区域；普通列表、侧栏、设置和工具视图必须使用稳定引用与 memo 边界隔离，不得在即时播放 command 期间切换全局 loading/disabled 状态造成整页闪烁。
- 目录扫描不得为每个候选文件启动外部进程。WAV 扫描使用进程内头解析；FFmpeg/ffprobe 仅用于实际转换和结果校验，并在 Windows 隐藏控制台窗口。
- 播放列表数据库操作使用事务并维护稠密位置；当前曲目封面按需读取并限制缓存，不建立全局媒体索引。
- 性能优化以测量结果为依据，不通过牺牲边界和正确性换取不可见收益。

## 测试层次

- 纯领域逻辑：快速单元测试
- Rodio/CPAL/Symphonia、FFmpeg、SQLite adapter：固定 fixture 的集成测试
- Tauri command/event：契约测试
- 关键用户流程：桌面 E2E
- UI：浅色/深色、主题色、中文/英文、缩放、空/加载/错误状态截图验证
- 应用壳：空默认列表、多个用户列表、完整播放主内容视图、长路径和长翻译文本下不得发生布局跳动或溢出；可选底栏按钮出现前后必须保持固定几何；播放/暂停前后普通主内容 DOM 不得重挂载或重新触发页面入场动画
- 列表交互：应用正文默认禁止文本选择；曲目单击选择、Ctrl 切换、Shift 范围选择、双击播放，列表空白处单击清除选择；不实现拖动框选。曲目整行在超过移动阈值后进入 Pointer Events 排序，拖动开始时收束为当前曲目单选，并通过扩展间隔和插入线反馈目标位置；不能重新引入 HTML5 `draggable` 或干扰 Tauri 原生文件拖入。
- 右键菜单：应用内容区不得依赖浏览器原生菜单；曲目、空列表区和侧栏播放列表提供上下文操作。可编辑文本必须保留等价的剪切、复制、粘贴、全选能力或系统编辑菜单。
- 完整播放器：当前歌词自动居中、无横向滚动条，歌词/播放信息面板和顶部切换控件尺寸固定；输入元数据和实际 CPAL 输出流配置分别单行展示
- 播放定位：使用延迟 command/快照验证 Slider 点击、拖动和连续 seek 不回跳；歌词行点击与键盘激活必须提交准确 `startMs`，失败路径恢复权威位置并可诊断
- 滚动容器：主窗口所有可见滚动轨道使用 Mantine `ScrollArea`；验证滚轮、触控板、滚块拖动、键盘滚动和活动歌词居中，不允许父子双滚动
- 主窗口布局：宽屏验证 `1080×700`/`760×520`，窄屏验证 `420×720`/`360×600`；两种模式均覆盖中英文、明暗主题、长文本和完整功能可达性
- 主窗口几何：Windows 原生验证两种布局各自的位置/尺寸/最大化恢复、多显示器可见区域修正和 100%/125%/150% DPI；浏览器截图不能替代该项
- 原生材质：浏览器以 `material=mica` 检查根材质、大区域透明、局部表面不嵌套、hairline、设置分组、明暗对比度和溢出；至少覆盖宽屏主界面、窄屏设置/工具、完整播放器和压缩窗口。Windows 11 Release 还必须截图或采样主内容空白区，确认没有继续输出 `solid` 回退色，再检查主窗口/压缩窗口 Mica、主题切换、冷启动、最大化/还原和阴影；Windows 10/Linux 检查完整实色回退
- 启动首帧：冷/暖启动、浅色/深色/跟随系统下不得出现明显白色空窗；隐藏到首帧的窗口必须有 Release 启动失败检测
- Windows SMTC：系统回调必须通过 typed command 进入播放服务，并在实机验证媒体键与系统显示状态
- Windows 桌面歌词：透明、置顶、焦点、跨进程点击/滚轮/拖动穿透和原生解锁辅助窗口必须实机验证；浏览器预览不能替代
- Windows 应用生命周期：关闭主窗口、隐藏歌词资源释放、重复启动单实例和退出后无残留进程必须单独验证；WebView2 子进程数量不能作为播放核心重复的判断依据
- Windows Release：必须通过 `npm run release:windows` 构建并验证内嵌前端；仅通过 `npm run build` 或普通 `cargo build --release` 不构成交付验证

音频 fixture 必须体积小、来源清晰并允许重新分发。

首版播放 fixture 至少覆盖：

- MP3、PCM WAV、FLAC
- 44.1 kHz/16-bit、48 kHz/24-bit、96 kHz/24-bit、192 kHz/24-bit
- 连续专辑、混合采样率队列、定位、暂停恢复和损坏文件
- 默认设备、指定设备、蓝牙断开和输出设备变化

AAC/M4A 只有在启用对应 feature 后才进入发布测试矩阵。

## 完成定义

一项工作只有同时满足以下条件才算完成：

- 行为符合验收条件
- 错误路径可诊断
- 相应测试或验证已执行
- 没有引入未说明的许可证或运行时验证机制
- `docs/STATUS.md` 已更新
- 长期架构决定已记录为 ADR
