# 开发状态

最后更新：2026-11-04

## 当前阶段：0.3.0 正式版实施与验证

代码版本为 `0.3.0-rc.1`，基线为 main `8ebbe56`。已接续[正式版计划](plans/0.3.0.md)：统一主题提示、修复输出/CUE 快照比较、FFmpeg 管道与临时文件、文件任务互斥、偏好原子保存，并将列表写入和激活协调移入服务。设置视图已按所有权拆分，确认的死组件/样式与无用参数已清理。

## 验证事实

- Node 测试 74 通过、4 平台条件跳过、0 失败；新增输出格式/CUE 回归先失败后通过。前端 build、TypeScript unused 检查通过。许可证重新生成无内容差异。
- Rust 最小工具链已安装在忽略目录，fmt 通过；本机缺少 MSVC，Rust 编译、tests 和 Clippy 由现有双平台 PR CI 验证，尚未记为通过。
- 浏览器代表性视觉与行为验收进行中；已核查深色底栏提示、字号/颜色、Escape 和音量弹层互斥。原生材质和硬件结果不由浏览器代替。
- 按所有者本次要求，Linux 仅做代码审查、可行测试及既有 PR validation，不增加独立 Linux CI 或要求 Linux 桌面专项。Windows 尚未完成本轮 Release 原生验收。

## 交付门禁

全仓审查和自动/视觉测试仍在继续；新增 Rust 回归尚待执行。无未处置 P1、最新双平台 PR validation 成功后才能合并。版本尚未升级至正式版，未记录发布完成。交付以 main CD 的完整构建、验签和发布结果为准。已知限制见 [KNOWN_ISSUES](KNOWN_ISSUES.md)，材质边界见 [ADR 0033](decisions/0033-frameless-titlebar-and-kde-material.md)。
