# 计划目录结构

该结构在初始化应用时逐步创建；当前不为尚未存在的模块添加空目录。

## 0.0.1 实际结构

最小验证刻意不提前建立完整分层。当前可执行代码只有以下边界：

```text
src/
├─ App.tsx                     # 单窗口最小播放界面与 Tauri 调用
├─ main.tsx                    # React/Mantine 启动和主题
└─ styles.css                  # 最小窗口布局
src-tauri/
├─ capabilities/default.json  # 主窗口与文件选择权限
├─ src/
│  ├─ commands.rs             # Tauri command 边界
│  ├─ playback/mod.rs         # 契约、Rodio actor、状态与测试
│  ├─ lib.rs                  # Tauri 组装
│  └─ main.rs                 # 桌面入口
├─ Cargo.toml
└─ tauri.conf.json
tests/
└─ fixtures/audio/             # 可再生成的 WAV/FLAC/MP3 与边界样本
scripts/
├─ generate-audio-fixtures.ps1
└─ generate-license-report.mjs
.github/workflows/ci.yml       # Windows 构建、测试与 Clippy
```

`App.tsx` 在本技术验证中直接使用 Mantine，避免为一次性界面建立无收益的包装层。进入多窗口或第二个前端 feature 时，再按下述目标结构拆分 `app`、`features` 与 `shared/ui`。

## 目标结构

```text
Resona/
├─ src/                         # React 前端
│  ├─ app/                     # 启动、路由、provider、theme
│  ├─ windows/                 # 主窗口、桌面歌词窗口入口
│  ├─ features/                # playback、library、lyrics、settings...
│  ├─ shared/
│  │  ├─ bridge/               # 类型化 Tauri command/event 客户端
│  │  ├─ model/                # 前端共享类型和纯转换
│  │  ├─ ui/                   # Resona UI 与必要的 Mantine 适配
│  │  └─ utils/                # 无领域状态的工具
│  └─ assets/                  # 字体、图像和静态资源
├─ src-tauri/
│  ├─ src/
│  │  ├─ app/                  # Tauri 启动、commands、events
│  │  ├─ domain/               # 模型、错误和核心契约
│  │  ├─ services/             # 播放、媒体库、歌词、元数据、转换
│  │  ├─ adapters/             # Rodio、FFmpeg、SQLite、文件系统
│  │  └─ platform/             # Windows；未来 Wayland/PipeWire
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
