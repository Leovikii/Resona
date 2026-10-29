# 开发状态

最后更新：2026-10-29

## beta.1 实施与待验收

- 当前验证主机为 Windows 11 x64 build 26200；以下 CachyOS/KDE 结果是此前 Linux 实机记录。本轮 `991f045` 的 Node 71 通过/4 平台跳过、Rust 145 通过/12 条件忽略，另 9 项真实输出/CUE/SMTC 和 1 项固定 FFmpeg 转换矩阵通过；fmt/Clippy、前端/嵌入 Release、许可证/品牌资源/fixture/工作流检查通过。[最新基线双平台 CI](https://github.com/Leovikii/Resona/actions/runs/40888887844) 通过，后续提交仍需新 CI。详情及人工清单见 [Windows 验证](plans/0.3.0-beta.1-windows-validation.md)。
- 所有者本轮授权 Windows 测试通过后合并 PR #23、由 main CD 发布并将 dev 快进到 main；原生人工结果尚未完整确认，当前不合并发布。Windows 测试版使用独立标识隔离现有数据；没有执行 NSIS 覆盖/卸载，不提交私人媒体路径。
- Windows 映射网络目录 19 个 FLAC/对应歌词可读，一首真实 FLAC 的目录/去重、完整解码、seek 和低音量暂停/继续专项通过。测试移除“200 毫秒必已启动”的固定等待，最多 5 秒等待实际进度，并在暂停后读取权威快照；不修改播放生产逻辑，不将单首专项扩展成全目录播放或听感通过。
- 所有者要求禁止鼠标模拟等低效交互测试。Windows 无法自动覆盖的桌面交互交由人工；Linux 自动化由 Linux 平台的 AI agent 和测试脚本执行，包括依赖 KDE/Wayland、D-Bus 和真实输出的条件测试，Arch CI 覆盖容器检查。当前 Windows 主机缺少 Linux 环境的项目标为待 Linux agent 执行，不改派人工。已删除 chooser 测试中的人工点击分支与 KWin 动画轮询，尺寸/重建各保留两次往返；历史压缩最小化动画未捕获保留为未通过记录，视觉和物理操作仍人工验收。规则见 [开发规范](DEVELOPMENT.md#双平台精简执行)。

- 播放列表外部拖放已扩大热区：宽屏侧栏空白处可拖入创建列表，固定 44px 高新建按钮仅用于点击，已有列表整行高亮追加；宽/窄屏间隙使用 28px 几何吸附且不撑开布局，窄屏加号为 44×32px 普通按钮、标签末尾空白可拖入新建。窄屏用户标签右侧增加悬停/聚焦删除按钮，预留位置且不嵌套按钮；宽/窄高亮不再被普通悬停覆盖。末尾新建仅显示空白区高亮，隐藏同位置插入线，避免边框重叠；正文底部固定显示新建/追加目标，沿用导入互斥和边缘滚动。共享 Node 75 项、TypeScript/Vite/嵌入 Release 与桌面生命周期回归通过，四种规定视口代表性明暗及新建/追加预览通过；原生 Dolphin 拖影与实际落点待所有者验收。

- 所有者已于 2026-10-29 确认网络导入性能达到可接受水平，本轮性能优化验收完成；后续工作转向播放列表拖放的热区与可见反馈。此确认不扩展到 Windows 实机、真实断网或网络 WAV 转换验收。

- 当前开发版本为 `0.3.0-beta.1`。[开发方案](plans/0.3.0-beta.1.md)、[ADR 0036](decisions/0036-native-file-access-and-focused-playback-controls.md) 已接受；Windows 人工验收通过或所有者明确接受保留项前不合并发布。
- 网络文件夹支持已实施：Dolphin 拖到列表区域创建列表、拖到内容区追加、手动添加共用 Rust 目录边界。真实两个 NFS 目录分别导入 14、2 项，创建、追加、同请求 URI/映射去重、默认列表、名称及导入根上下文测试通过，导入后原文件仍可读。三个入口的后端已验证；本轮尚未重新人工操作各鼠标落点与文件夹选择器，不把后端通过等同于原生交互全部通过。
- 本机 KIO ListJob 返回正常子项，但“.”类型为普通文件（32768）；KIO-FUSE 源码会据此更新目录节点，可解释此前 readdir 空及 ENOTDIR。现在对 KIO-FUSE NFS 目录直接用 KIO ListJob 排除“.”/“..”列举，文件继续走官方 D-Bus 映射与 Rodio；不直接对故障 FUSE 目录 readdir。尚未确认对应上游缺陷编号，不推断服务端内部原因。
- 最小补修编译进同一可执行文件，re-exec 内部工作模式先于 GTK/单实例初始化，在独立 QtCore 事件循环列目录；普通目录仍用标准库。递归导入、同目录音频、CUE、歌词、封面及压缩扫描共用边界，一次导入复用 CUE 候选。未引入 libnfs、sidecar、新播放引擎或数据库迁移。Arch 构建/运行依赖与许可证已同步系统 Qt/KIO。
- 工作任务 15 秒、父进程 20 秒兜底并回收进程，条目/响应有界且校验子名称。本轮保留地址不可达测试 20.20 秒由父进程兜底返回错误；这不是 OMV 真实关机验证，也不保证所有 FUSE stat/read 同样可中断。所有者曾因内核挂载离线卡死 Dolphin 改用网络路径，本轮不要求重新挂载或修改系统配置。
- 文件打开保留明确选择的目标，目录扫描空/失败/漏项只产生警告。Linux adapter 统一处理 `file://`、原始 `nfs://` 与 KIO-FUSE 路径；临时映射失效后需重新选择，持久列表仍存物理路径，不提供注销/机器重启后的永久 URI 恢复。
- Linux 直接使用锁定 ashpd 标准 XDG FileChooser API，保留完整 URI、取消/失败区别及真实 Wayland parent；Windows 沿用 Tauri dialog 原生后端并指定 parent。所有者已看到 OMV、选择 NFS 音频并取消第二次对话框，返回可读与取消自动断言通过。GTK adapter 保留 Dolphin 原始 URI 和统一逻辑坐标，所有者已确认单首 NFS 拖入成功；多文件/取消及其他落点仍待验收。
- 跟随系统语言通过 GLib / Win32 GetUserPreferredUILanguages 初始化，手动偏好优先；修复本机 WebKit 报告 `C` 导致英文，所有者已确认新版主界面中文。旧的“暂不支持网络文件夹”保护和专用文案已删除；之前“提示明确”只代表文案检查，不能计为目录功能通过。辅助窗口、Dolphin 冷启动和 Windows 语言仍需独立验收。
- 主/压缩窗口支持 Space 播放暂停、左右 ±5 秒、上下 ±10 个音量百分点。Rust actor 计算相对动作；窗口聚焦、可见且无弹窗阻挡时生效，控件操作优先。主窗口音量滑条弹层、压缩窗口百分比/音量条提供反馈。
- 本机共享 Node 71 项、Rust 常规 134 项（17 项条件忽略）、fmt/Clippy、TypeScript/Vite 与嵌入前端 Release 构建通过；另补同目录/单文件/递归导入 CUE 回归。真实两首 NFS 完整解码 EOF、seek、低音量输出、暂停/继续及上述文件夹专项通过。最新 Release 的 MP3/WAV/FLAC 关联、单实例、冷桌面打开、歌词 ready/重建、MPRIS 元数据/控制/定位/音量/退出与启动更新单飞再次通过。见 [Linux 验证](plans/0.3.0-beta.1-linux-validation.md)。
- 本机测试目录仍为 `src-tauri/target/test-build/Resona-0.3.0-beta.1-arch-wayland-x86_64/`，保留原 test-profile，启动器与同名压缩包同步更新；未替换系统安装版。
- 已实施目录性能补修：同一内部 KIO 工作进程串行复用连接/句柄，换行 JSON 协议有界，每请求保留 15 秒原生任务期限和 20 秒父进程读取兜底；失败回收、下一请求重建，父进程关闭输入后退出。不增加全局目录缓存。真实 25 目录/325 条目的树：旧逐目录工作进程列举约 31.29 秒；复用后的暖列举约 41–57 毫秒。应用首次创建并提交列表为 0.32–1.34 秒，暖递归约 60–89 毫秒；同音频结构本地目录约 1 毫秒，未宣称达到内核挂载性能。
- 修复 KIO-FUSE remoteUrl 的可读 URI（空格、方括号、中文）被严格 QUrl 拒绝的问题，采用 Qt 标准宽容解析并保留协议/主机/凭据校验。所有者补充目录的原始/编码 URI 一致，19 首音频完整递归、相对路径/根上下文/追加均通过；真实私有路径只存在本机测试数据。
- 默认/用户列表、外部打开和音频压缩输入复用同一文件任务门禁，覆盖网络解析、扫描与列表提交，忙时立即拒绝新添加/扫描，不排队。前端共享导入 gate 与 Rust 快照，主/压缩窗口禁用添加/拖入，超过 200 毫秒显示正文内扫描反馈；播放控制可继续。跨线程拒绝、扫描/安装/转换互斥、失败/结束释放已测试。压缩窗口真实遍历同一树通过，目录无 WAV，因此候选为 0；本地 WAV 验证矩阵仍通过，不将其等同于网络 WAV 转换验收。
- 右下角反馈沿用 Mantine 原生 Notification 色条、边框与关闭按钮，移除自定义圆形图标，统一圆角并限制宽度；错误/警告保留供关闭，普通反馈按原时限消失，音量条保留。参考 Mantine Notification 与 WinUI 3 InfoBar 的主题、严重级别及文本换行原则，不引入新组件库。
- 本轮共享 Node 73/73、Rust 136 通过/20 条件忽略，另有 4 项真实 NFS 导入/音频/压缩遍历与 1 项工作进程复用/恢复专项通过；不可达目录 20.20 秒由父进程兜底返回。fmt/Clippy、许可证、前端与嵌入 Release 通过；UI 与桌面结果详见本版验证。
- [草稿 PR #23](https://github.com/Leovikii/Resona/pull/23) 的 [635f2d9 双平台 CI](https://github.com/Leovikii/Resona/actions/runs/40870191090) 已通过：Windows Rust 144/12 条件忽略、Arch Rust 135/17 条件忽略，另有真实 FFmpeg 2 项和包元数据测试；后续提交仍以最新 SHA 门禁为准。Windows 原生和 UNC/共享目录、CUE/列表命名回归见 [Windows 交接](plans/0.3.0-beta.1-windows-validation.md)。CI 不等于实机通过，真实网络中断、物理听感与既有硬件/动画门禁仍保留。

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

- beta.1 已替换 Linux GTK chooser，并通过 KIO 目录边界恢复 NFS 递归导入与伴随文件发现。真实文件夹后端已通过，鼠标落点、文件夹 chooser、网络断线/权限失败与恢复仍需实机专项；不能将当前会话映射当作永久网络列表路径。
- 语言初始化已改用原生接口，避免本机 WebKit 报告 `C` 导致英文；用户偏好为 `system`，手动选择仍优先。需继续验证实际主/辅助窗口、原始 Dolphin 冷启动及 Windows 显示语言，不能仅凭进程环境宣布 UI 通过。
- 此前 Linux 实机为 CachyOS / KDE Plasma 6.8.1 / 原生 Wayland，双屏均为 150% 缩放。当前 Windows 主机不重跑 Linux 桌面测试；需要会话环境的自动化交由 Linux 平台 AI agent 按现有脚本执行，Arch 包安装事务由 disposable Actions 容器补足；未回填结果前保持待测。Polkit 授权、应用内跨版本升级与重启仍未验收。
- Linux 系统 `LocalFileSigLevel` 不变；Tauri `.tauri.sig` 不是 OpenPGP。无图形授权代理、包锁、冲突或严格本地签名政策时明确失败，程序不删锁、不关闭检查、不自动全系统升级。
- Windows NSIS 真实跨版本覆盖、取消、卸载及完整 SMTC/托盘/Mica/文件关联仍需安装版实机回归；本机测试和 CI 构建不能替代。安装包尚无 Authenticode，Tauri 包与清单签名保留。
- 多屏/DPI、物理媒体键、睡眠/蓝牙恢复等既有人工门禁保持；歌词跨输出拖动/热拔插恢复、托盘宿主热重启恢复未实现或未验收。Linux 转换目标文件系统需支持同卷硬链接；不支持则保留源文件并失败。
