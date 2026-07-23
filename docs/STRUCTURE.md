# 计划目录结构

该结构在初始化应用时逐步创建；当前不为尚未存在的模块添加空目录。

## 当前实际结构（截至 0.1.0-rc.1）

应用已按实际变化边界拆分前端与 Rust 模块：

```text
assets/
├─ resona-icon.svg             # 透明单 R 应用图标源
├─ resona-r-mark.svg           # 紧裁画布的标准单 R 设计源
├─ resona-gothic-wordmark.svg  # 主界面横向字标
├─ resona-file-flac.svg        # FLAC 文件关联图标源（待注册）
├─ resona-file-wav.svg         # WAV 文件关联图标源（待注册）
├─ resona-file-mp3.svg         # MP3 文件关联图标源（待注册）
└─ FILE_ICONS.md               # 格式图标映射和栅格化约定

src/
├─ App.tsx                     # 应用入口兼容转发，不承载页面逻辑
├─ main.tsx                    # React 启动、provider 和 i18n 初始化
├─ app/
│  ├─ App.tsx                 # 应用壳、视图组合与 Mantine 局部布局
│  └─ preferences.tsx         # 主题、主题色和语言 provider
├─ features/
│  ├─ lyrics/                 # 桌面歌词窗口状态与权威正在播放快照 hooks
│  ├─ library/                # 用户播放列表、项目操作与最近播放 hooks
│  ├─ playback/               # 播放控制与无回跳 seek transaction hooks
│  ├─ update/                 # GitHub Releases 更新检查、安装和进度 hook
│  └─ window/                 # 主窗口布局模式 typed hook；不持有原生坐标
├─ shared/
│  ├─ bridge/                 # 类型化 Tauri command/event 与单实例文件选择边界
│  │  └─ windowAppearance.ts  # 当前窗口材质初始化、DOM 语义与原生主题同步
│  ├─ i18n/                   # zh-CN/en 资源与 i18next 初始化
│  ├─ model/                  # 播放、播放列表、歌词和桌面窗口共享契约
│  ├─ ui/
│  │  ├─ AddMediaMenu.tsx     # 播放列表/压缩窗口统一添加菜单
│  │  ├─ OverflowMarquee.tsx  # 少量高价值单行文本的按溢出滚动展示
│  │  ├─ PlaylistTrackList.tsx # 默认/用户列表共享曲目交互
│  │  └─ usePointerReorder.ts # 不依赖组件库的播放列表指针排序行为
│  ├─ ui/useDocumentVisibility.ts # 隐藏 WebView 轮询休眠边界
│  └─ utils/format.ts         # 时间等纯格式化函数
├─ windows/
│  └─ DesktopLyricsWindow.tsx # 不加载应用壳的轻量桌面歌词入口
└─ styles.css                 # 响应式应用壳、语义色与动画
src-tauri/
├─ capabilities/default.json  # 主窗口与文件选择权限
├─ capabilities/desktop-lyrics.json # 桌面歌词 WebView 最小权限
├─ src/
│  ├─ commands.rs             # 薄 Tauri command 边界
│  ├─ application_update.rs   # GitHub 原生 Release 发现、SemVer 通道、签名更新
│  ├─ filesystem.rs           # 直属音频上下文枚举与拖入路径展开
│  ├─ lyrics.rs               # LRC/SRT/WebVTT 发现、解码、解析、缓存和同步
│  ├─ main_window.rs          # 主窗口宽/窄模式、两套几何、可见约束与首帧 ready
│  ├─ media_import.rs         # 默认列表、当前列表来源、执行序列同步与 external-open 协调
│  ├─ metadata.rs             # 后续按需元数据 adapter 源文件，当前不编译
│  ├─ persistence.rs          # 用户列表、最近历史与 schema v3 migration
│  ├─ playlists.rs            # 播放列表导入、命名和事务编排服务
│  ├─ platform/
│  │  ├─ mod.rs              # 平台能力模块入口
│  │  ├─ desktop_lyrics.rs   # 桌面歌词 capability、状态与跨平台边界
│  │  ├─ desktop_lyrics/
│  │  │  └─ windows.rs       # Tauri 歌词窗与 Win32 原生解锁辅助窗
│  │  ├─ media_session.rs    # SMTC/MPRIS capability 边界
│  │  ├─ media_session/
│  │  │  └─ windows.rs       # souvlaki Windows SMTC adapter
│  │  └─ window_material.rs  # Windows Mica/跨平台实色回退与主题同步
│  ├─ playback/
│  │  ├─ mod.rs              # 播放契约、Rodio actor、内部执行序列与测试
│  │  └─ output.rs           # CPAL 输出枚举、选择与错误回调
│  ├─ lib.rs                  # Tauri 组装
│  └─ main.rs                 # 桌面入口
├─ Cargo.toml
├─ tauri.conf.json            # 跨平台不透明窗口默认
└─ tauri.windows.conf.json    # Windows 主窗口透明创建属性
tests/
└─ fixtures/
   ├─ audio/                  # 可再生成的 WAV/FLAC/MP3 与边界样本
   └─ lyrics/                 # LRC/SRT/WebVTT 文本样本
scripts/
├─ generate-audio-fixtures.ps1
├─ generate-license-report.mjs
├─ prepare-ffmpeg-test-tools.ps1 # 仅为真实转换回归准备被忽略的固定 FFmpeg 测试工具
├─ finalize-windows-artifacts.ps1 # 生成带平台/架构的 NSIS 产物名与 SHA-256 元数据
├─ release-channel.mjs        # 版本一致性、SemVer prerelease 与发布判定
├─ verify-windows-distribution.ps1 # 审计版本、身份、关联、无 bundled FFmpeg 与安装器元数据
└─ verify-release-webview.ps1 # 验证 Release 主窗口稳定可见；可选 DPI-aware 原生截图
```

Mantine 直接导入仍限制在 `src/app` 与 `src/shared/ui`；`AddMediaMenu` 统一跨窗口添加来源行为，`PlaylistTrackList` 统一默认/用户列表选择与排序，`CompactTopNavigation` 负责窄屏顶部全局导航和播放列表 Tabs，`OverflowMarquee` 只负责按尺寸判断的单行展示，`usePointerReorder.ts` 是不导入 Mantine、Tauri 或领域服务的纯 Pointer Events 行为。不为每个 Mantine 组件创建空包装层。

0.0.13 已删除 managed-folder/media-library 运行时代码；`src/features/library` 保持为播放列表与最近历史，不为目录整洁做无收益搬迁。桌面歌词仍通过通用 facade 隔离 Windows 实现。0.0.17 的 `src/windows/AudioCompressionWindow.tsx` 只提供轻量窗口入口，Mantine 组合位于 `src/app/AudioCompressionApp.tsx`；Rust `compression_window.rs` 只管理普通辅助窗口生命周期，扫描和转换仍归 `CompressionService`。

ADR 0020 已由 `main_window.rs` 与 `features/window` 实现：Rust 保存模式和原生几何，React 只消费布局快照。当前 ScrollArea 组合没有形成重复默认值或复杂行为，因此继续在 `src/app` 直接组合 Mantine，不增加无收益包装层。

ADR 0022 由 `platform/window_material.rs`、`shared/bridge/windowAppearance.ts` 和 `styles.css` 的语义表面 token 实现。Windows 版本判断与 Tauri Window Effects 在 Rust 边界终止；React 不出现 Windows 条件分支。Mica 下导航与底栏 token 透明，组成连续外壳；主内容使用唯一 content surface，独立工作单元使用 subtle surface，普通列表行保持无描边透明。完整播放器在主内容内保持透明，桌面歌词继续使用独立透明平台边界。`verify-release-webview.ps1` 负责等待稳定主窗口并可选抓取 Per-Monitor v2 DPI 原生截图，避免浏览器预览替代 Windows 材质验收。

## 目标结构

```text
Resona/
├─ src/                         # React 前端
│  ├─ app/                     # 启动、路由、provider、theme
│  ├─ windows/                 # 主窗口、桌面歌词和音频压缩工作窗口入口
│  ├─ features/                # playback、playlists、recent、lyrics、compression、settings...
│  ├─ shared/
│  │  ├─ bridge/               # 类型化 Tauri command/event 客户端
│  │  ├─ i18n/                 # zh-CN/en 资源、locale 解析与格式化
│  │  ├─ model/                # 前端共享类型和纯转换
│  │  ├─ ui/                   # Resona UI 与必要的 Mantine 适配
│  │  └─ utils/                # 无领域状态的工具
│  └─ assets/                  # 字体、图像和静态资源
├─ src-tauri/
│  ├─ src/
│  │  ├─ app/                  # Tauri 启动、commands、events
│  │  ├─ domain/               # 模型、错误和核心契约
│  │  ├─ services/             # 播放、列表、最近历史、歌词、按需元数据、压缩扫描/任务
│  │  ├─ adapters/             # Rodio、FFmpeg、SQLite、文件系统
│  │  └─ platform/             # Windows SMTC/桌面歌词；通用辅助窗口在 app 层组装
│  ├─ migrations/              # SQLite migrations
│  ├─ binaries/                # 已忽略的 FFmpeg 真实转换测试工具；不进入 bundle
│  ├─ capabilities/            # Tauri capability 配置
│  └─ tests/                   # Rust 跨模块集成测试
├─ tests/
│  ├─ e2e/                     # 桌面关键流程
│  └─ fixtures/                # 小型、可再分发的音频和标签样本
  ├─ docs/
  │  ├─ decisions/               # ADR
  │  ├─ performance/             # 版本性能与音频审计证据
  │  └─ AGENT_GUIDE.md           # AI Agent 完整开发规则
├─ scripts/                    # 可重复的开发、检查和打包脚本
├─ AGENTS.md                   # 开发代理入口约束
├─ README.md
  └─ LICENSE
  ```

`.local-docs/` 是本机忽略目录，可保存与当前依赖版本对应的大型组件参考（当前为 Mantine `llms-full.txt`）；它不参与构建、测试或发布，也不上传 GitHub。

## 所有权规则

- `features` 可以依赖 `shared`，不同 feature 不直接读取彼此内部状态。
- 跨 feature 的稳定概念进入 `shared/model`；不要把所有代码提前提升为共享代码。
- `commands` 只做输入验证、授权边界和服务调用，不写业务流程。
- `domain` 不导入 Tauri、数据库、Rodio、CPAL、Symphonia 或平台 crate。
- `adapters` 实现领域契约，第三方类型在 adapter 内终止。
- `adapters/playback` 内部包含 audio actor、Rodio engine、设备管理和 decoder 配置，不把音频库类型导出到应用层。
- `platform` 只放无法由跨平台 API可靠完成的代码。

## 文件命名

- React component：`PascalCase.tsx`
- hooks：`useSomething.ts`
- 普通 TypeScript module：`camelCase.ts`
- Rust module：`snake_case.rs`
- 测试与被测模块相邻；跨模块流程放入对应 `tests/`
- ADR：`NNNN-short-title.md`
