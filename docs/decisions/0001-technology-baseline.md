# ADR 0001：技术栈基线

- 状态：Accepted
- 日期：2026-07-17

## 背景

Resona 需要以尽可能低的自有开发成本实现完整、可靠且美观的本地高解析度播放器。首版只开发 Windows，但不能让未来 Wayland + PipeWire 实现要求重写业务层。

## 决定

- Tauri 2 负责桌面壳、系统集成和 Rust 后端。
- React + TypeScript + Vite 负责多窗口 UI。
- Mantine 是主要组件库；Zustand 管理临时 UI 状态；TanStack Virtual 处理长列表；Lucide 提供图标。
- Rodio + CPAL + Symphonia 负责首版本地音频播放，具体边界见 ADR 0005。
- SQLite 保存媒体库，lofty-rs 处理标签，FFmpeg sidecar 处理转换。
- Windows 首版实现 WASAPI；Linux 仅保留能力边界。

## 理由

这些项目提供成熟实现和活跃生态。Mantine 的完整组件、hooks、普通 CSS 和机器可读开发文档降低 AI 主导开发中的 API 误用和视觉不一致风险。

## 后果

- UI 使用 React 生态，不再同时维护 Vue 方案。
- 平台和第三方能力必须通过 adapter/capability 收口。
- 外部依赖升级需遵守许可证和运行时行为审查。
