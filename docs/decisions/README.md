# 架构决策记录

ADR 记录会长期影响代码、依赖或产品边界且无法仅从实现看清原因的决定。

状态使用：`Proposed`、`Accepted`、`Superseded`、`Rejected`。

## 索引

- [0001 - 技术栈基线](0001-technology-baseline.md)
- [0002 - mpv sidecar 播放架构（已替代）](0002-mpv-sidecar.md)
- [0003 - UI 依赖与解耦策略](0003-ui-dependency-policy.md)
- [0004 - 项目采用 GPL-3.0-only](0004-project-license.md)
- [0005 - Rodio 播放核心](0005-rodio-playback-core.md)
- [0006 - 应用壳、主题与语言](0006-application-shell-theme-and-locale.md)
- [0007 - Windows 系统媒体会话 adapter](0007-windows-media-session.md)
- [0008 - SQLite persistence boundary](0008-persistence-storage.md)
- [0010 - read-only metadata index adapter](0010-metadata-index-adapter.md)
- [0011 - local timed lyrics formats](0011-local-timed-lyrics.md)
- [0012 - 桌面歌词窗口与原生解锁辅助窗口](0012-desktop-lyrics-window-and-unlock-helper.md)
- [0013 - Windows 应用生命周期与桌面歌词控件归属](0013-windows-lifetime-and-desktop-lyrics-controls.md)
- [0014 - 播放器单一范围与临时默认播放列表](0014-player-only-scope-and-transient-default-playlist.md)
- [0015 - 播放列表激活、队列入口与单一状态展示](0015-playlist-activation-and-single-state-presentation.md)
- [0016 - 音频压缩独立工作窗口](0016-audio-compression-workspace-window.md)
- [0017 - 完整播放器作为右侧主内容视图](0017-full-player-as-main-content-view.md)
- [0018 - 播放列表即当前播放序列](0018-playlist-as-playback-sequence.md)
- [0019 - 临时默认列表退出语义与桌面歌词三分区工具栏](0019-transient-default-playlist-and-lyrics-toolbar.md)
- [0020 - 主窗口宽屏/窄屏模式与几何恢复](0020-main-window-layout-modes-and-geometry.md)
- [0021 - 窄屏顶部导航、播放列表标签与单行底栏](0021-compact-top-navigation-and-playlist-tabs.md)
- [0022 - 原生窗口材质与表面层级](0022-native-window-materials-and-surface-hierarchy.md)
- [0023 - Windows Shell 媒体与任务栏集成](0023-windows-shell-media-integration.md)
- [0024 - Windows 托盘生命周期、分发与更新基线](0024-windows-tray-distribution-and-update.md)
- [0025 - 公开仓库首页与 Agent 规范归位](0025-public-repository-surface-and-agent-guidance.md)
- [0026 - CUE 逻辑音轨与本地封面回退](0026-cue-logical-tracks-and-artwork-fallback.md)

- [0027 - 压缩活动分支、整体进度与关闭询问](0027-compression-progress-and-close-dialog.md)

- [0028 - Arch Linux / Wayland 初始兼容](0028-arch-linux-wayland-baseline.md)

新 ADR 不修改历史决定的原文；如果决定被替代，新增 ADR 并将旧记录标记为 `Superseded`。

- [0029 - Wayland 置顶桌面歌词与 KDE 媒体集成](0029-wayland-lyrics-and-mpris.md)

- [0030 - 同版本预览构建、Arch 分发与更新归属](0030-preview-builds-and-linux-distribution.md)

- [0031 - 每个应用版本只发布一次](0031-version-release-tags.md)

- [0032 - GitHub Arch 原生包与应用内更新](0032-arch-in-app-update.md)
- [0033 - 无边框标题栏与 KDE 原生模糊外壳](0033-frameless-titlebar-and-kde-material.md)
- [0034 - 仅 main 更新触发 CI 与依赖缓存](0034-main-only-ci-and-dependency-cache.md)
