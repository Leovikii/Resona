# Arch Linux / Wayland 开发预览

`0.3.0-alpha.1` 开始支持 Arch Linux 系 x86_64 的原生 Wayland 会话。本机验证环境为 CachyOS、KDE Plasma、PipeWire；其他发行版、X11 和 ARM 不在范围内；交付状态见 [STATUS](STATUS.md)。

当前开发版本为 [0.3.0-beta.1](plans/0.3.0-beta.1.md)，仅推送 dev，Windows 验收前不合并发布；[beta.1 本机验证与限制](plans/0.3.0-beta.1-linux-validation.md)。当前公开安装版仍为 [0.3.0-alpha.4](plans/0.3.0-alpha.4.md)，所有者已确认本机测试通过并授权发布；双平台交付状态以 [main Actions](https://github.com/Leovikii/Resona/actions/workflows/main-merge-delivery.yml) 和下方 Release 资产为准。安装与更新规则见 [ADR 0032](decisions/0032-arch-in-app-update.md)。alpha.4 改用无边框标题栏与 KDE 模糊外壳，本机自动回归已完成，压缩窗口 squash 动画严格门禁仍失败，最终功能与标题栏视觉已由所有者确认，鼠标/菜单专项未逐项记录，性能未测；完整步骤、CI 历史与本机结果见 [alpha.4 Linux 实机交接](plans/0.3.0-alpha.4-linux-validation.md)。

## 安装、打开方式与更新

从 [alpha.4 Release](https://github.com/Leovikii/Resona/releases/tag/v0.3.0-alpha.4) 下载原生包与 SHA256SUMS：

```bash
sha256sum --ignore-missing -c SHA256SUMS
sudo pacman -U ./resona-0.3.0alpha4-1-x86_64.pkg.tar.zst
# 卸载，保留用户数据与音频
sudo pacman -R resona
```

- 不再分发 AUR 配方。预览与正式版包名统一为 `resona`，系统依赖由 Pacman 检查；不会执行 `pacman -Sy`、自动全系统升级或绕过依赖/签名检查。
- 关于页和启动静默检查使用现有 Release、SemVer 与签名清单；用户触发下载后 Tauri 验签，展示包变更，再由图形 Polkit 代理授权 Pacman 安装。只有系统官方包且当前进程版本匹配时可安装，开发版/解压运行需先手动安装。
- 预览到正式版沿用同一个 `resona` 包名，严格按更高 SemVer 升级。没有旧包名兼容或自动迁移：测试环境若装有 `resona-preview-bin` / `resona-bin`，先执行 `sudo pacman -R <实际旧包名>`，再安装新包；用户数据和音频保留。旧测试包的 test-profile 不自动合并。
- 本次按所有者授权删除旧 alpha.4 Release/tag 后同版本重发；已安装 alpha.4 的程序不会收到同版本自动升级，需手动重装。[ADR 0035](decisions/0035-unified-arch-package.md)。
- 系统 `LocalFileSigLevel` 继续生效；`.tauri.sig` 不能替代 OpenPGP。定制系统若强制本地包 OpenPGP 签名，需要按系统政策处理，应用不会关闭该要求。
- 预览最多保留 30 分钟，安装前重查包、内容与事务。安装期间不允许退出或新文件任务；有扫描/转换时先完成或明确取消。下载可以取消，Pacman 安装不可强杀。失败保留诊断，不删系统锁；成功复核数据库后提示重启。
- 包声明 `polkit`，桌面需运行图形授权代理；无代理、授权取消、锁或依赖冲突均在当前弹窗报告。本轮尚未执行真实图形授权与安装升级。
- 桌面入口、图标、MP3/WAV/FLAC 打开方式保持；不更改系统默认播放器。标准卸载钩子清理桌面缓存。

## 环境与运行

构建依赖（使用系统工具链时）：

```bash
sudo pacman -S --needed base-devel rust nodejs npm webkit2gtk-4.1 libayatana-appindicator alsa-lib openssl gtk-layer-shell polkit desktop-file-utils qt6-base kio
```

播放复用 Rodio / CPAL 的 ALSA 后端；PipeWire 桌面需安装 `pipewire-alsa`。压缩使用系统 `ffmpeg` 包。文件选择需有当前桌面适用的 portal 后端，例如 KDE 的 `xdg-desktop-portal-kde`。

```bash
sudo pacman -S --needed pipewire-alsa ffmpeg xdg-desktop-portal xdg-desktop-portal-kde kio-fuse kio-extras
npm ci
npm run dev:linux
```

生成嵌入前端的本机可执行文件并运行：

```bash
npm run build:linux
npm run start:linux
# 也可传入本地音频文件
GDK_BACKEND=wayland ./src-tauri/target/release/resona /path/to/track.flac
```

`build:linux` 使用 Tauri `--no-bundle`，启用 `custom-protocol`；无需 Vite 服务。它不生成 NSIS、AppImage、deb 或 updater 资产，也不安装到系统目录。可执行文件仍依赖本机 GTK / WebKitGTK 等共享库，不是跨发行版便携包。`dev:linux`、`start:linux` 明确选择 Wayland，不以 XWayland 启动成功代替原生验证。

数据遵循 Tauri 的 XDG 目录（应用标识 `io.github.vki.resona`）。测试使用临时 XDG 数据、配置与缓存目录，与用户数据分离。

### beta.1 文件选择与 NFS

Linux 通过标准 XDG FileChooser 调用当前桌面 backend，KDE 下共享网络位置；它是 KDE 原生选择对话框，并非 Dolphin 主窗口。应用直接使用已锁定的 ashpd API，以保留远端 URI、错误和 Wayland 父窗口；Windows 继续使用系统原生后端。缺失 portal 服务会明确报错，不自动切换到不共享网络位置的 GTK/zenity 界面。

本机 NFS 的 KIO 列举会把“.”报告成普通文件，KIO-FUSE 用它更新目录节点后会破坏原可读文件映射。beta.1 保留官方 D-Bus 文件映射，网络目录改用 KIO ListJob 列举并排除“.”和“..”；递归文件夹、同目录音频、自动 CUE、伴随歌词与外置封面共用此边界。普通本地/内核挂载目录继续使用标准库，不自动挂载或修改桌面配置。

KIO 工作模式编译在同一程序中，首次列目录时启动自身的独立进程，在 QtCore 事件循环中串行复用 KIO 会话与句柄，跳过 GTK、单实例与窗口初始化；原生任务 15 秒、父进程 20 秒兜底，目录条目最多 100000、响应最多 16 MiB。失败后回收工作进程，下次请求重建；父进程关闭输入后正常退出。支持 Qt 可读及编码 URI；主/压缩窗口共享任务锁，扫描/提交期间禁用重复添加并在超过 200 毫秒后显示状态。依赖系统 Qt6Core、KF6KIOCore/KF6CoreAddons；Arch 包声明 `qt6-base`、`kio>=6.23`、`kio-extras` 与 `kio-fuse`，没有单独的 sidecar 或运行时下载。

这些期限约束目录工作进程，不保证任意 FUSE stat/read 可立即中断。临时物理路径仍可能在桌面会话结束后失效，用户列表需重新选择；本次不增加永久网络源数据库模型。

### alpha.4 本机测试程序

本轮 beta.1 程序与同名压缩包保存在忽略目录 `src-tauri/target/test-build/Resona-0.3.0-beta.1-arch-wayland-x86_64/`；执行其中 `start-resona.sh` 即可。更新保留原 test-profile，压缩包不含测试数据。依赖、验证与人工清单见 [本机记录](plans/0.3.0-beta.1-linux-validation.md)。使用本机已安装的 gtk-layer-shell 和 Qt/KIO，不附带私有库；历史 alpha.1 包规则保留如下。

### alpha.1 历史本机测试包

alpha.1 曾提供 `src-tauri/target/test-build/Resona-0.3.0-alpha.1-arch-wayland-x86_64/`，在仓库根目录执行：

```bash
./src-tauri/target/test-build/Resona-0.3.0-alpha.1-arch-wayland-x86_64/start-resona.sh
```

无需 Node、Rust、Vite 或 sudo；启动器使用包内私有 gtk-layer-shell，其他 GTK / WebKitGTK / ALSA 库沿用本机环境。先退出其他 Resona 实例，保留整个目录结构，通过启动脚本运行；可传入音频路径。设置、列表和日志保存在包内 `test-profile/`，日志为 `test-profile/data/io.github.vki.resona/logs/resona.log`。同目录的 `.tar.gz` 可解压到可写目录使用；这只是本机测试产物，不是通用发行版便携包或正式安装器。

启动脚本模板为 `scripts/start-linux-test.sh`，组装测试包时复制为 `start-resona.sh`。只隔离 XDG 数据与缓存；保留桌面的 `XDG_CONFIG_HOME` 供 GTK/fontconfig 读取，并将原 XDG 数据目录的 `fonts/` 链接进测试数据目录。不能隔离整套桌面配置或屏蔽用户字体，否则 `system-ui` 仍会回退到其他字体。应用当前将偏好与列表保存在应用数据目录；旧测试包的 `test-profile/config/` 不再使用，用户数据仍保留。

包内 `bin/resona` 与 `lib/Resona/icons/` 遵循 Tauri Linux 资源布局；附带许可证、库源码与 SHA-256 清单。已从 `/tmp` 启动实际包，确认无需临时构建库路径即可加载附带库，完成歌词前端 ready/重建和 MPRIS 控制/退出回归。

## 普通窗口尺寸

与 Windows 使用相同的逻辑客户端尺寸；alpha.4 的 48px 自绘标题栏包含在客户端内，compositor 的边框和阴影不计入：

| 窗口模式 | 默认尺寸 | 最小尺寸 |
| --- | --- | --- |
| 宽屏主窗口 | 1080×700 | 760×520 |
| 窄屏主窗口 | 420×720 | 360×600 |
| 音频压缩 | 720×500 | 640×440 |

宽、窄两种模式各自记忆手动调整的尺寸；切换恢复对应尺寸，可用空间不足时约束到当前显示器。窗口几何记录升级到 `geometryVersion: 1`，统一保存逻辑像素；旧 Linux 记录因包含 GTK 装饰且可能混用模式，会一次恢复规范尺寸，保留布局模式、列表和偏好。

## 能力边界

| 能力 | 本轮实现 |
| --- | --- |
| MP3 / WAV / FLAC、CUE、输出设备、列表、封面、普通同步歌词 | 复用现有 Rust 服务与 React 界面 |
| WAV → FLAC | 检查 `/usr/bin/ffmpeg`、`/usr/bin/ffprobe` 的真实版本；缺失时提示 pacman 安装并提供“重新检查”，不执行提权或下载 Windows 二进制 |
| 文件安全 | 临时文件在目标目录内；校验后以硬链接原子发布新文件，再移除临时文件；已有目标不会被 POSIX rename 覆盖。文件系统不支持硬链接时明确失败并保留源文件 |
| 主窗口与压缩窗口 | 共享无边框标题栏，KDE 原生背景效果探测、外壳区域模糊与实色回退；协议与几何自动回归已执行，最终功能与标题栏视觉已由所有者确认；压缩最小化动画自动观察仍失败，鼠标/菜单专项未逐项记录。保存尺寸与布局，主窗口保存最大化状态；位置由 compositor 管理，不查询或恢复全局坐标 |
| 系统托盘 | 需要 StatusNotifier host；启动时经 Arch 自带的 `busctl` 有界检查一次。宿主或托盘创建失败不阻止播放，并拒绝隐藏主窗口；不持续轮询宿主 |
| 桌面悬浮歌词 | KDE Wayland layer-shell overlay 置顶透明窗口，字号固定高度、两行歌词、拖动/左右调整宽度、锁定穿透及原生 GTK 解锁按钮；关闭后释放 WebView |
| KDE 媒体面板 / MPRIS | 曲目信息、封面、位置、播放/暂停/停止、前后曲、定位、音量、循环/随机、激活与退出；复用同一播放引擎 |
| Windows 任务栏控件与 Mica | Linux 不提供 |
| 安装、文件关联与更新 | GitHub 原生包、桌面打开方式、Tauri 下载验签与 Polkit/Pacman 安装；实际交付与实机边界见 STATUS |

桌面歌词必须使用支持 layer-shell 的 Wayland compositor；当前验证 KDE Plasma。置顶由 overlay 协议保证，不抢键盘焦点；系统锁屏仍由 compositor 管理。几何为当前输出内的逻辑坐标，暂不支持拖动跨输出或热拔插恢复。

本机原生验证使用经 Arch 签名验证的 gtk-layer-shell 0.10.1，未修改系统；上述测试包自带私有库。直接运行裸 Release 时需安装系统包 `sudo pacman -S --needed gtk-layer-shell`。

托盘宿主在应用运行期间退出或重启后的恢复还需实机验收；目前启动检查不持续监控桌面会话。

## 验证

```bash
npm test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
# 本机有真实输出设备和系统 FFmpeg 时，串行运行通常忽略的用例
cargo test --manifest-path src-tauri/Cargo.toml --locked -- --ignored --skip native_ --skip packaged_update_metadata --skip nfs_ --skip unreachable_network_ --test-threads=1
# NFS/失联专项需桌面会话与专用环境，见 beta.1 Linux 验证文档；不纳入上述通用命令
# packaged_update_metadata 需先构建真实 Arch 包，并设置 RESONA_TEST_PACKAGE 指向包文件
# KDE Wayland 协议与生命周期测试，单独运行，使用隔离目录
resona_native_root=$(mktemp -d)
XDG_DATA_HOME=$resona_native_root/data XDG_CACHE_HOME=$resona_native_root/cache GDK_BACKEND=wayland WAYLAND_DEBUG=client \
cargo test --manifest-path src-tauri/Cargo.toml --locked --features tauri/custom-protocol \
  native_layer_surface_lifecycle -- --ignored --nocapture --test-threads=1
# 原生标题栏命中区、按钮动作和并发几何更新，单独运行
resona_controls_root=$(mktemp -d)
XDG_DATA_HOME=$resona_controls_root/data XDG_CACHE_HOME=$resona_controls_root/cache \
GDK_BACKEND=wayland WAYLAND_DEBUG=client \
cargo test --manifest-path src-tauri/Cargo.toml --locked --features tauri/custom-protocol \
  native_window_controls_and_geometry -- --ignored --nocapture --test-threads=1
```

alpha.4 窗口测试在主窗口/压缩窗口刚显示时确认无 GTK 装饰、客户端在一秒内映射，再通过 Tauri 原生窗口 API 检查最大化、最小化和 CloseRequested；并发 resize、实际 GTK 内容区尺寸、宽窄切换、自定义尺寸记忆、重新加载和压缩窗口重建继续覆盖。测试要求 `qdbus6` 及已启用的 maximize/squash 特效，通过 KWin activeEffects 观察动画；最小化还需检查 Wayland `xdg_toplevel.set_minimized()`，不依赖协议未提供的最小化状态确认。该用例不能证明 WebView 自绘按钮真实命中、KWin 模糊、窗口菜单或 portal 主题已通过；这些在本版交接中单独验收。

原生 Release 集成回归（先关闭所有 Resona 实例，脚本拒绝复用正在运行的应用）：

```bash
python3 scripts/test-linux-desktop.py
```

脚本生成静音 FLAC、封面和 LRC，使用临时 XDG 数据/缓存目录启动 Release，读取既有桌面字体配置，通过 StatusNotifier/D-Bus 验证实际歌词前端 ready 与销毁重建，以及 MPRIS 元数据、控制和退出；不修改用户音频或既有应用数据。

指向 main 的 PR CI 使用固定摘要的官方 Arch `base-devel` 容器，执行构建、测试、Clippy 和真实 FFmpeg 转换，并验证真实 Pacman 首次安装、旧包升级、重装、包身份、文件关联和卸载。`PR validation` 汇总 Windows/Arch 结果，成功且跟上 main 才能合并；dev push 不单独触发，已有 PR 的更新会触发。main 更新只进行双平台构建、包验证、签名和发布 CD。容器没有图形 Polkit 代理、物理音频设备或 Wayland compositor，不能据此声称桌面实机验收通过。

本轮实测和剩余门禁见 [0.3.0 计划](plans/0.3.0.md)，长期决定见 [ADR 0028](decisions/0028-arch-linux-wayland-baseline.md) 和 [ADR 0029](decisions/0029-wayland-lyrics-and-mpris.md)。
