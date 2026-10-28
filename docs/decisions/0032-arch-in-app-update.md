# ADR 0032：GitHub Arch 原生包与应用内更新

- 状态：Accepted
- 日期：2026-10-28
- 授权：所有者批准 alpha.3 方案，要求实现、双平台验证并通过 dev → main PR 交付。
- 修订：ADR 0028/0030/0031 中的 AUR、Linux 不检查/不提权/不安装条款。每版本单一 tag、签名与 main Actions 发布规则继续有效。

## 决定

1. Linux 范围保持 Arch 系 x86_64 / KDE Wayland。首次从具体 GitHub Release 下载 `.pkg.tar.zst`，核对 SHA256SUMS 后执行本地 `pacman -U`；移除 AUR 同步、配方分发和 yay/paru 引导。系统库、文件归属、升级与卸载仍由 Pacman 管理。
2. Linux 与 Windows 共用 Release 发现、严格更高 SemVer、预览偏好、签名清单和 Tauri `download()` 验签。Linux 平台资产为 `linux-x86_64`。下载由用户触发；启动只检查，不静默下载。Windows NSIS 和界面设计保持现有实现。
3. 只有从 `/usr/bin/resona` 运行、Pacman 管理且版本匹配的官方包可应用内安装。后端独占权限受限的临时目录，核对包名、版本、架构、内容摘要及只读事务；用户看到包变更后确认。安装前重查，预览最长保留 30 分钟。
4. 固定调用 `pkexec --disable-internal-agent /usr/bin/pacman -U --noconfirm -- <package>`，不用 shell、免密规则或特权常驻服务。安装期间阻止退出和新文件任务，有活动扫描/转换则拒绝安装。仅下载允许取消，不强杀 Pacman，不删除系统锁，不放宽依赖/签名策略；错误保留实际诊断。
5. 当前只支持同包名升级；`resona-preview-bin` 与 `resona-bin` 间切换需手动安装。预览事务出现警告/冲突或环境变化则失败，最终锁与依赖/文件检查仍由 Pacman 执行。成功后复核包数据库，再提示用户重启；不新增重启 helper。
6. 内部 PKGBUILD 封装同 SHA 的编译产物，依赖包含 Polkit。发布 job 用现有 Tauri 密钥签 Arch 包，独立签名命名 `.tauri.sig`，避免被 Pacman 当成 OpenPGP `.sig`。Windows、Linux 与清单均验签后，经完整草稿统一公开。保留源码和许可证资产。
7. alpha.1/alpha.2 Release 及资产按授权删除，tag 与历史保留；旧用户首次手动安装 alpha.3，保留用户数据。公开 alpha.3 后不可覆盖。

## 验证边界

Windows 本机覆盖 Rust、前端和原生窗口；Arch CI 覆盖编译、Clippy、真实 Pacman 安装/升级/重装/卸载及桌面注册。容器没有 KDE/Polkit 会话，不能替代图形授权、真实跨版本应用内更新与重启实机验收。系统要求本地 OpenPGP 签名时，Tauri 签名不能替代该策略，应用明确失败。
