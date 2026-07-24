# 开发规范

## 环境

- Windows 10/11 x64，Node 与 Rust 版本以锁文件和 `rust-version` 为准。
- 首次安装使用 `npm ci`。Rust Windows 命令在已加载 MSVC 环境的终端运行。
- FFmpeg 不进入安装包。普通开发和 release build 不下载它；只有真实转换矩阵需要 `npm run prepare:test-tools`。
- 大型 Mantine 参考只放在忽略的 `.local-docs/llms-full.txt`，按 API 名称局部检索。

常用命令：

```powershell
npm run dev
npm test
npm run build
npm run licenses
npm run lint:workflow
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run release:windows
```

真实转换矩阵：

```powershell
npm run prepare:test-tools
cargo test --manifest-path src-tauri/Cargo.toml pinned_ffmpeg_test_tools_preserve_matrix_and_quantize_32_bit_to_24 -- --ignored
```

固定 FFmpeg 版本、URL、归档/二进制哈希和许可证必须同步审查。测试工具只写入忽略的 `src-tauri/binaries/`；运行时依赖写入 Local AppData 并由 `FfmpegDependencyService` 验证。

## 实现规则

- 先沿用现有 module/service/adapter；只有消除真实重复或隔离平台/安全边界时才新增抽象。
- Rust 是领域状态权威。前端不推演播放、列表、更新、依赖或压缩终态；Tauri command 不承载业务状态机。
- 跨层 DTO 必须类型化并使用稳定 camelCase 序列化。不要暴露 Rodio、CPAL、数据库行或 Win32 类型。
- Mantine 用于组件行为和可访问性，应用 token/CSS 负责布局与材质。不得加入第二套组件库、全局 Acrylic/blur 或自绘标题栏。
- 平台能力放入 `platform` adapter。Win32/COM 回调只读缓存或发 typed command，不能做文件 I/O、图片解码、等待播放锁或直接控制 Rodio。
- 不增加第二播放引擎、mpv fallback、DSP、隐式重采样或未经测量的高频轮询。

## 文件与数据安全

- 数据库 schema 变更必须有从仍可能存在的旧版本迁移测试；事务失败不得留下半迁移状态。
- 批量文件写入先验证目标、写唯一临时文件、校验结果并原子提交；默认不覆盖。源文件删除只能发生在成功提交之后。
- 取消必须终止所有活动子进程并清理未提交临时文件；成功项不因其他项失败而回滚。
- 递归扫描规范化和去重路径，不跟随符号链接、junction 或 reparse point；单项错误结构化报告。
- 更新日志 Markdown 禁止原始 HTML；外链只允许 HTTP(S)。更新下载与安装交给 Tauri updater 验签。
- 卸载、迁移和清理只能处理应用拥有的目录，永不删除应用目录外的用户音频。

## 性能

- 先测量再优化；记录场景、构建模式和指标，避免用任务管理器瞬时读数代替基准。
- 播放 actor tick、原生投影和前端轮询保持当前有界频率；隐藏 WebView 暂停轮询并在恢复时同步。
- 元数据与封面按需读取并有大小/条目上限，不建立全局索引。
- 压缩扫描不得按文件启动外部进程；转换使用 1–4 个文件级 worker，不能无界创建 FFmpeg。
- 大目录树增量展开并使用 `content-visibility`；新增虚拟化依赖前必须证明现有上限不足。

## 测试层次

1. 纯函数/状态机：顺序、迁移、解析、SemVer、偏好、转换安全矩阵。
2. 服务集成：actor、SQLite、扫描、并行转换、取消、原子提交、更新选择。
3. 浏览器视觉：常用桌面与窄窗口、明暗主题、溢出、键盘焦点、更新 Markdown、压缩行进度。使用项目规定的浏览器客户端，不用浏览器结果宣称原生材质通过。
4. Windows Release：SMTC、任务栏缩略图/进度/按钮、托盘、Mica、文件关联、拖入、DPI/多显示器、桌面歌词、睡眠/蓝牙恢复和覆盖更新。

UI 固定格式控件要有稳定尺寸，文本不得溢出或遮挡。压缩窗口最低覆盖 `720x500`，主窗口至少覆盖 `1080x700`、`760x520`、`420x720`、`360x600`。

## 依赖准入

新增/升级依赖前确认：

- 许可证与 GPL-3.0-only 兼容，并更新 `docs/THIRD_PARTY_LICENSES.md`
- 是否增加 sidecar、运行时下载、联网、遥测或新的原生权限
- 锁定版本、维护状态、安装/产物体积和离线失败行为
- 现有标准库、Tauri、Rodio、Lofty、Hound、FFmpeg 或 Mantine 是否已满足需求

大版本升级必须独立处理，不与功能修复混在同一改动中。

## 完成定义

- 行为与 [PRODUCT.md](PRODUCT.md)、[ARCHITECTURE.md](ARCHITECTURE.md) 和相关 ADR 一致
- 自动测试、build、format、Clippy、许可证和差异检查通过
- 涉及 UI 时完成规定视口与主题视觉验证；涉及原生能力时明确记录仍需人工观察的项目
- 没有未清理临时文件、调试日志、测试密钥或构建产物
- `STATUS.md` 只更新当前事实；长期约束写入 ADR，活动范围写入计划
