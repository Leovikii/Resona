# ADR 0024：Windows 托盘生命周期、分发与更新基线

- 状态：Accepted
- 日期：2026-07-23

## 背景

ADR 0013 当前规定关闭主窗口即退出，防止桌面歌词和播放核心在无入口时残留。0.0.19 将加入可发现的系统托盘，因此可以支持明确的后台播放，但必须重新定义关闭、退出、第二实例、辅助窗口和转换任务的唯一生命周期。

同一阶段还要建立安装器、文件关联和 AppUserModelID。更新器在 0.1.0-rc.1 实现，但安装渠道、包格式、应用身份和签名边界必须在安装器落地前冻结，否则升级兼容和 Shell 身份会反复变化。

## 决定

### 托盘与退出

- 默认关闭行为为询问用户“退出应用 / 隐藏到托盘并继续播放”。确认框提供“不再询问”；勾选后将本次选择写入 Rust 偏好。设置提供“每次询问 / 隐藏到托盘 / 退出应用”三个选项，并可随时恢复询问。该偏好由 Rust 持久化；已保存的直接退出或隐藏选择在 WebView 不可用时仍然生效。
- 最小化保持普通最小化和任务栏按钮，不最小化到托盘。
- 托盘左键释放显示、还原并聚焦主窗口；右键原生菜单保持紧凑，只提供打开应用、截断后的只读曲目标题、上一曲、播放/暂停、下一曲、桌面歌词和退出。原生菜单不支持固定像素宽度，曲目标题最多保留 10 个 Unicode 字符再加省略号，以限制最长菜单项；检查更新只位于设置“关于”，不重复占用托盘菜单。
- “退出 Resona”是唯一完整退出操作。它统一保存状态、停止播放、关闭辅助窗口、协调转换、移除托盘并退出；不能依赖最后一个窗口关闭推断生命周期。
- 活动扫描或转换导致退出需要确认时，显示主窗口并交给应用 UI 确认，不在原生托盘回调中复制文件业务逻辑。
- 第二实例和文件关联打开媒体时，无论主窗口当前隐藏、最小化还是可见，都显示并聚焦主窗口，然后进入既有 `open_media_context`。

### 分发

- 0.1.0 只维护经过品牌化的一键式 Tauri 官方 NSIS `currentUser` x64 安装器；不同时维护 MSI。默认安装位置采用 Windows 当前用户本地应用目录，不要求管理员权限。
- 使用稳定标识 `io.github.vki.resona` 统一进程、窗口、快捷方式、任务栏分组、文件关联和 SMTC 身份。
- 注册 MP3/WAV/FLAC 关联并使用已准备的格式图标。覆盖升级保留应用数据；卸载清理安装目录、注册表、WebView 缓存、数据库、偏好和按需下载的依赖，但绝不删除应用目录以外的用户音频。
- FFmpeg/ffprobe 不进入安装包。音频压缩入口先检查固定版本依赖，只有用户明确操作才从固定 GitHub Release HTTPS 资源下载，并在 SHA-256 校验和安全解压成功后原子启用；缺失、下载中或校验失败时不得启动压缩任务。FFmpeg.org 只正式发布源代码，因此 Windows 二进制使用其下载页列出的构建提供方所维护的 GitHub Release，不使用第三方站点直链。
- Windows 发行物名称包含产品、版本、平台、架构和用途，例如 `Resona_0.0.19_windows_x64-setup.exe`。未来 Linux 发行物沿用同一平台显式命名规则。
- 标准安装器面向 Windows 10/11 并使用 Evergreen WebView2/官方 bootstrapper 回退；只有明确完全离线安装需求出现后才增加离线变体。
- 公开发行前需要可信 Windows Authenticode 方案；内部测试可使用明确标注的未签名安装器，自签名证书不冒充正式发布身份。

### 更新

- 首版更新由用户主动检查，不启动即联网、不强制更新、不静默下载。
- 0.1.0-rc.1 使用 Tauri 官方 updater 与 GitHub Releases 静态清单，更新对象为已安装 NSIS 渠道。Tauri bundle 开启 `createUpdaterArtifacts`，继续只有一个 NSIS 安装渠道；`.sig` 和更新 JSON 是该安装包的更新元数据，不是第二个替换程序。
- 设置“关于”提供“接收预览版更新”开关。更新通道以 Rust 类型化偏好保存，默认由当前 SemVer 决定：包含 prerelease 段的版本首次进入预览通道，普通稳定版本首次进入稳定通道，之后尊重用户显式选择。
- 稳定与预览检查都调用 GitHub 原生 Releases REST API，不维护 `update-preview`、通道指针或自建更新服务。Rust 跳过 draft 和标记不一致的 Release；稳定设置排除所有 prerelease，预览设置同时接受 prerelease 与稳定版，再按完整 SemVer 选择最高且高于本地的版本。每个不可变版本 Release 自带该版本的 `latest.json`、安装包和 `.sig`；选定 Release 后才把它的清单交给 Tauri updater 验签安装。
- GitHub Release 是否为 prerelease 只由三个版本文件一致的 SemVer prerelease 段决定。`alpha`、`beta`、`rc` 以及其他合法 prerelease 标识统一进入预览发布，普通版本进入稳定发布；不能用 PR 标签、分支名或手工勾选覆盖版本事实。完整 SemVer 顺序确保 `0.1.0-alpha < 0.1.0-beta < 0.1.0-rc < 0.1.0`，预览用户也能发现更高的正式版。
- Updater 签名密钥与 Windows Authenticode 证书分离。Updater 私钥不进入仓库、安装包或日志；公钥进入应用，私钥保管、备份、轮换和发布注入在 0.1.0-rc.1 记录。
- 网络、清单、下载或签名失败只影响更新操作，不影响本地播放器。

### CI/CD

- GitHub Actions 在单一 `main-merge-delivery.yml` 中分为合并前验证和合并后交付两条边界。目标为 `main` 的 PR 在创建、更新、重新打开和转为 ready 时，以只读权限运行稳定 job `PR validation`，验证 GitHub 生成的 PR merge commit；该 job 必须配置为 `main` ruleset 的 required status check，并要求分支在合并前与 `main` 保持最新。
- 同一 workflow 在 `pull_request` `closed` 且 `github.event.pull_request.merged == true` 时才运行 `Post-merge verification` 与 release job；不增加 `push`、tag push 或 `workflow_dispatch` 发布入口。仓库规则必须禁止直接推送 `main`，使通过 `PR validation` 的版本 PR 合并成为唯一发布意图。合并前 job 不读取 Environment、不接触签名密钥、不创建 tag 或 Release。
- 工作流检出该 PR 的合并提交，先以只读权限执行版本一致性、构建、测试、许可证和分发审计。只有版本合法、三个版本文件完全一致且 `v<version>` 尚不存在时，发布 job 才取得 `contents: write`，创建版本 tag 和 GitHub Release。
- 合并未改变版本时通常只运行 CI；唯一例外是当前版本从未创建过 `v<version>` tag，此时允许首个合并后的发布任务为该版本建立初始 Release。tag 建立后，同版本后续合并必须跳过 Release。版本回退、重复 tag、签名缺失、清单不完整或产物校验失败时不得发布部分产物。
- 所有第三方 Action 固定完整 commit SHA；同一仓库使用串行 release concurrency。updater 私钥和未来 Authenticode 凭据只通过受保护的 GitHub Environment/Secrets 注入。
- prerelease 版本创建 GitHub 原生 prerelease，稳定版本创建普通 Release；不刷新额外通道清单。发布产物、版本说明、许可证通知、SHA-256、更新 URL 和签名必须来自同一次构建。

## 理由

- 首次明确询问可以避免托盘引入后静默改变既有退出习惯；保存的固定偏好比“播放中隐藏、空闲时退出”更可预测，也避免同一关闭按钮按运行状态产生不同语义。
- 保留普通最小化符合 Windows 用户预期，关闭到托盘则由托盘图标提供明确恢复入口。
- 单一 NSIS 渠道减少安装、升级、关联、签名和更新回归矩阵。
- 用户主动更新符合本地离线播放器范围，并把联网行为保持为明确动作。
- 提前分离两种签名可以避免把 updater 完整性签名误当成 Windows 发布者身份。
- 按 SemVer 自动分流让版本号成为单一发布事实；稳定通道不会因 GitHub prerelease 更新而意外收到候选版，预览用户仍能自然升级到更高的正式版。
- 合并前 required check 阻止未通过构建、测试、格式、Clippy、许可证和版本门禁的提交进入 `main`；只在 PR 合并后运行发布链则避免 direct push、手工 tag 和手工工作流形成发布旁路。版本不变时只做 CI，避免每次合并都产生无意义 Release。

## 后果

- 本 ADR 接受后将 ADR 0013 的“关闭主窗口即退出”标记为被本 ADR 修订；ADR 0013 其余桌面歌词资源和控件决定继续有效。
- Rust 需要应用生命周期协调服务和原生持久偏好；Tauri run event、托盘回调和第二实例 handler 只调用该边界。
- Tauri 启用 `tray-icon` feature 和 NSIS bundle；新增依赖或 feature 前继续执行许可证和体积审查。
- 0.0.19 负责安装器和更新设计，0.1.0-rc.1 负责 updater 实现、托管和更新回归。
- 关闭到托盘、活动转换确认、升级覆盖、卸载清理和无残留进程必须在真实安装版 Windows 环境验收。

## 依据

- [Tauri Updater：静态 JSON、签名和 `createUpdaterArtifacts`](https://v2.tauri.app/plugin/updater/)
- [Tauri GitHub Actions 发布流程](https://v2.tauri.app/distribute/pipelines/github/)
- [GitHub Actions：仅在 PR 合并后运行](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#running-your-pull_request-workflow-when-a-pull-request-merges)
- [GitHub Rulesets：Required status checks](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/available-rules-for-rulesets#require-status-checks-to-pass-before-merging)
- [GitHub Releases REST API：列出版本与原生 `prerelease` 字段](https://docs.github.com/en/rest/releases/releases#list-releases)
