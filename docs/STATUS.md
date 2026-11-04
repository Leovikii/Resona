# 开发状态

最后更新：2026-11-04

## 当前阶段：0.3.0 正式版验证

代码版本为 `0.3.0-rc.1`，基线 main `8ebbe56`。已完成[正式版计划](plans/0.3.0.md)的提示统一、快照/异步请求、转换与临时文件、列表协调、互斥、原子偏好和清理；审查补充输出恢复、侧文件读取上限和 Pacman 诊断。现有 [PR #26](https://github.com/Leovikii/Resona/pull/26) 为 dev→main 草稿。

## 验证事实

- Node 测试 79 通过、4 平台条件跳过、0 失败；输出/CUE 和播放/歌词旧轮询回归已确认旧实现失败、修复通过。前端 build、TypeScript unused 检查、39 个音频 fixture、三项 branding 检查通过；许可证重新生成无内容差异。
- Rust fmt 通过；本机缺少 MSVC，编译/tests/Clippy 由既有双平台 PR CI 执行。提交 `5459f40` 的 Windows/Arch 及汇总检查均成功：Windows Rust 154 通过、12 条件忽略；Arch Rust 149 通过、22 条件忽略，另外真实 FFmpeg 2 项及包元数据 1 项通过，Clippy、构建及包安装回归通过。侧文件/Pacman 最后一批回归仍待最新 CI，不将上一提交结果算作最终通过。
- 浏览器代表性矩阵完成：主窗口 1080×700、760×520、420×720、360×600，浅/深/系统主题、中英文、Mica 预览；压缩 720×500；歌词普通/64px。核查 focus/Escape、右边缘避让、菜单/音量弹层互斥和页面溢出。真实 hover/移入提示、超长路径、系统缩放/高对比度及原生字体/材质未验证。
- workflow 静态检查通过（最终提交前复核）；本轮 Linux 仅执行代码审查、可行测试和现有 PR validation，不新增独立 Linux CI。Linux 桌面/NFS/硬件未测。

## 交付门禁

最新合并引用双平台 PR validation 必须成功。`5459f40` 的 Windows Release EXE 与资源已下载并核对 Actions SHA-256，已交所有者原生/硬件验收或明确接受保留项，尚无验收结论。版本未升级，PR 未合并，未发布正式版；交付以 main CD 完整构建、包身份、验签和发布成功为准。开放限制见 [KNOWN_ISSUES](KNOWN_ISSUES.md)。
