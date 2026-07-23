# ADR 0025：公开仓库首页与 Agent 规范归位

- 状态：Accepted
- 日期：2026-07-23

## 背景

根目录 `README.md` 当前混合产品介绍、技术栈、开发命令、文档导航和开发状态，更像开发台账而不是用户首次看到的项目首页。根目录 `AGENTS.md` 又包含完整开发规范；同时仓库中存在只为 AI 辅助开发保存的 4.09 MB Mantine 全量文档快照和已经长期失真的 `CHANGELOG.md`。全量组件参考对本机 Agent 开发仍有价值，但不应增加公开仓库体积。

0.1.0-rc.1 需要在公开发布前清理仓库表面，但不能为了“看起来干净”删除测试 fixture、许可证、可重复构建脚本或 Agent 发现入口。

## 决定

### 用户 README

- 根 `README.md` 改为简洁英文用户首页，并新增同结构的 `README.zh-CN.md`；主 README 顶部直接链接中文版本。
- 两份 README 使用 `assets/resona-gothic-wordmark.svg` 或等价 Resona 品牌资源作为标题，保留可访问的 `alt` 文本。
- 内容只保留一句定位、核心特色、支持平台/格式、下载入口、语言切换、许可证和一个开发文档入口。技术栈、内部版本状态、Agent 阅读顺序、完整开发命令和历史验收不再放在用户 README。
- 中英文 README 必须结构对应、信息一致，避免维护两套不同承诺；不加入徽章墙、营销套话、长截图墙或未验证能力。

### Agent 与开发文档

- 完整 Agent 规则迁入 `docs/AGENT_GUIDE.md`，并由 `docs/README.md` 索引。
- 根目录必须保留极简 `AGENTS.md` 作为 Agent 自动发现入口，只负责要求完整读取 `docs/AGENT_GUIDE.md` 以及任务相关核心文档；不能直接删除根入口。
- 详细开发流程、架构、计划、状态、发行记录和第三方说明继续由 `docs/` 维护。源目录旁只保留防止误打包、误提交或说明资源生成方式所必需的短 README。

### 仓库清理

- 删除文件前必须证明其不参与构建、测试、分发、许可证合规或已接受文档链接；先列清单和引用，再删除并执行干净检出验证。
- 从 Git 跟踪中移除仅供 AI 本地搜索、无构建引用的 `docs/vendor/mantine/llms-full.txt` 及其专属说明，但把完整快照保留为 `.local-docs/llms-full.txt` 并通过 `.gitignore` 排除。`docs/AGENT_GUIDE.md` 明确本机检索入口；文件缺失时再以锁定依赖的 TypeScript 类型和官方版本文档为准。
- 根 `CHANGELOG.md` 不再与 `docs/releases/` 和 GitHub Releases 维护第三套版本事实。实施时删除该过期文件，公开变更记录由 GitHub Releases 提供，详细验收证据留在 `docs/releases/`。
- 音频 fixture、安装器品牌资源、许可证生成器、发布验证脚本和 Tauri schema 不因体积或“看起来像生成文件”直接删除；只有存在可重复生成命令且干净检出构建不需要时才能移除。
- `node_modules/`、`dist/`、`src-tauri/target/` 和本地 FFmpeg 测试工具属于已忽略的本机产物，不是仓库提交清理对象；清理流程只确认它们不会进入 Git 或发布包。

## 理由

- 用户首页应该快速回答“这是什么、能做什么、去哪里下载”，内部实现细节在 `docs/` 更易维护。
- 保留极简根 `AGENTS.md` 同时满足自动发现和文档集中，不会因移动规则导致后续 Agent 不读取规范。
- 先做引用与可重复性审计可以避免误删音频回归样本、生成器或许可证据；把大型组件参考改为本地忽略资料，兼顾 Agent 开发效率和公开仓库体积。

## 后果

- 0.1.0-rc.1 实现阶段新增 `README.zh-CN.md` 和 `docs/AGENT_GUIDE.md`，重写根 README/AGENTS，并同步所有相对链接。
- 仓库清理后必须执行 Markdown 链接检查、许可证生成、前后端完整测试、Windows 分发审计和从干净检出的 Release 构建；本机 Agent 开始组件开发前检查 `.local-docs/llms-full.txt` 是否存在。
- 后续面向用户的特性说明进入两份 README 和 GitHub Release；内部事实只进入 `docs/`，不再把 README 当状态台账。
