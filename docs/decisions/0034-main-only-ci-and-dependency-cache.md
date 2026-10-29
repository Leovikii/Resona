# ADR 0034：PR CI、main CD 与依赖缓存

- 状态：Accepted；所有者于 2026-10-28 修订此前“仅 main CI”的要求，改为 PR 验证通过后合并、main 仅执行 CD。
- 修订 ADR 0030 的触发与缓存策略；每版本单一 tag、签名和双平台完整交付规则保留。

## 决定

1. `pr-validation.yml` 在指向 `main` 的 PR 创建、更新及重开时触发 Windows/Arch CI，检出 GitHub 的 PR 合并引用，验证与 main 合并后的代码。普通 dev push 不单独触发；已有 PR 的 dev 更新通过 PR synchronize 触发。同一 PR 的新运行取消过时 CI。
2. 两端执行既有测试、Clippy、构建与 Arch 真实包安装回归，不访问 release Environment 或签名密钥。固定名称 `PR validation` 以 `always()` 汇总两端结果，只有两端均 success 才通过；失败、取消和跳过均不能放行。
3. main 活动保护规则要求 GitHub Actions 的 `PR validation` 成功并跟上最新 main；保留必须通过 PR 合并、禁止强推与删除。不得跳过 CI 合并，也不伪造检查状态。合并前仍运行受影响的本机检查，实机和硬件未测项准确记录。
4. `main-merge-delivery.yml` 在 `push main` 时只执行 CD：检查版本/tag，构建该 main SHA 的 Windows NSIS 与 Arch 包，验证包安装和产物身份，统一验签、草稿上传并公开。发布依赖已受保护的 PR CI，不在 main 重复整套 CI；签名仅在 release Environment。双平台构建、身份、验签或包校验失败阻止发布。主分支交付串行排队，不取消正在发布的版本。
5. `setup-node` 按 `package-lock.json` 缓存 npm 下载，仍执行 `npm ci`，不缓存 `node_modules`。Rust 使用固定 SHA 的 `Swatinem/rust-cache` 缓存 Cargo 下载和依赖编译产物；不缓存项目自身产物、签名密钥、发布目录或用户数据。
6. Rust 缓存按 job、工具链、Cargo 清单/锁文件和编译环境隔离；Windows 加入 runner 镜像版本，Arch 加入安装后完整 Pacman 包版本摘要。缓存缺失时正常完整构建；命中不跳过测试、Clippy、验签或包安装回归。
7. 所有者于 2026-10-29 明确要求精简测试：自动验证使用可重复的测试/脚本和原生接口，不使用鼠标模拟等低效交互测试。当前主机和 CI 都无法覆盖的 Linux 桌面、视觉或硬件项目交由人工验收；保留真实未测/失败记录，不能以自动测试通过代替人工结论。具体执行见 [开发规范](../DEVELOPMENT.md#双平台精简执行)。

## 运维与迁移

- PR #20 合并时按先前授权移除了不会触发的旧检查；本次恢复与新 PR CI 对应的必需 `PR validation`，不添加合并后才出现的 CD 状态作为合并前条件。
- PR #21 已通过双平台 CI 后合并，PR CI/main CD 规则已生效。已有公开 tag 的同版本合并跳过发包；本次所有者明确授权删除 alpha.4 Release/tag 后重发的单次例外见 [ADR 0035](0035-unified-arch-package.md)。
- 不通过手动 tag、本机上传或虚假成功检查绕过发布流程。仓库规则与工作流的检查名称必须同步维护。
