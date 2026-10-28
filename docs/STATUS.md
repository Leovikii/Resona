# 开发状态

最后更新：2026-10-28

## 当前版本与交付

- 版本为 `0.3.0-alpha.3`，所有者已批准实施并授权 dev → main PR、合并交付及分支同步。[开发方案](plans/0.3.0-alpha.3.md)、[ADR 0032](decisions/0032-arch-in-app-update.md)。
- 旧 alpha.1 / alpha.2 两条 Release 及资产已按授权删除，其他 Releases、Git tags 与历史提交保留。旧用户首次手动安装 alpha.3，用户列表、偏好和音频保留。
- Linux 收敛到 GitHub 原生包安装与应用内 Tauri 下载验签、事务预览、Polkit/Pacman 安装；删除 AUR 发布与指引。仍只支持 Arch 系 x86_64 / KDE Wayland。Windows 与现有 UI 设计沿用。
- 已实现双平台签名清单、受限临时包、身份/摘要/事务复核、操作单飞、转换/扫描互斥、安装期间退出保护与安装后数据库确认。只支持同包名升级，预览/稳定包切换需手动安装。
- 显式设置 updater 检查 15 秒、下载 600 秒超时；锁定插件不会自动将检查超时传给下载。Pacman 安装不受该超时强杀。
- [PR #18](https://github.com/Leovikii/Resona/pull/18) 已通过[双平台门禁](https://github.com/Leovikii/Resona/actions/runs/40629669622)并合并，初始实现提交为 `bee44e5`。超时修正和验证记录由 [PR #19](https://github.com/Leovikii/Resona/pull/19) 跟踪。Actions 使用内部 PKGBUILD 与 Arch PR 包安装测试；签名仅在 main 的 release Environment，Windows/Arch 同 SHA 完成后统一草稿验签公开。[main 交付流水线](https://github.com/Leovikii/Resona/actions/workflows/main-merge-delivery.yml)，公开包入口为 [alpha.3 Release](https://github.com/Leovikii/Resona/releases/tag/v0.3.0-alpha.3)。

## 本轮已验证

- Windows 本机 Rust：133 项通过、10 项按硬件/工具条件忽略；包括新的更新互斥、文件任务安装互斥、Arch 事务解析、错平台/版本包与签名清单校验。Clippy、fmt 通过。
- PR 的 Windows 与 Arch 编译、Rust、Clippy、前端及发布契约检查均通过。Arch 常规 Rust 123 项通过、12 项条件忽略；另外 2 项真实 FFmpeg 和 1 项 Pacman 元数据测试通过，真实包首装、alpha.2→alpha.3 升级、重装、桌面注册和卸载通过。
- Node：58 项通过、4 项平台专属跳过。TypeScript、生产前端构建、许可证清单生成通过。actionlint 1.7.12 通过。
- Windows Tauri debug `--no-bundle` 嵌入前端程序构建通过；原生窗口自动化因应用授权超时未完成，不能据此声称原生视觉通过。
- 浏览器更新预览覆盖 1080×700、760×520、420×720、360×600，中英/明暗代表组合、20 项长依赖列表、完成提示、等待阶段和 Escape/重复安装保护；控件可达。这只证明共享 UI，不证明系统授权与安装。
- 既有 alpha.2 KDE 主题、应用身份、歌词和窗口实机结论保留在历史计划；本轮未修改这些 adapter，也未在此 Windows 主机重跑 KDE 会话。

## 开放门禁与限制

- 此主机没有可用 Linux/WSL；Linux 结果来自 Arch CI。没有 KDE/Polkit 会话的容器不能代替真实授权、应用内跨版本升级和重启验证。
- Linux 系统 `LocalFileSigLevel` 不变；Tauri `.tauri.sig` 不是 OpenPGP。无图形授权代理、包锁、冲突或严格本地签名政策时明确失败，程序不删锁、不关闭检查、不自动全系统升级。
- Windows NSIS 真实跨版本覆盖、取消、卸载及完整 SMTC/托盘/Mica/文件关联仍需安装版实机回归；本机测试和 CI 构建不能替代。安装包尚无 Authenticode，Tauri 包与清单签名保留。
- 多屏/DPI、物理媒体键、睡眠/蓝牙恢复等既有人工门禁保持；歌词跨输出拖动/热拔插恢复、托盘宿主热重启恢复未实现或未验收。Linux 转换目标文件系统需支持同卷硬链接；不支持则保留源文件并失败。
