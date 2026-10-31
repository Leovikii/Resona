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

从 [rc.1 发布页](https://github.com/Leovikii/Resona/releases/tag/v0.3.0-rc.1) 下载原生包和同一版本的 `SHA256SUMS`，先核对包的哈希，再安装：

```bash
sha256sum --ignore-missing -c SHA256SUMS
sudo pacman -U ./resona-0.3.0rc1-1-x86_64.pkg.tar.zst
```

安装会注册 MP3/WAV/FLAC 的桌面“打开方式”。后续在应用内检查更新、下载验签并确认安装预览，经 Polkit 授权后由 Pacman 安装，成功后自动重启。卸载使用 `sudo pacman -R resona`，保留用户数据与音频。预览和正式版统一使用 `resona` 包名。测试环境如已安装旧包，先卸载再安装，不提供包名自动迁移。

应用遵守系统签名策略，`.tauri.sig` 是 Tauri 签名，不是 OpenPGP 签名；需要图形 Polkit 授权代理。压缩继续使用系统 FFmpeg。当前桌面范围为 KDE Plasma，其他发行版、X11 和 ARM 暂不支持，详见 [Linux 指南](docs/LINUX.md)。

## 开发

```bash
npm ci
npm run tauri dev
```

详见[开发准则](docs/DEVELOPMENT.md)。Resona 使用 [GPL-3.0-only](LICENSE) 许可证。

Arch Linux 的依赖与构建命令见 [Linux 指南](docs/LINUX.md)。
