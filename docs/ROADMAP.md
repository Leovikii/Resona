# 路线图

路线图只表达顺序，不承诺日期。事实见 [STATUS](STATUS.md)，产品边界见 [PRODUCT](PRODUCT.md)。

| 版本 | 阶段与范围 |
| --- | --- |
| 0.1.0 | Windows x64 首个正式版已发布，见[发行说明](releases/0.1.0.md) |
| 0.2.0 | 本地专辑体验增强已正式发布，见[发行说明](releases/0.2.0.md) |
| 0.3.0 | Arch Linux x86_64 / Wayland 基础兼容 alpha.1，见[计划与验收边界](plans/0.3.0.md) |
| 0.2.1 | 关闭询问与压缩进度 UI 优化已完成开发验证，见[计划与发行门禁](plans/0.2.1.md) |

后续修复按实际反馈确定范围；标签编辑、额外格式等能力不能以 UI 补丁扩大范围。

Linux 当前提供本机构建开发预览，复用领域层与 React UI；已实现 layer-shell 桌面歌词和 KDE MPRIS；正式发行格式、签名和多屏验收仍有开放门禁。

[跨平台预览交付](plans/0.3.0-preview-delivery.md)已接受并实现：Windows NSIS/Tauri 签名更新、Arch/AUR 安装和包管理器更新引导，保持 `0.3.0-alpha.1`。每个版本只发布一个普通版本 tag；待 main Actions 按此规则发布、AUR 配置与 Windows 原生覆盖更新验收。
