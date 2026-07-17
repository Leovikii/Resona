# ADR 0004：项目采用 GPL-3.0-only

- 状态：Accepted
- 日期：2026-07-17

## 背景

Resona 将作为开源桌面应用开发，并计划分发音频依赖、FFmpeg 等第三方组件。仓库原本使用 MIT License，项目决定改用 GNU General Public License v3.0。

## 决定

Resona 自有代码使用 `GPL-3.0-only`。仓库根目录 `LICENSE` 保存 GNU 官方 GPL v3 完整正文。

第三方依赖、sidecar、字体、图像和测试素材保留各自许可证；发布时必须提供对应通知和源码获取义务所需材料。不得因为项目使用 GPL 而假设所有第三方组件自动兼容。

## 后果

- 新增源码文件应在适合的语言和文件类型中使用 `SPDX-License-Identifier: GPL-3.0-only`。
- 引入依赖前必须检查与 GPL-3.0-only 的兼容性。
- Rodio/CPAL/Symphonia 与 FFmpeg 的具体 feature、构建选项和分发方式需在发布前单独审计。
- 已经发布的第三方代码仍遵循其原许可证和通知要求。
