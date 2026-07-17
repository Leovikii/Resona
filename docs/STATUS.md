# 开发状态台账

最后更新：2026-07-17

## 当前阶段

0.0.1 最小技术验证：已完成。

0.0.2 播放能力闭环：计划已确认，等待 0.0.1 封板后开始开发。

## 已完成

- [x] 明确 Windows-first，Linux 只预留 Wayland + PipeWire 结构
- [x] 确定 Tauri 2 + React + TypeScript + Mantine
- [x] 收束播放需求为 WASAPI Shared、板载声卡/蓝牙和 MP3/WAV/FLAC
- [x] 确定 Rodio + CPAL + Symphonia 为首版播放核心
- [x] 以 ADR 0005 替代 mpv sidecar 决策，首版不实现备用引擎
- [x] 确定 SQLite、lofty-rs、FFmpeg sidecar
- [x] 定义 UI 依赖准入标准，排除运行时许可证注入机制
- [x] 建立产品、架构、目录、开发准则、路线图和 ADR 文档
- [x] 将仓库许可证切换为 GPL-3.0-only
- [x] 保存 Mantine 完整 LLM 文档并记录来源、大小和校验值
- [x] 初始化 Tauri 2 + React 19 + TypeScript + Vite + Mantine 应用
- [x] 锁定 Rodio 0.22.2 的最小 `playback`、`flac`、`wav` feature 集
- [x] 实现专用播放 actor、typed channel、`PlaybackEngine` 和 Tauri commands
- [x] 实现 WAV/FLAC 文件选择、播放、暂停/继续、停止及自然结束状态
- [x] 通过 TypeScript/Vite 构建、Rust 测试、Clippy、默认音频设备和 release 构建验证
- [x] 在 680×440 与 360×640 视口验证暗色最小界面，无溢出或控制台告警
- [x] Windows 实机真实音频试听通过，播放、暂停、继续和停止功能正常
- [x] 完成 0.0.1 验收记录并梳理 0.0.2 提案

## 进行中

- [ ] 封板提交当前工作区并创建 `v0.0.1` Git 标签
- [ ] 按已接受的 [0.0.2 播放能力闭环计划](plans/0.0.2.md)推进

## 下一步

1. 经确认后封板提交 0.0.1 并创建 `v0.0.1` 标签。
2. 先建立 Windows CI 与当前依赖许可证清单，固定现有质量基线。
3. 开发 0.0.2：MP3、固定 fixture、进度/时长、定位、音量与结构化错误。
4. 0.0.3 再处理队列、混合采样率连续播放与 Gapless。
5. 0.0.4 再处理设备枚举、默认设备变化、蓝牙断开与恢复。
6. 只在实际功能需要时引入 Zustand、TanStack Virtual 或 Mantine 专项 skill。

## 阻塞项

当前无技术阻塞项。下一阶段等待范围确认，尚未开始实现。

## 尚未验证的关键风险

| 风险 | 验证阶段 | 状态 |
| --- | --- | --- |
| Rodio/CPAL/Symphonia 版本组合与 API 稳定性 | 0.0.1 / P1 | 最小播放链与实机输出已验证，完整矩阵未验证 |
| 24/192、混合采样率与 Gapless 正确性 | P1 | 未验证 |
| WASAPI Shared 设备切换与蓝牙断开恢复 | P1 | 未验证 |
| Rodio 上游停更后的固定、fork 或替换路径 | 持续检查 | 已设计，未触发 |
| 透明歌词窗口的输入穿透和 DPI | P4 | 未验证 |
| GPL-3.0-only 与 Rodio/CPAL/Symphonia/FFmpeg 依赖组合 | P6 前持续检查 | 未完成 |

## 台账维护规则

- 每次有效开发后更新“已完成、进行中、下一步、风险”。
- 只记录已经验证的事实；计划不得写成完成。
- 测试失败或外部依赖阻塞必须明确记录，不能省略。
- 完成一个阶段后，把验收证据或命令摘要记录在本文件或对应阶段文档。
