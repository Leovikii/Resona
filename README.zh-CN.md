<p align="center">
  <img src="assets/resona-resonance-wordmark.svg" width="420" alt="Resona">
</p>

<p align="center">专注聆听的本地优先 Windows 音频播放器。</p>

<p align="center">
  <a href="README.md">English</a> ·
  <a href="https://github.com/Leovikii/Resona/releases">下载</a> ·
  <a href="docs/README.md">项目文档</a>
</p>

## 特色

- 本地 MP3、WAV、FLAC 播放，支持播放列表和同步歌词
- Windows SMTC、任务栏控件、托盘控制和桌面歌词
- 按需下载并校验 FFmpeg，进行无损 WAV 转 FLAC 压缩
- SQLite 本地数据，无账号、无遥测、无云端媒体库

## 安装

从 [GitHub Releases](https://github.com/Leovikii/Resona/releases) 下载 Windows x64 NSIS 安装包。0.1.0 未使用 Authenticode，Windows 可能显示 SmartScreen 提示；请只从官方发布页下载并核对公开的 SHA-256。应用内更新仍使用加密签名验证。

## 开发

```bash
npm ci
npm run tauri dev
```

详见[开发准则](docs/DEVELOPMENT.md)。Resona 使用 [GPL-3.0-only](LICENSE) 许可证。
