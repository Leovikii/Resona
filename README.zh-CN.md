<p align="center">
  <img src="assets/resona-resonance-wordmark.svg" width="420" alt="Resona">
</p>

<p align="center">专注聆听的本地优先桌面音频播放器。</p>

<p align="center">
  <a href="README.md">English</a> ·
  <a href="https://github.com/Leovikii/Resona/releases">下载</a> ·
  <a href="docs/README.md">项目文档</a>
</p>

## 特色

- 本地 MP3、WAV、FLAC 播放，支持播放列表和同步歌词
- Windows SMTC、任务栏控件、托盘控制和桌面歌词
- Arch / KDE Wayland 置顶桌面歌词与 MPRIS 媒体控制
- FFmpeg 无损 WAV 转 FLAC 压缩：Windows 按需下载校验，Linux 使用系统包
- SQLite 本地数据，无账号、无遥测、无云端媒体库

## 安装

### Windows x64

从 [GitHub Releases](https://github.com/Leovikii/Resona/releases) 下载 `.exe` NSIS 安装包；预览版标记为 **Pre-release**。安装包尚未使用 Authenticode，Windows 可能显示 SmartScreen 提示。请从官方发布页下载并核对 `SHA256SUMS`；应用内更新使用 Tauri 签名验证。

### Arch Linux x86_64 · Wayland

推荐使用 AUR 预编译包：预览版为 `resona-preview-bin`，未来正式版为 `resona-bin`。首次 AUR 同步尚未完成，上线后可任选一种方式安装：

```bash
yay -S resona-preview-bin
# 或
paru -S resona-preview-bin
```

当前预览包将在 main Actions 发布后提供。AUR 未上线时，可从 [GitHub Releases](https://github.com/Leovikii/Resona/releases) 下载 `.pkg.tar.zst` 原生包安装：

```bash
sudo pacman -U ./resona-preview-bin-0.3.0alpha1-1-x86_64.pkg.tar.zst
```

安装包会注册 MP3/WAV/FLAC 的桌面“打开方式”。AUR 更新使用 `yay -Syu` / `paru -Syu`；手动安装原生包的用户需下载新包后重新安装。Linux 应用内提供包管理器更新指引，压缩使用系统 FFmpeg。当前实测桌面为 KDE Plasma，其他发行版、X11 和 ARM 暂不支持，详见 [Linux 指南](docs/LINUX.md)。

## 开发

```bash
npm ci
npm run tauri dev
```

详见[开发准则](docs/DEVELOPMENT.md)。Resona 使用 [GPL-3.0-only](LICENSE) 许可证。

Arch Linux 的依赖与构建命令见 [Linux 指南](docs/LINUX.md)。
