# Resona

Resona 是一个从零构建的本地桌面高解析度音频播放器。当前开发目标是 Windows，架构为未来的现代 Linux（Wayland + PipeWire）实现保留边界，但首个版本不实现或验证 Linux。

## 技术基线

- 桌面框架：Tauri 2
- 前端：React、TypeScript、Vite、Mantine
- 播放核心：Rodio + CPAL + Symphonia
- 本地数据：SQLite
- 元数据：lofty-rs
- 格式转换：按需下载并校验的 FFmpeg 工具，不进入安装包

技术选择以最低总开发成本、成熟实现优先、离线可用和可维护性为准。详细约束见[开发准则](docs/DEVELOPMENT.md)。

## 本地开发

```powershell
npm ci
npm run tauri dev
```

普通开发和发行构建不下载 FFmpeg/ffprobe。应用只在用户从工具页明确操作后，从固定 GitHub Release 资源下载并校验依赖；真实转换矩阵测试可单独执行 `npm run prepare:test-tools`。版本、来源和校验值见 [测试工具说明](src-tauri/binaries/README.md)。

Windows 安装包通过 `npm run release:windows` 生成，文件名包含版本、平台和架构。

## 文档

- [文档索引](docs/README.md)
- [产品范围](docs/PRODUCT.md)
- [系统架构](docs/ARCHITECTURE.md)
- [计划目录结构](docs/STRUCTURE.md)
- [路线图](docs/ROADMAP.md)
- [开发状态台账](docs/STATUS.md)
- [架构决策记录](docs/decisions/README.md)

## 当前状态

`0.0.19` 已完成 Windows SMTC、任务栏媒体控件与进度、托盘和三态关闭行为、NSIS 当前用户安装器、文件关联、Local AppData 迁移及按需 FFmpeg 依赖，并通过项目所有者安装版验收。当前进入 `0.1.0-rc.1` 发布候选收尾，只处理更新器、签名/发布链、诊断与性能审计、遗留原生矩阵和发行阻塞缺陷。完整进度见 [开发状态台账](docs/STATUS.md) 与 [0.1.0-rc.1 计划](docs/plans/0.1.0-rc.1.md)。

## 许可证

Resona 使用 [GNU General Public License v3.0 only（GPL-3.0-only）](LICENSE)。
