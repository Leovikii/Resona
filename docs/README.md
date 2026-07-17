# Resona 文档索引

本目录是项目范围、架构、进度和长期决策的权威记录。README 只保留入口信息，具体内容在这里维护。

## 核心文档

| 文档 | 用途 | 更新时机 |
| --- | --- | --- |
| [PRODUCT.md](PRODUCT.md) | 产品目标、首版范围和非目标 | 功能范围变化时 |
| [ARCHITECTURE.md](ARCHITECTURE.md) | 系统分层、边界和关键数据流 | 模块职责或通信方式变化时 |
| [STRUCTURE.md](STRUCTURE.md) | 计划目录及所有权 | 新增顶层模块或调整边界时 |
| [DEVELOPMENT.md](DEVELOPMENT.md) | 开发、依赖和质量准则 | 工程规则变化时 |
| [ROADMAP.md](ROADMAP.md) | 阶段、验收条件和顺序 | 计划调整时 |
| [STATUS.md](STATUS.md) | 已完成、进行中、下一步和阻塞项 | 每次有效开发后 |
| [releases/](releases/) | 各版本范围、验收证据和已知限制 | 每次版本验收时 |
| [plans/](plans/README.md) | 待确认版本计划及其状态 | 进入新版本前或范围变化时 |
| [decisions/](decisions/README.md) | 架构决策记录（ADR） | 形成难以从代码看出的长期决定时 |
| [vendor/](vendor/README.md) | 外部开发参考快照及来源 | 更新外部参考时 |

## 权威性

发生冲突时，优先级如下：

1. 当前代码、自动化测试和实际构建结果
2. 已接受的 ADR
3. `ARCHITECTURE.md` 与 `DEVELOPMENT.md`
4. `STATUS.md` 与 `ROADMAP.md`
5. 其他说明文档

发现文档与代码不一致时，应在同一任务中修正文档，不能让台账长期失真。
