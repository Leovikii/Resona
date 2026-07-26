# 目录与所有权

## 当前结构

```text
Resona/
├─ src/
│  ├─ app/                         # 窗口入口、壳、路由、跨 feature 编排
│  │  ├─ App.tsx
│  │  ├─ AudioCompressionApp.tsx
│  │  ├─ ApplicationUpdateDialog.tsx
│  │  ├─ DesktopLyricsApp.tsx
│  │  └─ preferences.tsx
│  ├─ features/
│  │  ├─ playback/                 # 播放快照与控制意图
│  │  ├─ playlists/                # 用户/默认列表工作流
│  │  ├─ library/                  # 列表读取与操作编排
│  │  ├─ lyrics/                   # 歌词和桌面歌词状态
│  │  ├─ compression/              # 扫描、转换、依赖快照 hooks
│  │  ├─ metadata/                 # 当前曲目详情
│  │  ├─ settings/                 # 偏好展示
│  │  └─ update/                   # 更新协调 hook
│  ├─ shared/
│  │  ├─ bridge/                   # Tauri、对话框、窗口外观
│  │  ├─ model/                    # 跨层 TypeScript DTO
│  │  ├─ i18n/                     # 中英文资源
│  │  └─ ui/                       # 无业务共享组件
│  ├─ assets/                      # 品牌与占位资源
│  └─ styles.css                   # 全局 token 与窗口布局
├─ src-tauri/
│  ├─ src/
│  │  ├─ lib.rs                    # 服务装配、commands 注册、生命周期
│  │  ├─ commands.rs               # 薄 IPC adapter
│  │  ├─ playback.rs               # actor 与 Rodio 引擎
│  │  ├─ playlists.rs              # 用户/默认列表领域逻辑
│  │  ├─ persistence.rs            # SQLite schema 与事务
│  │  ├─ metadata.rs               # 按需元数据与有界封面缓存
│  │  ├─ lyrics.rs                 # 本地歌词解析
│  │  ├─ compression.rs            # 扫描、并行转换与任务快照
│  │  ├─ ffmpeg_dependency.rs      # 按需依赖下载与验证
│  │  ├─ application_update.rs     # Release 选择与 updater
│  │  ├─ application_lifecycle.rs  # 退出、托盘与辅助窗口协调
│  │  └─ platform/                 # Windows/非 Windows adapters
│  │     ├─ media_session/
│  │     ├─ playback_projection.rs
│  │     ├─ taskbar.rs
│  │     ├─ tray.rs
│  │     ├─ window_material.rs
│  │     └─ desktop_lyrics/
│  ├─ capabilities/                # Tauri 最小权限
│  ├─ icons/                       # 应用、任务栏与文件关联图标
│  ├─ resources/                   # 运行时占位资源
│  └─ tauri.conf.json
├─ scripts/                        # Node 构建/测试工具
├─ tests/                          # 前端与发布规则测试
├─ assets/                         # 品牌 SVG、图标规范与直接引用正式资源的预览页
├─ docs/                           # 当前规范、ADR 和活动计划
└─ .github/workflows/              # PR 门禁与 main 自动交付
```

实际文件以仓库为准；本页只维护职责边界，不列举每个测试或组件。

## 所有权规则

- `app` 可以组合多个 feature；feature 不能反向依赖 `app`。
- feature 通过 `shared/model` DTO 和 `shared/bridge` 与 Rust 通信，不直接使用 `@tauri-apps/api`，窗口入口的生命周期调用除外。
- Rust command 保持薄；状态机、事务、安全检查和后台任务属于相应 service/module。
- `platform` 只接收平台无关快照/命令，Win32/COM 类型不得泄漏到播放、列表、压缩或前端。
- 通用 UI 只有在至少两个真实调用点共享行为时才进入 `shared/ui`；不得为了目录对称创建空抽象。
- 测试靠近所有者模块；跨模块发布规则留在 `tests/` 或 `scripts/`。

## 文档规则

- `STATUS.md` 只记录当前版本、开放门禁和风险，不积累逐日流水。
- `plans/` 只保留活动 RC、下一个发行门禁及仍直接约束当前工作的计划。
- `decisions/` 保存仍有效或明确标记 Superseded 的长期决定；实现变化必须同步修订相关 ADR。
- 大型第三方参考只保存在忽略的 `.local-docs/`，不得提交。
- 许可证清单由脚本生成，不手工压缩或删除。

## 命名

- React 组件 `PascalCase.tsx`，hook `useXxx.ts`，普通模块 `camelCase.ts`。
- Rust 模块 `snake_case.rs`，类型 `PascalCase`，函数/字段 `snake_case`。
- 平台实现放在 `platform/<capability>/` 或带平台条件的 adapter 中，不在文件名散布 UI 术语。
