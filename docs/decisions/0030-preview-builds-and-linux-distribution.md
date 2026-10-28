# ADR 0030：同版本预览构建、Arch 分发与更新归属

- 状态：部分 Superseded（[ADR 0031](0031-version-release-tags.md) 取代同版本发布/tag/更新比较/pkgrel，保留 Arch/AUR 与包管理器更新规则）；所有者已批准实施，由 main Actions 交付；本机产物仅测试，提交推送到 dev，main 由所有者 PR。
- 日期：2026-10-27
- 授权：所有者要求先提供当前版本的预览构建方案、保持版本号，并明确 Linux 只构建 Arch/AUR `-bin`，内置更新改为包管理器引导，不提供 AppImage 或 Linux 自更新。
- 关联：拟补充 [ADR 0024](0024-windows-tray-distribution-and-update.md)、[ADR 0028](0028-arch-linux-wayland-baseline.md)；补充并取代上述 ADR 中仅 Windows/版本变化发布与 Linux 手动预览的分发规则。

- alpha.3 修订：Linux 安装与更新条款由 [ADR 0032](0032-arch-in-app-update.md) 取代，移除 AUR，启用应用内下载验签与 Polkit/Pacman 安装；以下保留历史决定。
- alpha.4 修订：[ADR 0034](0034-main-only-ci-and-dependency-cache.md) 修订为 PR 到 main 时运行双平台 CI、必需 `PR validation` 成功后合并、main 只执行 CD；以下保留历史决定。

## 背景

当前 `0.3.0-alpha.1` 只提供本机 Arch / Wayland 测试包。Linux 更新 capability 关闭；main 发布只生成 Windows NSIS，版本 tag 是 `v<version>`，无法重复交付同一应用版本的不同预览构建。

AUR 安装由 pacman 管理文件。Linux 沿用这一更新归属即可满足当前 Arch 范围，无需新增 Tauri 安装路径。应用版本不变时，Windows 自更新仍需显式构建比较，Arch 包则使用包修订号，不能把 SHA 或 SemVer `+metadata` 当成更高应用版本。

## 决定

1. 保持项目与程序版本 `0.3.0-alpha.1`；另以工作流运行序号、重试序号和完整 commit SHA 记录构建身份。预览使用唯一 tag 与不可覆盖的资产；关于页同时显示应用版本和构建身份。
2. Windows 继续使用 NSIS 与 Tauri 签名更新。Linux 只提供 Arch 预编译载荷、原生 `.pkg.tar.zst` 与 AUR 配方，暂定 `resona-preview-bin`；依赖使用 Arch 系统库，不构建 AppImage。
3. Linux 的应用内更新入口改为“由包管理器更新”说明，提供 yay/paru 完整更新命令复制、AUR 页面和手动包下载入口。应用不执行命令、不提权、不轮询更新，也不自写 `/usr`；后端仍拒绝 Linux 安装更新请求。
4. Windows 应用版本按 SemVer 优先级比较；同应用版本的 opt-in 预览再比较构建二元组。Windows 更新清单额外签名以绑定构建身份，安装包继续由 Tauri 验签；安装请求固定 tag、构建、平台和资产。Linux 同版本更新只递增 Arch `pkgrel`，不接入 Tauri 清单或比较器。
5. 保留受保护 main 合并发布入口与唯一 `PR validation` 门禁。预发行应用版本允许 main 的交付提交生成新预览构建；稳定版继续版本递增与唯一 tag 规则，纯文档改动跳过发包。
6. Windows 和 Arch 的同一 SHA 构建完成后，由单个发布任务汇总清单并公开完整 prerelease；AUR 在公开资产可取后同步。AUR 同步失败单独重试，不能覆盖 Release 或回退较新配方。
7. 本轮仍只支持 Arch x86_64 / Wayland。Windows 稳定用户默认不接收预览，Arch 的预览/稳定由所安装的 AUR 包决定；不改变现有稳定下载入口。

## 取舍

- Linux 复用系统依赖和包管理器的升级/卸载流程，减少打包、依赖隔离与自更新维护面；滚动发行的库 ABI 变化仍需干净 Arch 环境检查和包修订重建。
- 独立构建身份供两平台追溯；额外清单签名与同版本比较仅服务 Windows 更新，不升级应用版本，不放宽 Tauri 的安装包签名校验。
- 首个具备构建识别能力的 Windows 预览包需手动安装；本机 Linux 测试包首次安装和隔离数据迁移也需说明，不能被当作已自动支持。
- 预览与稳定仍共用应用标识，安装会替换同平台既有版本；本提案不增加可并行运行的另一套数据库或播放器。

详细资产、触发条件、包修订格式、失败处理、实施顺序和验收要求统一维护于[预览交付方案](../plans/0.3.0-preview-delivery.md)。实现与本机验证已落盘；GitHub Actions 实际发布、AUR 凭据接入与 Windows 原生覆盖更新仍是发行门禁。
