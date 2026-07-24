# ADR 0024：Windows 生命周期、分发与更新

- 状态：Accepted；rc.2 修订启动更新检查
- 日期：2026-07-23，修订于 2026-07-24

## 托盘与退出

- 关闭主窗口按 Rust 持久化偏好执行“每次询问 / 隐藏到托盘 / 退出”；最小化保持普通任务栏行为。
- 托盘左键恢复并聚焦主窗口；右键只提供打开、只读曲名、上一首、播放/暂停、下一首、桌面歌词和退出。
- “退出 Resona”是唯一完整退出操作，统一停止播放、辅助窗口、扫描/转换、托盘和后台服务。活动文件任务需先由应用 UI 确认。
- 第二实例或文件关联打开媒体时，始终恢复/聚焦主窗口后进入既有 external-open 服务。

## 分发与依赖

- 0.1.0 只发布 Tauri 官方 NSIS `currentUser` x64 安装包，稳定标识为 `io.github.vki.resona`；不并行维护 MSI。
- 注册 MP3/WAV/FLAC 文件关联与品牌图标。覆盖升级保留应用数据；卸载只清理应用拥有的安装、注册、缓存、数据库、偏好和下载依赖，绝不删除用户音频。
- FFmpeg/ffprobe 不进入安装包。用户明确下载依赖后，`FfmpegDependencyService` 从固定 GitHub Release 获取归档，验证归档和二进制 SHA-256、安全解压并原子启用。缺失、下载中或验证失败时不能开始转换。
- Updater 完整性签名与 Windows Authenticode 发布者签名是独立信任边界；私钥不进入仓库、安装包或日志。

## 更新

- 应用首个可交互帧后，每进程静默检查一次；同一服务使用 single-flight 和有界连接/总超时。检查失败不弹错误、不影响本地播放。
- 只有发现更高版本才打开懒加载更新弹窗。设置“关于”仍提供主动检查与“接收预览版”开关。
- 更新弹窗渲染 GitHub Release Markdown/GFM，跳过原始 HTML；HTTP(S) 外链经 Rust opener。用户可直接下载、验签、取消和安装。
- 禁止强制更新、后台静默下载、遥测、远程配置或自建更新服务。下载与安装继续完全由用户动作触发。
- 稳定通道排除 prerelease；预览通道接受更高 prerelease 或稳定版。完整 SemVer 与 GitHub `prerelease` 必须一致，draft/标记不一致的 Release 被忽略。
- Tauri updater 使用同一 Release 的 `latest.json`、NSIS 更新产物和 `.sig`；网络、清单、下载或签名失败只影响更新操作。

## CI/CD

- 单一 `main-merge-delivery.yml`：目标为 `main` 的 PR 运行只读 `PR validation`；受 ruleset 保护的 `main` 合并 push 运行轻量交付判定并按条件发布。
- `PR validation` 是唯一 required check，分支必须与 `main` 最新；合并前不读取 release Environment、签名秘密或写 Release。
- 版本文件一致、SemVer 高于既有 Release 且 `v<version>` 不存在时，main 交付自动由固定 SHA 的 Tauri 官方 Action 构建签名 NSIS、上传 updater 产物并创建 prerelease/stable Release。
- 不提供手工 tag、`workflow_dispatch` 或 GitHub CLI 旁路。第三方 Actions 固定完整 commit SHA；密钥只由限制为 `main` 的 `release` Environment 注入。

## 后果与验证

关闭到托盘、活动任务退出、文件关联、升级/卸载、启动检查、Markdown 日志、真实覆盖更新和签名失败必须在安装版验证。静默检查增加一次启动后网络请求，但不下载内容、不阻塞首帧，并保持本地播放器离线可用。
