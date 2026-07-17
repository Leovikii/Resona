# ADR 0002：mpv sidecar 播放架构

- 状态：Superseded by [ADR 0005](0005-rodio-playback-core.md)
- 日期：2026-07-17

## 背景

首版是纯音频播放器，不需要把 mpv 视频渲染嵌入 WebView。直接使用 libmpv 会引入 FFI、ABI、线程回调、构建和平台窗口集成成本。

## 决定

首版将 mpv 作为随应用分发的 sidecar，通过 JSON IPC 控制。Rust 后端实现 `PlaybackEngine` 契约，并由 `MpvSidecarEngine` 适配 mpv。

## 理由

- 保留 mpv 的成熟解码、输出和播放能力。
- 进程崩溃与主应用隔离，可独立诊断和重启。
- 音频不经过 IPC，sidecar 不降低输出质量。
- 未来 Linux 主要替换二进制、IPC endpoint 和输出配置。
- libmpv 对当前功能的额外收益不足以覆盖开发和分发成本。

## 后果

- 必须实现进程生命周期、IPC 请求关联、日志、超时和恢复。
- UI 不得接触 mpv 属性名或 JSON。
- 如果未来出现自定义渲染或底层音频回调需求，可新增 libmpv adapter，不改变应用契约。
