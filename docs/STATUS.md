# 开发状态

最后更新：2026-10-29

## beta.1 实施与待验收

- 当前开发版本为 `0.3.0-beta.1`。所有者已批准方案、实施、本机与可行 Windows 测试、交接及推送 `dev`；**Windows 实机测试完成前不合并，不发布**。[开发方案](plans/0.3.0-beta.1.md)、[ADR 0036](decisions/0036-native-file-access-and-focused-playback-controls.md) 已接受。
- 文件打开保留明确选择的目标，目录扫描空/失败/漏项只产生警告。Linux adapter 统一处理 `file://`、原始 `nfs://` 与 KIO-FUSE 物理路径，并通过 KIO-FUSE 官方 D-Bus 映射或刷新路径；Rodio 保持单引擎。
- 本机发现 KIO-FUSE 目录枚举不只返回空，还会使原可读 NFS 文件返回 ENOTDIR；按 GIO 挂载类型识别后跳过同目录音频、自动 CUE、伴随歌词、外置封面与递归目录发现，提示选择文件。内嵌封面与正常本地/内核挂载目录逻辑保留。临时映射失效后需重新选择。
- Linux 直接使用锁定的 ashpd 标准 XDG FileChooser API，保留完整 URI、取消/失败区别及真实 Wayland 父窗口；Windows 沿用 Tauri dialog 原生后端并指定父窗口。Arch 运行依赖已补 portal、KDE backend、KIO-FUSE；不修改用户桌面配置。
- 本机人工反馈补修 Dolphin 拖入 NFS：GTK adapter 保留原始 URI，避免锁定 Wry 提前解码损坏 `#`/`%` 文件名；主/压缩窗口消费统一逻辑坐标事件，Linux 不重复除以缩放。列表导入与压缩输入统一在后台解析网络位置，失败可诊断；真实 Dolphin 拖放仍需重建包人工复测。
- 本次工具启动的测试版英文已定位为继承 `LC_ALL=C.UTF-8`，覆盖系统 `LANG=zh_CN.UTF-8`；重启测试时清除此工具覆盖，不修改应用语言初始化或强制中文。原始 Dolphin 启动变英文仍需独立复现。
- 主/压缩窗口新增 Space 播放暂停、左右 ±5 秒、上下 ±10 个音量百分点。Rust actor 计算相对动作；窗口聚焦、可见且无弹窗阻挡时生效，控件操作优先。主窗口复用音量滑条弹层，压缩窗口用单条百分比/音量条反馈。
- 本机共享 Node 68 项、Rust 常规 131 项（15 项条件忽略）、fmt/Clippy、TypeScript/生产前端和嵌入前端 Release 构建通过；另有真实 FFmpeg 2 项、CUE 暂停定位及 NFS 完整解码/输出专项通过。拖放补修新增 URI 保真、取消清理、坐标、文件 URI 导入与去重检查，真实 NFS 原始 URI 导入专项通过。KDE chooser 的 OMV 选择和取消已由所有者操作，返回路径可读与取消结果自动断言通过。Release 桌面回归及两首 NFS 冷启动/第二实例、暂停/定位/继续通过；不将其当作真实鼠标拖放通过。详见 [Linux 验证](plans/0.3.0-beta.1-linux-validation.md)。
- 已推送 dev，并创建保持草稿的 [PR #23](https://github.com/Leovikii/Resona/pull/23) 运行双平台 CI；不合并发布。首轮 Windows 前端/构建、许可证、fmt 与 Rust 139 项通过（12 项条件忽略），Arch job 通过；仅 Linux 构造的共享错误枚举已按 ADR 0028 补非 Linux 局部 lint 标注，最终双平台门禁以 PR 最新 SHA 为准。CI 不等于 Windows 原生通过；MSVC/CI 与实机操作的独立清单见 [Windows 交接](plans/0.3.0-beta.1-windows-validation.md)。界面变英文仍未复现，不声明已修复；原生网络中断、物理听感与既有硬件/动画门禁仍保留。

## alpha.4 实施与验证

- 所有者本机实测功能正常，并于 2026-10-28 确认标题栏优化后的测试程序通过。已将共享主/压缩标题栏图标画布统一为 16px、实际线宽 1.5px、宽窄标题统一 13px/400，中性灰随明暗切换；保留品牌图标、44×48px 命中区、8px 分组间距、紫色选中与关闭红色悬停。
- 标题栏优化后的 TypeScript/Vite/Tauri Release 构建通过；真实前端覆盖主窗口四个规定尺寸与压缩 720×500 的代表性明暗组合，无横向溢出，五按钮测得 44×48px、SVG 16px。新测试程序从 `/tmp` 独立启动、隔离数据及 MPRIS 退出通过；所有者已确认本机最新程序测试通过；不将该确认扩展为未逐项记录的硬件或性能测试。
- 版本已提升为 `0.3.0-alpha.4`。所有者已授权更新文档、推送 dev、创建并合并到 main 的 PR、将 dev 快进同步到合并后的 main，并通过 main 双平台流水线发布。交付结果以 [main Actions](https://github.com/Leovikii/Resona/actions/workflows/main-merge-delivery.yml) 和 [alpha.4 Release](https://github.com/Leovikii/Resona/releases/tag/v0.3.0-alpha.4) 为准。
- 正式主/压缩窗口采用共享无边框标题栏；宽屏侧栏字标已移除，窄屏工具/设置并入 48px 标题栏，36px 标签行保留排序、拖放、菜单。
- Windows 保留 Mica，新增 Win32 最大化按钮命中、Snap hover 路径与系统菜单；关闭仍进入现有生命周期。KDE 接入新旧背景效果协议探测、区域更新、能力撤回回退与 GTK 同连接生命周期；所有者已完成本机功能验收，专项边界见下文。
- 前端 TypeScript、生产 build、Node 59 项（4 项平台跳过）通过；Windows Rust 134 项通过、10 项条件忽略，fmt/Clippy、嵌入前端 debug 构建通过。[第二轮 CI](https://github.com/Leovikii/Resona/actions/runs/40667001692) Windows validation 成功。
- 本机 CachyOS / KDE Wayland 已核对远端 dev `f5e7dee`：Linux Node 12 项与完整 Node 63 项、Rust 124 项（12 项条件忽略）、fmt、Clippy、TypeScript/生产前端和嵌入前端 Release 构建通过。另有真实 FFmpeg/音频条件测试 9 项、真实 Arch 包只读元数据/事务 1 项及歌词 layer-shell 生命周期 1 项通过；许可证重新生成无差异。未再次触发 Actions。
- 正式 App 的四个规定尺寸、720×500 压缩窗口、代表性明暗/中英、长名称新建、工具/设置、添加菜单/Escape 与焦点恢复已检查。Windows 原生最大化/还原可操作；自动化后续无法读取原生窗口，未确认 Snap hover、Alt+Space、Mica 视觉与真实 DPI。
- 本机原生 Release 与独立测试目录的桌面集成通过：MP3/WAV/FLAC 中文/空格路径、单实例、歌词 ready/重建、MPRIS 控制与退出、每进程一次启动更新检查。旧脚本的“Linux 不检查更新”断言已按 alpha.3 规则修正。KWin 新协议 Blur capability 与排除正文的区域请求已确认；所有者已确认功能与标题栏优化通过；专项菜单、阴影/圆角、真实鼠标命中未逐项记录，GPU/功耗未测。
- 原生窗口用例的尺寸/模式记忆、并发 resize、最大化恢复、压缩重建、两个窗口的最大化/最小化请求和 CloseRequested 均到达并通过；主窗口 maximize/squash、压缩 maximize 动画已捕获。**压缩窗口 squash 动画未捕获，整项严格用例仍失败**，未降低门禁。测试已修正聚焦插入 resize/configure 往返及仅等待首帧造成的竞态。
- 最新本机测试程序为忽略目录 `src-tauri/target/test-build/Resona-0.3.0-alpha.4-titlebar-v2-arch-wayland-x86_64/` 与同名 `.tar.gz`；原始 alpha.4 测试目录保留，含隔离启动器、低音量 WAV/LRC 样本、许可证及 SHA256SUMS。无需构建工具；依赖本机系统库。详细结果和人工清单见 [Linux 实机交接](plans/0.3.0-alpha.4-linux-validation.md)。
- 所有者最新要求：**PR 指向 main 时运行双平台 CI，`PR validation` 通过且跟上 main 才允许合并；main 更新只执行 CD**。远程规则已恢复 GitHub Actions 来源的必需检查，并保留 PR 合并、禁止强推和删除。PR CI 不接触签名密钥，同一 PR 取消过时 CI；main 发布串行，保留签名和双平台产物门禁。依赖缓存继续使用，详见 [ADR 0034](decisions/0034-main-only-ci-and-dependency-cache.md)。
- [PR #21](https://github.com/Leovikii/Resona/pull/21) 已通过双平台 CI 合并到 `ac8e2b8`，dev 已同步；原 alpha.4 曾完成双平台签名发布。所有者现授权将 Arch 包统一为 `resona`，不加旧包名兼容代码，更新文档并删除 alpha.4 Release/tag 后由 main CD 重发同版本；PR CI 与签名门禁保持。已有同版本测试安装需手动重装。见 [ADR 0035](decisions/0035-unified-arch-package.md)。
- 统一包名修订的本机 Node 64 项、Rust 125 项（12 条件忽略）、fmt/Clippy/actionlint 通过；生产代码仅接受 `resona`，不增加旧包兼容。隔离 Arch CI 增加同包名预览→正式包事务验证，图形授权与实际运行中的跨版本更新仍按开放边界记录。
- 正式前端验收页：`/alpha4-preview.html?production=1`，iframe 加载真实 App；普通 `/alpha4-preview.html` 仍是已批准的独立设计稿。

## alpha.3 历史交付


- 版本为 `0.3.0-alpha.3`，所有者已批准实施并授权 dev → main PR、合并交付及分支同步。[开发方案](plans/0.3.0-alpha.3.md)、[ADR 0032](decisions/0032-arch-in-app-update.md)。
- 旧 alpha.1 / alpha.2 两条 Release 及资产已按授权删除，其他 Releases、Git tags 与历史提交保留。旧用户首次手动安装 alpha.3，用户列表、偏好和音频保留。
- Linux 收敛到 GitHub 原生包安装与应用内 Tauri 下载验签、事务预览、Polkit/Pacman 安装；删除 AUR 发布与指引。仍只支持 Arch 系 x86_64 / KDE Wayland。Windows 与现有 UI 设计沿用。
- 已实现双平台签名清单、受限临时包、身份/摘要/事务复核、操作单飞、转换/扫描互斥、安装期间退出保护与安装后数据库确认。只支持同包名升级，预览/稳定包切换需手动安装。
- 显式设置 updater 检查 15 秒、下载 600 秒超时；锁定插件不会自动将检查超时传给下载。Pacman 安装不受该超时强杀。
- [PR #18](https://github.com/Leovikii/Resona/pull/18) 已通过[双平台门禁](https://github.com/Leovikii/Resona/actions/runs/40629669622)并合并，初始实现提交为 `bee44e5`。超时修正和验证记录由 [PR #19](https://github.com/Leovikii/Resona/pull/19) 跟踪。Actions 使用内部 PKGBUILD 与 Arch PR 包安装测试；签名仅在 main 的 release Environment，Windows/Arch 同 SHA 完成后统一草稿验签公开。[main 交付流水线](https://github.com/Leovikii/Resona/actions/workflows/main-merge-delivery.yml)，公开包入口为 [alpha.3 Release](https://github.com/Leovikii/Resona/releases/tag/v0.3.0-alpha.3)。

## alpha.3 历史验证

- Windows 本机 Rust：133 项通过、10 项按硬件/工具条件忽略；包括新的更新互斥、文件任务安装互斥、Arch 事务解析、错平台/版本包与签名清单校验。Clippy、fmt 通过。
- PR 的 Windows 与 Arch 编译、Rust、Clippy、前端及发布契约检查均通过。Arch 常规 Rust 123 项通过、12 项条件忽略；另外 2 项真实 FFmpeg 和 1 项 Pacman 元数据测试通过，真实包首装、alpha.2→alpha.3 升级、重装、桌面注册和卸载通过。
- Node：58 项通过、4 项平台专属跳过。TypeScript、生产前端构建、许可证清单生成通过。actionlint 1.7.12 通过。
- Windows Tauri debug `--no-bundle` 嵌入前端程序构建通过；原生窗口自动化因应用授权超时未完成，不能据此声称原生视觉通过。
- 浏览器更新预览覆盖 1080×700、760×520、420×720、360×600，中英/明暗代表组合、20 项长依赖列表、完成提示、等待阶段和 Escape/重复安装保护；控件可达。这只证明共享 UI，不证明系统授权与安装。
- 既有 alpha.2 KDE 主题、应用身份、歌词和窗口实机结论保留在历史计划；本轮未修改这些 adapter，也未在此 Windows 主机重跑 KDE 会话。

## 开放门禁与限制

- beta.1 已替换 Linux GTK chooser 并修复所选 NFS 文件的目录依赖；KIO-FUSE 递归目录发现不支持，需选择具体文件。网络断线/权限失败与恢复仍需实机专项；不能将当前会话的映射当作永久网络列表路径。
- 整套界面变英文尚未复现：此前隔离普通启动、直接 NFS 路径与 KDE ApplicationLauncherJob 均测得 `zh-CN`，用户偏好为 `system`。本轮不改语言初始化，继续保留故障状态采集门禁。
- 当前主机为 CachyOS / KDE Plasma 6.8.1 / 原生 Wayland，双屏均为 150% 缩放；本轮未更改系统安装版。alpha.4 Arch 包已构建并只读验证，实际 Pacman 首装/升级/重装/卸载未跑：专用脚本只允许 disposable Actions root 容器，不在个人系统执行。Polkit 授权、应用内跨版本升级与重启仍未验收。
- Linux 系统 `LocalFileSigLevel` 不变；Tauri `.tauri.sig` 不是 OpenPGP。无图形授权代理、包锁、冲突或严格本地签名政策时明确失败，程序不删锁、不关闭检查、不自动全系统升级。
- Windows NSIS 真实跨版本覆盖、取消、卸载及完整 SMTC/托盘/Mica/文件关联仍需安装版实机回归；本机测试和 CI 构建不能替代。安装包尚无 Authenticode，Tauri 包与清单签名保留。
- 多屏/DPI、物理媒体键、睡眠/蓝牙恢复等既有人工门禁保持；歌词跨输出拖动/热拔插恢复、托盘宿主热重启恢复未实现或未验收。Linux 转换目标文件系统需支持同卷硬链接；不支持则保留源文件并失败。
