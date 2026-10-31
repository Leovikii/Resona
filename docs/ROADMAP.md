# 路线图

路线图只表达顺序，不承诺日期。当前事实见 [STATUS](STATUS.md)，产品边界见 [PRODUCT](PRODUCT.md)。

| 阶段 | 范围 |
| --- | --- |
| 0.1.0–0.2.1 | Windows 本地播放、专辑与压缩体验，已发布；见 [发布说明](releases/) |
| 0.3.0 alpha/beta | Arch/KDE Wayland、原生包更新、无边框材质、网络文件访问与歌词多屏支持，已发布 |
| 0.3.0-rc.1 | Linux 拖动残线修复及原生阴影，所有者验收通过；见 [当前计划](plans/0.3.0-rc.1.md) |
| 0.3.0 正式版 | 根据 RC 反馈收敛缺陷，沿用双平台 CI、签名和完整交付门禁 |

后续修复按实际反馈确定范围；标签编辑、额外格式等能力需独立范围决定。Linux 继续聚焦 Arch x86_64 / KDE Wayland，Windows 沿用 NSIS。每个版本只发布一个普通版本 tag，流程见 [ADR 0034](decisions/0034-main-only-ci-and-dependency-cache.md)。
