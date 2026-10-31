# 开发状态

最后更新：2026-10-31

## 当前版本：0.3.0-rc.1

Linux 普通窗口拖动残线修复与 KWin 原生柔和阴影已实现，所有者确认最终测试程序通过，并授权 dev → main PR、最新双平台 CI 成功后合并发布、随后同步 dev。Windows 实现未改动，所有者接受其功能正常；这不是本轮 Windows 实机复测记录。

- 主/压缩窗口的原生模糊区域四周内收 1 个 surface-local 逻辑像素，保留自绘标题栏、窗口尺寸与输入区域。对照实验与验收支持 blur 和窗口外缘交互的判断，尚不能确定 KWin/驱动内部根因。
- 阴影使用同一 GTK Wayland 连接的 KWin 协议、固定 24px 八块渐隐贴片；最大化/全屏时移除，还原复用缓冲区。缺失协议时保留无阴影窗口。阴影与 blur 独立，保留已验收的 1px 内收。
- 修复 GTK 先销毁 surface 再发清理信号时的生命周期问题：清理仅释放自有对象，重显时重建并恢复正文区域。无新增依赖、数据迁移或设置。长期边界见 [ADR 0033](decisions/0033-frameless-titlebar-and-kde-material.md)。

## 验证与交付

- Linux 本机 Rust 库测试 139 通过 / 22 条件忽略；fmt、嵌入前端 Clippy、TypeScript/Vite、Linux/交付/通道 Node 21 项、许可证无差异、正式及隔离标识的嵌入 Release 构建通过。
- KDE Wayland 阴影生命周期专项通过：主/压缩、最大化/全屏、隐藏重显、正文保留和压缩重建；协议审计无旧 surface 请求，48 个 tile buffer 与 12 个阴影对象创建/销毁配对。
- 最终测试程序的 MP3/WAV/FLAC 打开、单实例、歌词重建、MPRIS 控制/退出与启动更新单飞回归通过；所有者确认残线修复及新增阴影测试通过，未逐项说明的场景不补写为独立通过。
- 保留最新阴影测试包和旧测试配置，压缩诊断证据并清除旧测试二进制；已完成计划从活动文档移除，历史见 Git 与 [发布说明](releases/)。有效回归测试继续保留。
- 合并门禁仍为最新 SHA 的 [PR validation](https://github.com/Leovikii/Resona/actions/workflows/pr-validation.yml) 双平台成功。合并后由 [main CD](https://github.com/Leovikii/Resona/actions/workflows/main-merge-delivery.yml) 构建、验签并发布；交付状态以 Actions 和 [rc.1 Release](https://github.com/Leovikii/Resona/releases/tag/v0.3.0-rc.1) 为准，不将授权或本机通过写成已发布。

当前范围与验证见 [rc.1 计划](plans/0.3.0-rc.1.md)；已知限制集中维护在 [KNOWN_ISSUES](KNOWN_ISSUES.md)，不重复历史验收流水。
