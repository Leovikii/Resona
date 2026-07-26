# 开发状态

最后更新：2026-07-26

## 当前版本

包版本已统一为 `0.1.0`，状态为 **Ready for owner release**。功能、品牌和文档收口
已完成；项目所有者发布正式 GitHub Release 后再标记为 Completed。

当前门禁见 [0.1.0 计划](plans/0.1.0.md)，公开说明见
[0.1.0 release notes](releases/0.1.0.md)。

## 已完成

- Rust 继续拥有播放、列表、偏好、压缩、更新和原生窗口权威状态；React 只消费
  类型化快照并提交意图。Rodio 是唯一播放引擎，FFmpeg 只用于用户明确启用的 WAV
  到 FLAC。
- MP3/WAV/FLAC 播放、临时默认列表和持久化用户列表、本地歌词、完整播放器、桌面
  歌词、SMTC、任务栏、托盘、文件关联、单实例和应用生命周期已完成回归与 Windows
  实机功能验收。
- 文件夹导入由 Rust 递归扫描并保留可折叠目录上下文；播放列表顺序仍是唯一播放
  序列。轻量元数据摘要、文件信息、系统文件管理器定位和当前曲目定位均按需执行。
- 当前曲目元数据和最大 512 px 封面由有界缓存共享给 React、SMTC 和任务栏。切歌
  时在新封面解码和信息就绪后共同提交；窄屏默认页按“有效歌词 → 真实封面 →
  播放信息”回退。
- 宽/窄底栏、透明辅助控件、圆形实体播放键、主题级无位移按下态和 10% 步进的
  字体/背景透明度、音量实时拖动已经统一。右下角临时反馈均调用共享 Mantine
  `TransientNotice`，使用语义图标、极简正文和 4 秒自动退出。
- 压缩窗口使用有界 1–4 worker、独立临时文件、结果验证与原子提交；源文件只在
  成功提交后允许删除。关闭窗口不会中断活动批次，终态会按既定生命周期清理。
- 品牌系统以唱片机主图标和唱片视窗式 `Resona` 字标为基石；默认封面、MP3/WAV/
  FLAC 图标和 NSIS header/sidebar 均由正式 SVG 与统一色板可重复生成。安装器位图
  不包含固定语言宣传文案。

## 发布与签名

- 0.1.0 只发布 Windows x64 NSIS `currentUser` 安装包；Linux 实现留待独立计划。
- Tauri updater 签名是正式发布硬门禁，密钥只由受保护的 GitHub `release`
  Environment 注入。
- 项目所有者决定 0.1.0 不采用 Authenticode，也不使用自签名证书占位。安装包可能
  触发 SmartScreen，发布页必须指向官方 GitHub Release 并提供 SHA-256。
- 交付仍使用受保护 PR 合并与单一 `main-merge-delivery.yml`；本地不保存发布密钥，
  不提供 PowerShell 发布脚本或平台专属签名依赖。

## 最终本地门禁

- 模拟 PR 流程通过：干净 `npm ci`、35/35 前端与发布规则测试、生产构建、39 个
  音频夹具、三套品牌资源复现、许可证无差异、工作流 lint、Rust format 和 Clippy。
- Rust 全量测试为 102 通过、9 个依赖真实音频设备或固定 FFmpeg 工具的用例按设计
  忽略。稳定版默认关闭预览更新，预览构建默认开启，两种默认值与偏好持久化均有覆盖。
- 文件图标生成器已在模拟 Windows CRLF checkout 中验证，主 SVG 的换行格式不再
  影响 MP3/WAV/FLAC 派生 SVG 的字节复现。
- 本地 unsigned 验证安装包：
  `src-tauri/target/release/bundle/nsis/Resona_0.1.0_x64-setup.exe`，
  6,308,037 bytes，SHA-256
  `1F5DE37BD6FFB875A11FAB5D42B2B9722CF9F9343F7172C5C93C5A47BD906E40`。
  它只用于本机验证；正式 Release 必须由受保护环境生成 updater `.sig` 与
  `latest.json`。

## 已知限制

- Windows 原生文件选择窗口仍存在无稳定复现的低频阻塞，重新调查条件见
  [KNOWN_ISSUES.md](KNOWN_ISSUES.md)。
- 32-bit integer/float WAV 转换为 24-bit FLAC；不支持播放 32-bit FLAC。
- 多显示器、DPI、睡眠和音频设备恢复仍应随实际环境与问题报告继续抽查。

## 所有者待办

1. 确认受保护 GitHub `release` Environment 中的 updater 公钥、私钥和可选密码。
2. 经 PR 将 `0.1.0` 合并到 `main`，由交付工作流生成 stable Release 和签名更新产物。
3. 公布安装包 SHA-256，并从正式渠道验证下载、安装、文件关联和一次应用内更新。
