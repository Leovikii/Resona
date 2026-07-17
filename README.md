# Resona

Resona 是一个从零构建的本地桌面高解析度音频播放器。当前开发目标是 Windows，架构为未来的现代 Linux（Wayland + PipeWire）实现保留边界，但首个版本不实现或验证 Linux。

## 技术基线

- 桌面框架：Tauri 2
- 前端：React、TypeScript、Vite、Mantine
- 状态与长列表：Zustand、TanStack Virtual
- 播放核心：Rodio + CPAL + Symphonia
- 本地数据：SQLite
- 元数据：lofty-rs
- 格式转换：FFmpeg sidecar

技术选择以最低总开发成本、成熟实现优先、离线可用和可维护性为准。详细约束见[开发准则](docs/DEVELOPMENT.md)。

## 文档

- [文档索引](docs/README.md)
- [产品范围](docs/PRODUCT.md)
- [系统架构](docs/ARCHITECTURE.md)
- [计划目录结构](docs/STRUCTURE.md)
- [路线图](docs/ROADMAP.md)
- [开发状态台账](docs/STATUS.md)
- [架构决策记录](docs/decisions/README.md)

## 当前状态

`0.0.1` 最小技术验证已完成自动化检查与 Windows 实机试听：Tauri 窗口可以选择 WAV/FLAC，Rust 后端通过 Rodio 在独立线程中播放、暂停、继续和停止。

开发环境、验证命令和后续工作以 [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md)、[docs/STATUS.md](docs/STATUS.md) 与 [0.0.2 提案](docs/plans/0.0.2.md)为准。

## 许可证

Resona 使用 [GNU General Public License v3.0 only（GPL-3.0-only）](LICENSE)。
