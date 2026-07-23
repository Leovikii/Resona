# Resona Development Instructions

开始开发前，按顺序阅读：

1. `docs/STATUS.md`
2. `docs/ARCHITECTURE.md`
3. `docs/STRUCTURE.md`
4. `docs/DEVELOPMENT.md`
5. 与当前任务相关的 `docs/decisions/` 记录

本机如存在 `.local-docs/llms-full.txt`，前端组件开发时优先用 `rg` 按组件或 API 名称检索所需章节；该版本化参考只保留在本地并由 `.gitignore` 排除，不上传仓库。缺失时以当前锁定版本的 Mantine 官方文档和 TypeScript 类型为准。

开发约束：

- 优先使用成熟库、Tauri 官方能力和 Rodio/CPAL/Symphonia/FFmpeg 已有功能。
- 不在前端复制播放、媒体库或转换领域逻辑；Rust 后端是业务状态的权威来源。
- 不让 Tauri command handler、React 页面或 Mantine 组件直接承载领域逻辑。
- Mantine 只允许在 `src/app` 的 provider/theme 和 `src/shared/ui` 中直接导入。功能模块通过项目组件使用它；简单局部布局例外需说明理由。
- 不为每个 Mantine 组件创建无行为包装层。只有需要统一默认值、无障碍、错误处理、平台行为或领域语义时才建立适配组件。
- 新增平台特定能力时，通过 capability 或 adapter 隔离，不在通用模块中散布 `cfg`/平台判断。
- 不自动升级依赖的大版本；升级前审查许可证、运行时激活、遥测、迁移成本和离线行为。
- 完成任务后更新 `docs/STATUS.md`；形成长期约束的决定时新增或更新 ADR。
- 文档中的“当前状态”必须与代码和测试结果一致。

质量要求：

- 保持错误可诊断，禁止静默吞错。
- 共享边界使用类型化数据，不向 UI 暴露 Rodio、CPAL、Symphonia 类型或数据库行。
- 首版只实现 `RodioPlaybackEngine`；未经新 ADR 不增加第二套播放引擎或 mpv fallback。
- 测试覆盖与风险相称；涉及播放生命周期、数据库迁移、文件写入和转换的改动必须有自动验证。
- UI 变更需要在常用桌面尺寸、缩放和暗色模式下做视觉验证。
