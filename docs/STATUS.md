# 开发状态

最后更新：2026-10-27

## 当前版本与交付

- 应用版本为 `0.3.0-alpha.2`，提供 Windows x64 与 Arch 系 Linux x86_64 / Wayland 预览；本机验证环境为 CachyOS / KDE Plasma / PipeWire。[版本计划](plans/0.3.0.md)、[Linux 指南](LINUX.md)。
- `0.3.0-alpha.2` 已实现 KDE portal 主题同步、GTK/Wayland 应用身份、歌词离开恢复与双平台解锁图标空闲隐藏；[实施与测试记录](plans/0.3.0-alpha.2.md)。本机原生测试确认主窗口/压缩窗口的 KWin maximize/squash 动画启动；Windows 原生消息测试交由 CI，真实鼠标、DPI 和 Windows 材质仍保留实机验收边界。
- 按所有者反馈恢复每版本一个 tag/Release：`v0.3.0-alpha.1`、`Resona 0.3.0-alpha.1`、pre-release。同版本合并不重复发包，Windows 更新只接受更高 SemVer；运行编号仅作诊断。[ADR 0031](decisions/0031-version-release-tags.md) 取代 ADR 0030 的重复构建发布部分。
- 已删除错误的 `preview-v0.3.0-alpha.1-b28.1-g03c89e0` 草稿、`preview-v0.3.0-alpha.1-b30.1-gc54bc92` pre-release 及后者实际存在的 Git tag；已核实两者均不存在，其他历史版本未改动。
- 当前本地基线已包含 PR #15 的合并提交 `d5c785e`；公开 Release 与 AUR 状态本轮未重新查询。版本发布由 main Actions 在双平台验证与构建成功后汇总验签、上传、核对完整资产并公开；失败上传清理草稿，已公开版本不覆盖。发布说明统一读取 [极简英文 Markdown](releases/0.3.0-alpha.2.md)。本机产物只测试，不提交或上传。
- README 中英文已补齐 Windows NSIS 和 Arch AUR 推荐安装方法、原生包替代安装及更新说明。预览包 `resona-preview-bin`，未来稳定包 `resona-bin`；AUR 查询尚无包，首次同步仍待配置，未宣称命令已可用。

## 已验证能力

- 本机 Tauri 开发/Release、嵌入前端离线启动与 IPC、系统 FFmpeg 检测/转换矩阵、真实设备 MP3/FLAC/高采样率 WAV 播放及错误恢复通过。SQLite 用户列表迁移、CUE 逻辑曲目、封面和文件写入安全回归保留。
- Wayland 歌词使用 layer-shell overlay 置顶、锁定空输入区域穿透与 GTK 解锁按钮；原生自动回归覆盖首次 ready、关闭/重建和退出清理。MPRIS 实测覆盖封面/转义 URI、播放/暂停/停止、前后曲、音量、循环/随机、定位、Raise/Quit。
- 系统默认字体已统一；本机测试启动器保留桌面配置与用户字体，只隔离应用数据。GTK 原生标题栏命中区域、启动等待和控件尺寸问题已修复；所有者已确认点击等待问题修复。
- 主窗口以逻辑内容区尺寸保存宽/窄模式，过滤旧查询和切换中间 resize，修正 GTK 装饰尺寸累加。原生回归覆盖连续切换、默认/最小尺寸、各模式尺寸记忆、最大化恢复和压缩窗口重建；所有者已确认宽窄切换和尺寸问题修复。
- 本机安装布局测试覆盖 `gio launch` 冷启动、三种音频交给已有实例、中文/空格路径、歌词重建与 MPRIS；Linux 启动无更新检查。Actions 已验证双平台构建、Windows 签名 NSIS、Arch 干净容器安装/文件关联/卸载；公开 `SRCINFO` 避免 GitHub 点文件重命名，内部和 AUR 保留 `.SRCINFO`。
- 本轮 63 项 Node 测试通过，含版本 tag 去重、真实 `vercmp`/`makepkg`、双平台资产/摘要、草稿重试/失败清理/已公开保护、MIME 与 AUR 幂等/拒绝降级；119 项 Rust 常规测试通过，11 项原生/设备用例默认忽略。签名 fixture 使用一次性测试密钥重新生成，私钥已删除；有效签名、CRLF/篡改拒绝通过。生产前端、Rust format、Clippy、许可证清单、actionlint 与差异检查通过。

## 开放门禁与限制

- 上轮留下的 PR 双平台 CI、正常版本 tag 发布与 AUR 首次同步验收记录需按实际远端结果更新，本轮未重验。AUR 同步要求包维护权限、`RESONA_AUR_ENABLED=true`、受 main 限制的 `aur` Environment、部署密钥与核实的 SSH 主机公钥，见[交付方案](plans/0.3.0-preview-delivery.md)。
- Windows 原生 NSIS 覆盖/取消、签名错误、卸载及 SMTC/托盘/Mica/文件关联回归仍需 Windows 实机；本机 Linux 结果不能替代。安装包尚无 Authenticode，README 已披露 SmartScreen；Tauri 安装包与清单签名保留。
- 真实多屏/DPI、分数缩放字体清晰度、物理媒体键、睡眠/蓝牙恢复、原生文件选择/拖入、标题栏拖动和歌词鼠标手感仍需所有者验收。歌词当前只在选定输出内移动，跨输出拖动和输出热拔插恢复未实现。
- 托盘宿主只在启动检查；宿主热重启恢复未验收。无托盘时拒绝隐藏主窗口。既有 libayatana-appindicator 弃用提示不阻止启动。
- Linux 依赖系统 `gtk-layer-shell`、FFmpeg 等；本机隔离测试包附带私有 layer-shell，不等同于原生安装包。Linux 转换目标文件系统须支持同卷硬链接，否则明确失败并保留源文件。
- Linux 只提供包管理器更新指引，不构建 AppImage 或应用自更新；其他发行版、X11、ARM 不在当前范围。32-bit WAV 可转 24-bit FLAC，不支持播放 32-bit FLAC。
