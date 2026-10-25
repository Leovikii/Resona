# Arch Linux / Wayland 开发预览

`0.3.0-alpha.1` 开始支持 Arch Linux 系 x86_64 的原生 Wayland 会话。本机验证环境为 CachyOS、KDE Plasma、PipeWire；其他发行版、X11、ARM 和正式 Linux 安装包尚未验收。

## 环境与运行

构建依赖（使用系统工具链时）：

```bash
sudo pacman -S --needed base-devel rust nodejs npm webkit2gtk-4.1 libayatana-appindicator alsa-lib openssl gtk-layer-shell
```

播放复用 Rodio / CPAL 的 ALSA 后端；PipeWire 桌面需安装 `pipewire-alsa`。压缩使用系统 `ffmpeg` 包。文件选择需有当前桌面适用的 portal 后端，例如 KDE 的 `xdg-desktop-portal-kde`。

```bash
sudo pacman -S --needed pipewire-alsa ffmpeg
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

### 已构建的本机测试包

本轮提供 `src-tauri/target/test-build/Resona-0.3.0-alpha.1-arch-wayland-x86_64/`，在仓库根目录执行：

```bash
./src-tauri/target/test-build/Resona-0.3.0-alpha.1-arch-wayland-x86_64/start-resona.sh
```

无需 Node、Rust、Vite 或 sudo；启动器使用包内私有 gtk-layer-shell，其他 GTK / WebKitGTK / ALSA 库沿用本机环境。先退出其他 Resona 实例，保留整个目录结构，通过启动脚本运行；可传入音频路径。设置、列表和日志保存在包内 `test-profile/`，日志为 `test-profile/data/io.github.vki.resona/logs/resona.log`。同目录的 `.tar.gz` 可解压到可写目录使用；这只是本机测试产物，不是通用发行版便携包或正式安装器。

启动脚本模板为 `scripts/start-linux-test.sh`，组装测试包时复制为 `start-resona.sh`。只隔离 XDG 数据与缓存；保留桌面的 `XDG_CONFIG_HOME` 供 GTK/fontconfig 读取，并将原 XDG 数据目录的 `fonts/` 链接进测试数据目录。不能隔离整套桌面配置或屏蔽用户字体，否则 `system-ui` 仍会回退到其他字体。应用当前将偏好与列表保存在应用数据目录；旧测试包的 `test-profile/config/` 不再使用，用户数据仍保留。

包内 `bin/resona` 与 `lib/Resona/icons/` 遵循 Tauri Linux 资源布局；附带许可证、库源码与 SHA-256 清单。已从 `/tmp` 启动实际包，确认无需临时构建库路径即可加载附带库，完成歌词前端 ready/重建和 MPRIS 控制/退出回归。

## 普通窗口尺寸

与 Windows 使用相同的逻辑内容区尺寸，标题栏、边框和阴影不计入：

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
| 主窗口与压缩窗口 | 实色背景、GTK 原生装饰与完整按钮命中区；保存尺寸与布局，主窗口保存最大化状态；位置由 compositor 管理，不查询或恢复全局坐标 |
| 系统托盘 | 需要 StatusNotifier host；启动时经 Arch 自带的 `busctl` 有界检查一次。宿主或托盘创建失败不阻止播放，并拒绝隐藏主窗口；不持续轮询宿主 |
| 桌面悬浮歌词 | KDE Wayland layer-shell overlay 置顶透明窗口，字号固定高度、两行歌词、拖动/左右调整宽度、锁定穿透及原生 GTK 解锁按钮；关闭后释放 WebView |
| KDE 媒体面板 / MPRIS | 曲目信息、封面、位置、播放/暂停/停止、前后曲、定位、音量、循环/随机、激活与退出；复用同一播放引擎 |
| Windows 任务栏控件与 Mica | Linux 不提供 |
| 自动更新、Linux 包与文件关联安装 | 本轮不提供；设置显示手动更新，不查询或提供 Windows Release 更新 |

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
cargo test --manifest-path src-tauri/Cargo.toml --locked -- --ignored --skip native_ --test-threads=1
# KDE Wayland 协议与生命周期测试，单独运行，使用隔离目录
XDG_DATA_HOME=/tmp/resona-native-test/data XDG_CONFIG_HOME=/tmp/resona-native-test/config \
XDG_CACHE_HOME=/tmp/resona-native-test/cache GDK_BACKEND=wayland WAYLAND_DEBUG=client \
cargo test --manifest-path src-tauri/Cargo.toml --locked --features tauri/custom-protocol \
  native_layer_surface_lifecycle -- --ignored --nocapture --test-threads=1
# 原生标题栏命中区、按钮动作和并发几何更新，单独运行
XDG_DATA_HOME=/tmp/resona-controls-test/data XDG_CACHE_HOME=/tmp/resona-controls-test/cache \
GDK_BACKEND=wayland WAYLAND_DEBUG=client \
cargo test --manifest-path src-tauri/Cargo.toml --locked --features tauri/custom-protocol \
  native_window_controls_and_geometry -- --ignored --nocapture --test-threads=1
```

标题栏测试在主窗口/压缩窗口刚显示时检查事件层不覆盖按钮，一秒内具备完整输入区域，再检查按钮的 GDK 输入窗口与控件尺寸一致（至少 48×48 逻辑像素），通过原生 GTK 按钮信号验证最大化和关闭，并在布局切换期间触发 UI resize 回调。窗口回归还检查实际 GTK 内容区、24 次宽窄切换、两套自定义尺寸、重新加载和压缩窗口重建，避免只验证窗口 API 返回值。最小化以 Wayland 日志中的 `xdg_toplevel.set_minimized()` 为证，不能依赖 Wayland 没有提供的最小化状态确认；真实启动后立即点击、按钮边缘点击和标题栏拖动仍需人工验收。

原生 Release 集成回归（先关闭所有 Resona 实例，脚本拒绝复用正在运行的应用）：

```bash
python3 scripts/test-linux-desktop.py
```

脚本生成静音 FLAC、封面和 LRC，使用临时 XDG 数据/缓存目录启动 Release，读取既有桌面字体配置，通过 StatusNotifier/D-Bus 验证实际歌词前端 ready 与销毁重建，以及 MPRIS 元数据、控制和退出；不修改用户音频或既有应用数据。

CI 使用固定摘要的官方 Arch `base-devel` 容器，执行构建、测试、Clippy 和真实 FFmpeg 转换。CI 容器没有物理音频设备或 Wayland compositor，不能据此声称桌面实机验收通过。

本轮实测和剩余门禁见 [0.3.0 计划](plans/0.3.0.md)，长期决定见 [ADR 0028](decisions/0028-arch-linux-wayland-baseline.md) 和 [ADR 0029](decisions/0029-wayland-lyrics-and-mpris.md)。
