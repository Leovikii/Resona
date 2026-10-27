# ADR 0028：Arch Linux / Wayland 初始兼容

- 状态：Accepted；桌面歌词与 MPRIS 的延期由 [ADR 0029](0029-wayland-lyrics-and-mpris.md) 修订
- 日期：2026-10-25
- 分发/更新与 CI/CD 由已接受的 [ADR 0030](0030-preview-builds-and-linux-distribution.md) 补充修订：Arch/AUR 安装分发、文件打开关联和包管理器更新引导；以下保留最初本机预览基线。
- 授权：所有者要求开始下一版本并兼容本机 Linux，当前只要求 Arch Linux 与 Wayland。
- 补充 ADR 0001、0005、0012、0022、0024 的平台范围；Windows 的既有约束继续有效。

## 决定

- 以 `0.3.0-alpha.1` 建立 Arch 系 x86_64 / Wayland 本机开发预览，继续使用 Tauri、Rodio / CPAL / Symphonia。Linux 音频经 ALSA 与 `pipewire-alsa` 接入 PipeWire，不增加播放引擎或隐式 DSP。
- Windows souvlaki、ZIP 下载依赖和 NSIS / 签名更新配置归属 Windows target / config。通用配置不指定安装包目标或 updater 资产，Linux 用 Tauri `--no-bundle` 构建带嵌入前端的可执行文件；正式 Linux 打包另行验收。
- FFmpeg 服务保持相同 DTO 和 command。Windows 保留固定版本、双 SHA-256 和按需下载；Linux adapter 只检查 `/usr/bin` 的系统包并报告真实版本，以 `managedBySystem` 区分安装方式。应用不运行 pacman、不提权、不下载 Windows 归档，也不修改系统二进制。
- Wayland adapter 保留窗口尺寸、布局和最大化恢复，交由 compositor 决定位置；不能因全局坐标 API 不受支持而令布局切换或尺寸保存失败。
- Linux 普通窗口使用实色。桌面歌词通过 layer-shell overlay 提供置顶、穿透与定位，KDE 媒体控制通过 MPRIS 接入，具体约束见 ADR 0029。
- 托盘是可降级能力。Arch 通过系统 `busctl`（2 秒超时、不自动启动 D-Bus 服务）检查 StatusNotifier host；无宿主或创建失败时保留主窗口并拒绝隐藏。宿主热重启恢复不作为本轮承诺。
- updater 的平台支持由 distribution adapter 决定并进入类型化快照。Linux 不发起 Release 发现，不提供 Windows 安装器或签名安装动作；UI 明确提示手动更新。
- Linux 输出提交不能使用会替换既有目标的 POSIX rename；对同目录临时文件用硬链接发布，再删除临时文件。目标出现竞争冲突或文件系统不支持硬链接时返回错误，保留源 WAV；禁止回退到覆盖写入。

## 验证与后果

- 前端与 Rust 回归、系统 FFmpeg 矩阵、默认音频设备、格式解码与连续播放在本机运行；显示与硬件结论单独记录。
- PR 的 `PR validation` 汇总 Windows 和 Arch 两项结果，任一失败都不能通过。Arch 容器与 Actions 固定摘要；不增加 Linux 自动发行链。
- 共享封面快照和播放接口保留 Windows 原生投影需要的字段/方法；Linux 中未消费的这些成员使用局部 lint 标注，不删除跨平台契约或给整个项目关闭警告。
- 不承诺其他发行版、X11、ARM、AppImage、AUR 发布或 Linux 自动更新。扩展时应追加具体能力、分发和验证方案。
