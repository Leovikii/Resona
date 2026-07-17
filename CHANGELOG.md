# Changelog

本项目的显著变更记录在此文件中。

## [0.0.1] - 2026-07-17

### Added

- Tauri 2、React 19、TypeScript、Vite 与 Mantine 最小应用壳。
- 基于 Rodio 0.22.2 的 WAV/FLAC 播放验证。
- 独立 Rust audio actor、类型化命令通道和 `PlaybackEngine` 边界。
- 文件选择、播放、暂停/继续、停止、自然结束与错误状态。
- GPL-3.0-only 许可证、架构决策和开发台账。

### Known limitations

- 仅用于技术可行性验证，不包含媒体库、队列、定位、音量、元数据或歌词。
- Windows 是唯一实现目标；本版本不实现或验证 Linux。

### Validation

- 自动化构建、Rust 测试、默认音频设备 smoke test 与界面检查通过。
- Windows 实机真实音频试听通过，播放、暂停、继续和停止行为正常。
