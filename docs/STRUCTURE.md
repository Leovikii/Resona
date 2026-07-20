# 计划目录结构

该结构在初始化应用时逐步创建；当前不为尚未存在的模块添加空目录。

## 当前实际结构（截至 0.0.13）

应用已按实际变化边界拆分前端与 Rust 模块：

```text
src/
├─ App.tsx                     # 应用入口兼容转发，不承载页面逻辑
├─ main.tsx                    # React 启动、provider 和 i18n 初始化
├─ app/
│  ├─ App.tsx                 # 应用壳、视图组合与 Mantine 局部布局
│  └─ preferences.tsx         # 主题、主题色和语言 provider
├─ features/
│  ├─ lyrics/                 # 桌面歌词窗口状态与权威正在播放快照 hooks
│  ├─ library/                # 用户播放列表、项目操作与最近播放 hooks
│  └─ playback/usePlaybackController.ts
├─ shared/
│  ├─ bridge/                 # 类型化 Tauri command/event 与单实例文件选择边界
│  ├─ i18n/                   # zh-CN/en 资源与 i18next 初始化
│  ├─ model/                  # 播放、播放列表、歌词和桌面窗口共享契约
│  └─ utils/format.ts         # 时间等纯格式化函数
├─ windows/
│  └─ DesktopLyricsWindow.tsx # 不加载应用壳的轻量桌面歌词入口
└─ styles.css                 # 响应式应用壳、语义色与动画
src-tauri/
├─ capabilities/default.json  # 主窗口与文件选择权限
├─ capabilities/desktop-lyrics.json # 桌面歌词 WebView 最小权限
├─ src/
│  ├─ commands.rs             # 薄 Tauri command 边界
│  ├─ filesystem.rs           # 直属音频上下文枚举与拖入路径展开
│  ├─ lyrics.rs               # LRC/SRT/WebVTT 发现、解码、解析、缓存和同步
│  ├─ media_import.rs         # 临时默认列表与统一 external-open 应用服务
│  ├─ metadata.rs             # 后续按需元数据 adapter 源文件，当前不编译
│  ├─ persistence.rs          # 用户列表、最近历史与 schema v3 migration
│  ├─ playlists.rs            # 播放列表导入、命名和事务编排服务
│  ├─ platform/
│  │  ├─ mod.rs              # 平台能力模块入口
│  │  ├─ desktop_lyrics.rs   # 桌面歌词 capability、状态与跨平台边界
│  │  ├─ desktop_lyrics/
│  │  │  └─ windows.rs       # Tauri 歌词窗与 Win32 原生解锁辅助窗
│  │  ├─ media_session.rs    # SMTC/MPRIS capability 边界
│  │  └─ media_session/
│  │     └─ windows.rs       # souvlaki Windows SMTC adapter
│  ├─ playback/
│  │  ├─ mod.rs              # 播放契约、Rodio actor、队列与测试
│  │  └─ output.rs           # CPAL 输出枚举、选择与错误回调
│  ├─ lib.rs                  # Tauri 组装
│  └─ main.rs                 # 桌面入口
├─ Cargo.toml
└─ tauri.conf.json
tests/
└─ fixtures/
   ├─ audio/                  # 可再生成的 WAV/FLAC/MP3 与边界样本
   └─ lyrics/                 # LRC/SRT/WebVTT 文本样本
scripts/
├─ generate-audio-fixtures.ps1
├─ generate-license-report.mjs
├─ prepare-ffmpeg-sidecars.ps1 # 下载并校验不进入 Git 的固定 FFmpeg sidecar
└─ verify-release-webview.ps1 # 验证 Release 主窗口实际可见并清理进程
.github/workflows/ci.yml       # Windows 构建、测试与 Clippy
```

Mantine 直接导入仍限制在 `src/app`；当前没有出现需要统一平台行为或领域语义的复用控件，因此不创建空的 `shared/ui` 包装层。后续出现真实复用边界时再按目标结构增加。

0.0.13 已删除 managed-folder/media-library 运行时代码；`src/features/library` 保持为播放列表与最近历史，不为目录整洁做无收益搬迁。桌面歌词仍通过通用 facade 隔离 Windows 实现。0.0.17 的 `src/windows/AudioCompressionWindow.tsx` 只提供轻量窗口入口，Mantine 组合位于 `src/app/AudioCompressionApp.tsx`；Rust `compression_window.rs` 只管理普通辅助窗口生命周期，扫描和转换仍归 `CompressionService`。

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
│  ├─ binaries/                # 按 target 分发的 FFmpeg sidecar
│  ├─ capabilities/            # Tauri capability 配置
│  └─ tests/                   # Rust 跨模块集成测试
├─ tests/
│  ├─ e2e/                     # 桌面关键流程
│  └─ fixtures/                # 小型、可再分发的音频和标签样本
├─ docs/
│  ├─ decisions/               # ADR
│  └─ vendor/                  # 带来源信息的外部开发参考
├─ scripts/                    # 可重复的开发、检查和打包脚本
├─ AGENTS.md                   # 开发代理入口约束
├─ README.md
└─ LICENSE
```

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
