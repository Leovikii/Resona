# ADR 0031：每个应用版本只发布一次

- 状态：Accepted
- 日期：2026-10-27
- 授权：所有者要求恢复普通版本 tag 和单个 pre-release，删除两条构建编号预览 Release，保持应用版本不变，发布说明与提交使用极简英文。
- 替代：[ADR 0030](0030-preview-builds-and-linux-distribution.md) 中同版本重复发布、构建编号 tag、同版本更新比较及构建编号 pkgrel；保留其 Arch/AUR 分发、Linux 包管理器更新和双平台完整交付规则。

- alpha.3 修订：Linux 安装与更新条款由 [ADR 0032](0032-arch-in-app-update.md) 取代，移除 AUR，启用应用内下载验签与 Polkit/Pacman 安装；以下保留历史决定。

- alpha.4 修订：[ADR 0035](0035-unified-arch-package.md) 统一包名为 `resona`，不实现旧包名兼容；所有者明确授权本次删除 alpha.4 Release/tag 后同版本重发。以下分包条款保留为历史。

## 决定

- 所有版本统一使用 `v<version>` 与 `Resona <version>`；预发行使用 GitHub pre-release，每个应用版本仅一个 Release。已有版本 tag 时同版本合并不发包，下一次交付递增 SemVer。
- 当前 `0.3.0-alpha.1` 不升级；清理错误预览 Release/tag 后，由下一次 main 合并在 Actions 首次发布普通版本 tag。
- 构建序号仅用于诊断，不进入公开命名或更新比较。Windows 恢复严格更高 SemVer 与 Tauri 默认比较器，保留清单及安装包签名、安装请求绑定。
- 发行说明只读 `docs/releases/<version>.md` 的简短英文 Markdown，不自动生成 changelog。
- Arch 包修订固定从 `pkgrel=1` 开始，预览/稳定包名为 `resona-preview-bin` / `resona-bin`；Linux 只给包管理器更新引导。
- 双平台汇总验证后才公开；失败上传清理草稿，重试可替换未公开草稿，已公开版本不可覆盖或自动删除。
- 沿用唯一 main Actions 发布入口，不添加手动 tag、workflow_dispatch 或本机上传旁路。

## 后果

发布命名和更新规则与正常版本一致。修复已发布应用需要递增版本；旧构建编号客户端需要手动换装普通版本。当前仅通过 GitHub 分发，完整交付规则见 [ADR 0034](0034-main-only-ci-and-dependency-cache.md) 与 [ADR 0035](0035-unified-arch-package.md)。
