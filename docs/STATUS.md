# 开发状态

最后更新：2026-10-27

## 当前版本与阶段

- 仓库版本为 `0.3.0-alpha.1`，开始提供 Arch 系 Linux x86_64 / Wayland 本机开发预览；[计划与验证](plans/0.3.0.md)、[运行指南](LINUX.md)、[ADR 0028](decisions/0028-arch-linux-wayland-baseline.md) 与 [ADR 0029](decisions/0029-wayland-lyrics-and-mpris.md)。尚未正式发布。
- `0.2.0` 是已正式发布的版本，见[发行说明](releases/0.2.0.md)。`0.2.1` 的所有者实测和原有 Windows 交付记录仍保留于[计划](plans/0.2.1.md)，本轮没有合并或正式发行该版本。
- [跨平台预览交付方案](plans/0.3.0-preview-delivery.md)与 [ADR 0030](decisions/0030-preview-builds-and-linux-distribution.md) 已接受并实现：保持 `0.3.0-alpha.1`，Windows NSIS/Tauri 签名更新，Arch 原生包与 AUR `-bin` 配方，以及 Linux 包管理器更新说明。main Actions 统一发布双平台资产；本机产物只测试，不提交/上传。AUR 同步默认关闭，需所有者配置账号、部署密钥与主机公钥后启用。

## 已验证能力

- 本机 CachyOS / Wayland 通过 Tauri 开发与 Release 构建；Release 嵌入前端，在停止 Vite 后仍完成初始化与前端 IPC，使用隔离 XDG 数据目录。
- 播放继续复用 Rodio / CPAL / Symphonia；Linux 使用 ALSA / pipewire-alsa。54 项前端测试、112 项 Rust 常规测试通过；单独串行运行的 9 项真实设备/工具用例全部通过，覆盖默认输出、MP3/FLAC/高采样率 WAV、错误恢复、连续播放、系统 FFmpeg 与转换矩阵。
- Linux 检查系统 `/usr/bin/ffmpeg` / `ffprobe` 并报告真实版本；缺失时提示 pacman 安装和重新检查。Windows 保留固定归档、SHA-256 与按需下载；新增 GTK layer-shell / MPRIS 依赖，未升级既有依赖大版本。
- Wayland 交由 compositor 管理全局位置，保留尺寸和布局恢复。普通窗口使用实色；桌面歌词已用 layer-shell overlay 置顶，提供逻辑几何、字号高度、锁定穿透与无 WebView 的 GTK 解锁按钮。Linux 更新界面显示手动更新，不提供 Windows 更新资产。
- Linux 输出使用不覆盖的原子硬链接提交；冲突测试、既有提交/删除顺序、取消和批次安全回归通过。托盘创建失败不阻止启动，无托盘时拒绝隐藏；正常 KDE 会话和无宿主的临时 D-Bus 会话均完成原生启动验证。
- 浏览器检查覆盖宽屏 `1080×700` / `760×520`、窄屏 `420×720` / `360×600`、压缩窗口 `720×500`，复核明暗主题、中英文、系统依赖就绪/缺失和手动更新；不据此声称原生多屏/DPI通过。
- `scripts/test-linux-desktop.py` 在最终 Release 上通过：实际歌词前端 ready、显示、关闭和重建，以及独立进程退出清理；修正隐藏 WebKitGTK 的初始读取与动画帧等待。KDE 会话的 D-Bus 实测通过：元数据/封面、播放/暂停/停止、前后曲、带曲目校验的定位、音量、循环/随机、Raise 和 Quit。原生忽略测试通过并由 Wayland 协议日志确认 overlay=3、无键盘交互、空输入区域、原生解锁回调、130/199px 字号高度与销毁重建。
- format、Clippy、生产前端、Release、许可证清单、工作流语法与差异检查通过。新增固定摘要的 Arch CI，与 Windows 共同汇总到既有 `PR validation`；PR #13 的首次 Arch 验证已在 GitHub 通过，实际发布仍待 main 交付。
- 已生成本机测试包 `src-tauri/target/test-build/Resona-0.3.0-alpha.1-arch-wayland-x86_64/`，通过 `start-resona.sh` 启动；包含私有 gtk-layer-shell、运行资源、许可证和库源码，测试数据隔离在包内。脱离构建目录和 Vite 的原生启动、私有库加载、歌词显示/重建与 MPRIS 控制回归通过，见 [Linux 指南](LINUX.md)。
- 所有者确认本机测试版功能正常。按反馈将正文、标题与桌面歌词统一改为系统默认字体（`system-ui`），等宽内容使用系统等宽回退；不内置或下载字体。TypeScript、生产前端与 Linux Release 构建通过，浏览器复核明暗主题和中英文，测试包已更新并保留已有测试数据；Windows 字体效果待原生回归。
- 修正测试启动器隔离 XDG 配置/数据导致系统字体配置与用户字体不可见的问题：保留桌面配置，并链接用户字体，只隔离应用数据/缓存。启动器回归和原生 WebKit 渲染对比通过，GTK 字体由 Adwaita 回到 Sarasa UI SC；测试包已更新。所有者反馈的模糊尚需重新观察；本机 150% 缩放下 GTK 3 使用 2 倍绘制，不能据此宣称分数缩放清晰度已验收。
- 修正主窗口几何查询持有状态锁时等待 UI 线程的锁顺序风险；Tao 的 Wayland 标题栏 EventBox 调整至原生按钮下方，避免覆盖按钮输入。所有者确认启动后的点击等待问题已修复。原生回归在首帧检查事件层与完整命中区域，验证主窗口/压缩窗口三种按钮动作、Wayland 最小化请求及 24 次并发布局切换。
- 按反馈将两类普通窗口的标题栏按钮从约 42×42 放大到 48×48 逻辑像素，Breeze 符号背景从 18px 放大到 28px，其他主题的 GtkImage 保留独立图标缩放。原生 GTK 绘制复核覆盖明暗 Breeze、五种常用窗口尺寸与本机 GTK 2 倍绘制，按钮布局与输入窗口尺寸一致；启动期原生回归、108 项 Rust 常规测试、Clippy 与 Release 构建通过。测试包同步更新并保留数据；放大后的真实鼠标手感、标题栏拖动与 Windows 仍待实机复核。

- 修正宽窄模式尺寸串写和 GTK 装饰尺寸累加：普通窗口以 `geometryVersion: 1` 保存逻辑内容区宽高，宽/窄模式各自恢复尺寸，切换期间过滤旧查询与中间 resize。正常窗口不重复发送取消最大化请求；GTK 内容控件直接约束最小尺寸，避免最大化时变化的标题栏/阴影干扰。默认尺寸与 Windows 规范一致，按当前工作区收敛；旧 Linux 几何一次恢复默认，保留布局模式、列表和偏好。原生回归连续三次通过，覆盖 24 次切换、首次启动和两种模式的最小尺寸、自定义尺寸及重新加载、最大化跨模式恢复、压缩窗口三次重建；54 项前端测试、112 项 Rust 常规测试及 Clippy 通过。最新 Release 的桌面歌词/MPRIS 回归通过；测试包和归档已同步，校验清单通过并保留既有测试数据。独立临时包从 `/tmp` 连续两次启动，验证旧 Linux 几何迁移为窄屏 `420×720`、重启尺寸不累加，以及私有库加载和歌词重建。所有者已确认本机宽窄切换与窗口尺寸问题修复。Windows 原生与多屏不同缩放仍待实机复核。

- 预览交付实现的本机检查：61 项前端/发布规则测试通过（含真实 `vercmp`/`makepkg` 配方解析、音频 MIME 检测和隔离桩的 AUR 重试/防降级/篡改拒绝）；117 项 Rust 常规测试通过，另 11 项原生/设备用例默认忽略。签名 fixture 由 Tauri CLI 生成，Rust 验证有效清单及篡改拒绝，系统 Minisign 同样验证通过；只声明已锁定的 MIT `minisign-verify 0.2.5` 为直接依赖，依赖版本与许可证清单不变。
- Linux Release 与实际 Arch 配方打包通过，生成约 8.7 MiB `.pkg.tar.zst`；载荷 SHA-256、包清单/root 权限、桌面入口和 MP3/WAV/FLAC 缓存注册通过。安装布局下的原生回归通过：`gio launch` 冷启动、三种音频交给已有实例、中文/空格路径、歌词 overlay 重建及完整 MPRIS 控制；日志确认 Linux 启动无更新检查。未在本机安装到系统目录或上传这些测试包。
- 更新说明浏览器检查覆盖中文浅色/英文深色、宽屏 `1080×700`、窄屏 `420×720` / `360×600`，命令复制和内容可达、无横向溢出。生产前端、format、Clippy、许可证、actionlint 与差异检查通过；Windows 覆盖安装、GitHub 实际发布和 AUR 首次同步仍需下述门禁。
- PR #13 首次 Windows CI 暴露两处问题：Git 的 CRLF 检出改变签名 fixture 字节，以及 `std::fs::rename` 会覆盖竞争创建的目标。签名 JSON 已固定 LF，并验证 CRLF 内容仍被拒绝；Windows 输出改用不设置替换/跨卷复制标志的 `MoveFileExW`，保留不覆盖约束。回归覆盖中文/空格路径、冲突保留、成功提交及 NUL 路径拒绝；本机 118 项 Rust 测试与 Clippy 通过，模拟 Windows 检出保持签名字节不变。Windows 后续 CI 结果以该 PR 检查为准。
- Windows Actions 已验证上述测试修复；后续 Clippy 暴露 SMTC 两处已填写全部字段后的冗余 `MediaMetadata::default()`，已删除。无封面回退中仍保留所需默认值，媒体信息和封面失败处理不变。

## 当前开放门禁

- 所有者需按 [0.3.0 计划](plans/0.3.0.md) 验收原生文件选择、拖入、托盘关闭/退出、蓝牙/睡眠恢复、物理设备与多屏缩放；Windows Release 仍需回归。
- Linux 安装包/文件关联与双平台发布代码已完成；公开安装包仍待所有者将 dev PR 合并到 main 后由 Actions 生成。AUR 首次同步需要配置 `RESONA_AUR_ENABLED` 与 `aur` Environment；实际包名可用性/维护权限待所有者确认。其他发行版、X11 和 ARM 不在本轮范围。
- 正式发行继续走受保护的 main 工作流；仍需真实运行 draft 汇总发布、Windows 同版本 NSIS 覆盖/取消/签名错误与卸载，以及 AUR 首次同步。CI 已加入干净 Arch 容器安装/校验/卸载步骤，尚未在 GitHub 执行。本轮未创建标签、发布、合并 PR 或覆盖安装用户应用。

## 风险与限制

- 本机未安装 gtk-layer-shell；测试包启动器加载附带的经 Arch 签名校验的 0.10.1 库，无需 sudo。裸 Release 可执行文件仍需系统安装 `gtk-layer-shell` 或显式配置库路径。
- 歌词拖动/调整宽度的真实鼠标手感、物理媒体键、多屏/DPI、全屏视觉遮挡仍需所有者观察；当前仅支持在选定输出内移动，不支持拖动跨输出或输出热拔插恢复。
- StatusNotifier host 只在启动检查一次；宿主运行中退出/重启后的托盘恢复仍待验收。
- Linux 目标文件系统必须支持同卷硬链接；不支持时转换明确失败并保留源文件，不回退覆盖写入。
- Tauri 既有 libayatana-appindicator 依赖在本机输出弃用提示，当前不阻止启动。
- Windows topmost、原生文件选择、多屏/DPI 与签名边界见[已知问题](KNOWN_ISSUES.md)。
- 32-bit integer/float WAV 转为 24-bit FLAC；不支持播放 32-bit FLAC。
