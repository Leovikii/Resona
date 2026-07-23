# ADR 0005：Rodio 播放核心

- 状态：Accepted
- 日期：2026-07-17
- 替代：[ADR 0002](0002-mpv-sidecar.md)

## 背景

进一步收束需求后，首版播放范围为 Windows WASAPI Shared、板载声卡或蓝牙端点，以及 MP3、PCM WAV、FLAC。典型源文件从 44.1 kHz/16-bit 至 192 kHz/24-bit。AAC/M4A 很少使用，可以作为尽力支持项。

首版不需要 WASAPI Exclusive、ASIO、bit-perfect、DSD、视频或尽可能覆盖所有长尾格式。原 mpv sidecar 方案为这些非目标引入了额外二进制、JSON IPC、进程生命周期和分发复杂度。

Tauri 后端使用 Rust，Rodio 与其底层 CPAL、Symphonia 可以在同一类型、线程、错误和构建体系内工作。

## 决定

- 首版使用 `RodioPlaybackEngine` 实现 `PlaybackEngine`。
- CPAL 负责 Windows WASAPI Shared 输出和设备枚举。
- Symphonia 负责解码；强制支持 MP3、PCM WAV、FLAC。
- AAC/M4A 仅在启用对应 feature 后进入尽力支持范围。
- 使用稳定发布版本、Cargo.lock 和最小 feature 集；首个正式版本目标为 `playback`、`flac`、`mp3`、`wav`，关闭默认 features，不启用录音或无关格式。
- `mp4` 只有通过 P1 的 AAC/M4A 可行性验证后才进入发布配置。
- 播放引擎运行在专用 Rust actor 线程，通过 typed channel 接收命令并发布领域事件。
- 产品中不实现 mpv fallback 或第二套并行播放引擎。
- FFmpeg 仍只承担格式转换，不参与日常播放；其 0.0.16 随包 sidecar 分发方式已由 [ADR 0024](0024-windows-tray-distribution-and-update.md) 的按需下载依赖替代。

## 理由

- 与 Tauri/Rust 直接集成，消除播放 sidecar、命名管道和 JSON IPC。
- 当前格式和输出需求落在 Rodio/CPAL/Symphonia 的合理能力范围内。
- 安装体积、启动、类型安全和自定义播放行为更有利。
- Rodio 为 MIT/Apache-2.0，允许锁定、修补或 fork；Symphonia 与 CPAL 可独立演进。
- 播放 adapter 边界使未来替换不会影响 UI、应用服务、队列和媒体库模型。

## 持续性策略

Rodio 当前活跃，但 API 仍可能快速变化。停止更新不等于必须迁移：稳定版本可以继续工作。

只有出现无法在合理成本内修补的阻塞缺陷、安全问题或平台不兼容时，才按顺序评估：

1. 固定并修补当前版本。
2. 在项目维护可控范围内 fork Rodio。
3. 直接组合 CPAL + Symphonia。
4. 新建 GStreamer 或 mpv adapter。

替换必须新增 ADR。不得为了假想风险预装或测试两套生产播放引擎。

## 后果

- Resona 自己负责播放状态机、队列策略、设备恢复、进度事件和兼容性 fixture。
- 音频库类型和错误在 adapter 内转换为领域类型。
- 进程内音频错误不再由 sidecar 天然隔离，因此 actor 必须可监督、可诊断且禁止 `unwrap` 进入运行路径。
- 必须验证不同采样率、位深、Gapless、定位、坏文件、蓝牙断开和默认设备变化。
- 不承诺未进入发布测试矩阵的格式。

## 0.0.1 验证记录

- crates.io 的 Rodio 0.22.2 不提供仓库 master 曾展示的 `simd` feature；0.0.1 只使用稳定发布版实际公开的 `playback`、`flac`、`wav`。
- 未发布分支的 README、Cargo feature 或 API 不作为实现依据。

## 0.0.2 验证记录

- Rodio 0.22.2 使用 `flac`、`mp3`、`wav` 与 `playback` features；MP3/WAV/FLAC 16/24-bit 和 WAV 32-bit 已通过矩阵解码，MP3、FLAC 与 192 kHz/32-bit WAV 已通过默认设备 smoke test。
- 生成的合法 32-bit FLAC 可被 Xiph FLAC 1.5.0 与 FFmpeg 8.1.2 独立读取，但 Rodio 0.22.2 的 Symphonia FLAC adapter 无法产生样本。项目所有者确认该格式没有实际需求，因此明确不支持，不为其引入第二套播放引擎。导入时进入可恢复的 `decode` 失败状态，随后仍可播放支持的文件。
