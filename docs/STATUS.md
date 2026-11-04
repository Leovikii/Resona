# 开发状态

最后更新：2026-11-04

## 当前阶段：0.3.0 正式版已发布

代码版本为 `0.3.0`。[PR #26](https://github.com/Leovikii/Resona/pull/26) 已通过最新双平台 CI 并合并至 main `7a47d35`。提示统一、快照/异步请求、转换/临时文件、列表协调、互斥、原子偏好及清理已完成，另补齐输出恢复、侧文件上限和 Pacman 诊断。已删除设置页快捷键说明，关于区在窄屏采用左标签、右控件，版本长文本可换行。

## 验证事实

- [最终 PR CI](https://github.com/Leovikii/Resona/actions/runs/41782550254) 成功：Node 79 通过、4 平台跳过；Windows Rust 156 通过、12 条件忽略；Arch Rust 151 通过、22 条件忽略，另有真实 FFmpeg 2 项及包元数据 1 项通过。Clippy、双平台构建、安装/升级/重装/预览版转正式版回归通过。
- 本机前端 build、TypeScript unused、Rust fmt、workflow、39 个音频 fixture 和中英文各 314 个翻译键检查通过；许可证重新生成无内容差异。缺少 MSVC，本机 Rust 编译由上述 CI 覆盖。
- Windows 预编译原生测试已在本机通过：双显示器/150%（含负坐标）、歌词 16/64px、锁定/解锁、helper 几何、隐藏重建与恢复；默认输出枚举/选择及真实输出静音 CUE 暂停/定位另外 2 项通过。单文件测试版交付后，所有者确认除两项设置调整外其他功能实测均通过，人工门禁解除。
- 设置修复后复核 1080×700、760×520、420×720、360×600、中英文及明暗主题，关于行无横向溢出、快捷键说明已删除。既有浏览器矩阵另覆盖系统/Mica 预览、focus/Escape、弹层互斥、压缩 720×500 及歌词普通/64px。
- 未逐项报告的超长路径/高对比度不推定独立验证通过。本轮 Linux 仅代码、可行测试和现有 PR CI；未新增独立 Linux CI，未运行 Linux 桌面/NFS/硬件专项。开放平台限制见 [KNOWN_ISSUES](KNOWN_ISSUES.md)。

## 交付状态

[0.3.0 Release](https://github.com/Leovikii/Resona/releases/tag/v0.3.0) 已公开，非草稿、非预览版，并设为 latest。main [CD](https://github.com/Leovikii/Resona/actions/runs/41783164607) 完整成功：Windows NSIS、Arch 包及干净安装校验通过，双平台身份、包签名和清单签名均已验证。公开后复核 10 个资源及 SHA256SUMS、stable 更新清单、tag 与构建 SHA 均一致（构建 b55.1，SHA `7a47d351066094e9b05b70a1000e1a714c470029`）。Windows 包尚无 Authenticode，updater 签名保留。代码实施计划已完成并移出活动计划，实施历史可从 PR/Git 查询。
