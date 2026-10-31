# Arch Linux / KDE Wayland

当前版本：[0.3.0-rc.1](plans/0.3.0-rc.1.md)。本机渲染修复与新增阴影已获所有者验收；测试与交付事实见 [STATUS](STATUS.md)，平台限制见 [KNOWN_ISSUES](KNOWN_ISSUES.md)。支持 Arch 系 x86_64 / KDE Wayland，不以 XWayland 或浏览器预览代替原生验证。

## 安装与更新

从 [rc.1 Release](https://github.com/Leovikii/Resona/releases/tag/v0.3.0-rc.1) 下载原生包及同版本 `SHA256SUMS`，校验后安装：

```bash
sha256sum --ignore-missing -c SHA256SUMS
sudo pacman -U ./resona-0.3.0rc1-1-x86_64.pkg.tar.zst
# 卸载保留用户数据与音频
sudo pacman -R resona
```

预览与正式版共用 `resona` 包名，仅通过 GitHub 分发，不使用 AUR。不兼容旧 `resona-preview-bin` / `resona-bin` 包名；旧测试安装需先卸载再安装，数据不删除。桌面入口注册 MP3/WAV/FLAC 打开方式，不修改默认播放器。

应用内检查使用 Release、SemVer 与签名清单，只接受更高版本。用户触发下载后 Tauri 验签、展示包事务，经图形 Polkit 代理授权 Pacman；安装前复核包与事务，成功后核对数据库、保存状态并自动重启。仅支持已安装的系统官方包且进程版本匹配，解压测试程序不能直接安装更新。预览最多保留 30 分钟；安装与扫描/转换互斥，安装不可强杀。失败保留诊断，不删除系统锁。

Pacman 保留系统签名策略，`.tauri.sig` 不替代 OpenPGP；应用不会执行 `pacman -Sy`、关闭验签或自动全系统升级。详见 ADR 0032/0035。

## 环境与构建

```bash
sudo pacman -S --needed base-devel rust nodejs npm webkit2gtk-4.1 libayatana-appindicator alsa-lib openssl gtk-layer-shell polkit desktop-file-utils qt6-base kio
sudo pacman -S --needed pipewire-alsa ffmpeg xdg-desktop-portal xdg-desktop-portal-kde kio-fuse kio-extras
npm ci
npm run dev:linux
# 构建嵌入前端的本机程序，无需 Vite 服务
npm run build:linux
npm run start:linux
# 可传入音频
GDK_BACKEND=wayland ./src-tauri/target/release/resona /path/to/track.flac
```

播放使用 Rodio/CPAL ALSA，PipeWire 桌面需有 `pipewire-alsa`。压缩检查系统 `/usr/bin/ffmpeg` 与 `ffprobe`，不下载 Windows 工具。`build:linux` 使用 Tauri `--no-bundle` 与 `custom-protocol`，不生成通用便携包；程序仍依赖本机 GTK/WebKitGTK 等共享库。CI 负责生成 Arch 发布包。

数据遵循 Tauri XDG 目录，应用标识 `io.github.vki.resona`。测试仅隔离数据/缓存，保留 `XDG_CONFIG_HOME` 和桌面字体；独立数据目录需要链接原用户数据目录的 `fonts/`。由 agent 启动时先清除继承的 LANG/LANGUAGE/LC_*，再完整复制桌面会话的语言变量，不强制中文或改应用偏好。

最新本机测试包位于忽略目录 `src-tauri/target/test-build/Resona-0.3.0-rc.1-shadow-arch-wayland-x86_64/`，执行 `start-resona.sh`；需保留整个目录，并先退出其他实例。其独立 Tauri 标识与 `test-profile/` 隔离安装版数据，但 GTK/MPRIS 沿用既有标识。旧测试配置迁存 `saved-profiles/`，归档不含用户测试数据。

## 窗口与材质

| 模式 | 默认逻辑内容尺寸 | 最小尺寸 |
| --- | --- | --- |
| 宽屏主窗口 | 1080×700 | 760×520 |
| 窄屏主窗口 | 420×720 | 360×600 |
| 音频压缩 | 720×500 | 640×440 |

共享 48px 自绘标题栏计入内容尺寸；宽/窄各自记忆正常尺寸，主窗口保存最大化状态，全局位置交给 compositor。`geometryVersion: 1` 统一逻辑像素；旧装饰几何一次恢复规范尺寸，保留布局/列表/偏好。

KDE 原生背景效果只模糊正文之外的外壳，缺失 capability 时使用实色。rc.1 将 blur 四周内收 1 个 surface-local 逻辑像素，规避已验收环境的拖动残线；固定 24px KWin 原生柔和阴影使用八块共享内存贴片，最大化/全屏时移除，恢复复用缓冲区。阴影不扩大窗口或输入区域，也不替代 blur 内收；缺失协议保留无阴影窗口。生命周期与协议边界见 [ADR 0033](decisions/0033-frameless-titlebar-and-kde-material.md)。

## 桌面能力与文件访问

- MP3/WAV/FLAC、CUE、播放列表、封面与普通歌词复用共享领域服务；WAV→FLAC 校验后原子发布，不覆盖已有输出，文件系统不支持硬链接时明确失败并保留源文件。
- 托盘需要 StatusNotifier host，启动时通过 `busctl` 有界检查。缺失宿主或创建失败不影响播放，并拒绝隐藏主窗口。
- 桌面歌词使用 layer-shell overlay、透明窗口、空输入区域穿透及原生 GTK 解锁按钮；按输出内逻辑坐标保存显示器，跨屏同时重绑定 helper。换屏会结束拖动，热拔插恢复暂不支持。
- MPRIS 提供元数据、封面、时间轴、播放控制、定位、音量、循环/随机、激活与退出，复用同一播放引擎。Linux 不提供 Windows 任务栏能力与 Mica。
- 文件选择直接使用标准 XDG FileChooser portal，保留远端 URI、错误和 Wayland parent；KDE backend 是原生选择器，不是 Dolphin 主窗口。缺失服务明确报错，不切换到第二套 chooser。
- NFS 文件使用 KIO-FUSE 官方映射，目录使用 KIO ListJob 并排除本机错误的“.”元数据。递归导入、CUE、歌词、封面和压缩扫描共用边界；普通本地/内核挂载目录使用标准库，不自动挂载或改桌面配置。
- KIO 工作模式编译在同一程序，re-exec 后先于 GTK/单实例初始化 QtCore；串行复用会话，每请求 15 秒期限、父进程 20 秒兜底，条目上限 100000、响应 16 MiB，失败回收。没有 sidecar、全局目录缓存或运行时下载。临时映射和 FUSE 读取限制见已知问题。

## 验证

```bash
npm run test:linux
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked --features tauri/custom-protocol -- -D warnings
# 系统 FFmpeg 的真实转换矩阵
cargo test --manifest-path src-tauri/Cargo.toml --locked ffmpeg -- --ignored --test-threads=1
# 嵌入 Release 桌面集成；先退出其他实例
python3 scripts/test-linux-desktop.py
```

KDE 原生会话测试单独、串行执行，先构建前端并设置隔离目录；以下命令将过滤器分别替换为 `native_layer_surface_lifecycle`、`native_shadow_lifecycle` 或 `native_window_controls_and_geometry`：

```bash
resona_native_root=$(mktemp -d)
XDG_DATA_HOME=$resona_native_root/data XDG_CACHE_HOME=$resona_native_root/cache \
GDK_BACKEND=wayland WAYLAND_DEBUG=client \
cargo test --manifest-path src-tauri/Cargo.toml --locked --features tauri/custom-protocol \
  native_shadow_lifecycle -- --ignored --nocapture --test-threads=1
```

Linux 自动化由 Linux 平台 agent 和测试脚本执行，不用鼠标模拟；确需视觉或物理操作的项目交所有者验收。协议/状态断言不证明鼠标命中、动画、阴影观感或物理听感。窗口几何用例的已知失败见 [KNOWN_ISSUES](KNOWN_ISSUES.md)。桌面集成脚本使用临时静音音频、封面和歌词验证单实例、歌词 ready/重建及 MPRIS，不改用户数据。

PR CI 使用锁定摘要的 Arch 容器，验证 Linux 测试、Clippy、构建、FFmpeg、真实包安装/升级/重装/卸载及桌面注册；共享前端测试由 Windows job 执行。`PR validation` 要求两端成功；main 只执行双平台 CD。容器没有图形 Polkit、物理音频设备或 Wayland，不能据此宣称桌面实机通过。完整分工见 [开发规范](DEVELOPMENT.md#双平台精简执行)。
