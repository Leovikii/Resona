# 开发规范

## 环境

- Windows 10/11 x64；0.3.0-alpha.1 起增加 Arch 系 Linux x86_64 / Wayland 开发预览。Node 与 Rust 版本以锁文件和 `rust-version` 为准；[Linux 环境、运行和验证](LINUX.md)。
- 首次安装使用 `npm ci`。Rust Windows 命令在已加载 MSVC 环境的终端运行。
- FFmpeg 不进入安装包。Windows 真实转换矩阵使用 `npm run prepare:test-tools`；Linux 直接使用系统 `ffmpeg` 包，不运行 Windows 工具准备脚本。普通开发与 build 不下载 FFmpeg。
- 大型 Mantine 参考只放在忽略的 `.local-docs/llms-full.txt`，按 API 名称局部检索。

常用命令：

```powershell
npm run dev
npm test
npm run build
npm run build:exe
npm run licenses
npm run lint:workflow
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run release:windows
```

可直接运行的测试 EXE 只能用 `npm run build:exe` 生成；裸 `cargo build` 不启用 Tauri `custom-protocol`，产物仍会访问开发服务器，不能交付测试。

真实转换矩阵：

```powershell
npm run prepare:test-tools
cargo test --manifest-path src-tauri/Cargo.toml pinned_ffmpeg_test_tools_preserve_matrix_and_quantize_32_bit_to_24 -- --ignored
```

固定 FFmpeg 版本、URL、归档/二进制哈希和许可证必须同步审查。测试工具只写入忽略的 `src-tauri/binaries/`；运行时依赖写入 Local AppData 并由 `FfmpegDependencyService` 验证。

预览分发由 `.github/workflows/main-merge-delivery.yml` 的 main push 完成：验证后构建 Windows NSIS 和 Arch 包，统一验签、draft 上传并公开；dev push 不发布。本机产物只测试，留在忽略的 target，不提交/上传。每个版本只发布 `v<version>` 的一个 Release，已有 tag 时同版本合并跳过；发布说明维护在 `docs/releases/<version>.md`，使用极简英文 Markdown。Linux 原生包与双平台签名规则见 [ADR 0032](decisions/0032-arch-in-app-update.md)，不再同步 AUR。Arch PR CI 的 `scripts/test-arch-package.mjs` 仅允许在 disposable Actions 容器 root 环境运行，覆盖真实 Pacman 安装/升级/重装/卸载。

Linux 本机构建使用 `npm run build:linux`，开发与运行使用 `npm run dev:linux` / `npm run start:linux`；不将裸 `cargo build` 的开发服务器模式产物交付测试。KDE 桌面歌词使用系统 `gtk-layer-shell`，MPRIS 使用会话总线；原生自动回归可运行 `python3 scripts/test-linux-desktop.py`（需要 Release、gio/desktop-file-utils，必须先关闭其他实例；也可设置 `RESONA_TEST_BINARY` / `RESONA_TEST_DESKTOP` 检查解包后的真实安装布局）。Linux 的同目录输出采用不覆盖硬链接提交，失败保留源文件。

## 实现规则

- 先沿用现有 module/service/adapter；只有消除真实重复或隔离平台/安全边界时才新增抽象。
- Rust 是领域状态权威。前端不推演播放、列表、更新、依赖或压缩终态；Tauri command 不承载业务状态机。
- 跨层 DTO 必须类型化并使用稳定 camelCase 序列化。不要暴露 Rodio、CPAL、数据库行或 Win32 类型。
- Mantine 用于组件行为和可访问性，应用 token/CSS 负责布局与材质。不得加入第二套组件库、全局 Acrylic/blur 或自绘标题栏。
- 短时通知使用 Mantine `Notification` 及其原生图标插槽；文案按“结果：实际原因”组织，在无歧义前提下尽量简短。只有无需持续处理的临时反馈才自动消失，持续错误保留在对应工作流中。
- 平台能力放入 `platform` adapter。Win32/COM 回调只读缓存或发 typed command，不能做文件 I/O、图片解码、等待播放锁或直接控制 Rodio。
- 同步窗口查询可能等待 UI 线程；查询尺寸、位置或窗口状态时不得持有 UI 事件处理也会使用的 service mutex。先读取窗口快照，再加锁更新服务状态，避免布局切换与 resize/close 事件互相等待。
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
4. 原生平台：Arch Wayland 验收见 [Linux 指南](LINUX.md)；Windows Release：SMTC、任务栏缩略图/进度/按钮、托盘、Mica、文件关联、拖入、DPI/多显示器、桌面歌词、睡眠/蓝牙恢复和覆盖更新。

UI 固定格式控件要有稳定尺寸，文本不得溢出或遮挡。压缩窗口最低覆盖 `720x500`，主窗口至少覆盖 `1080x700`、`760x520`、`420x720`、`360x600`。

### 双平台精简执行

- PR 的 Windows job 执行完整 `npm test`（自动发现 `scripts/*.test.mjs`），共享前端纯逻辑不在 Linux job 重复。Linux 的 `npm run test:linux` 保留 POSIX 路径、桌面打包和交付规则；Rust 测试、Clippy 与编译两端各跑，因为平台 adapter 和条件编译不同。最终 PR 门禁仍要求两端成功。
- Linux 两项 FFmpeg 忽略测试合并为一次 `cargo test --manifest-path src-tauri/Cargo.toml --locked ffmpeg -- --ignored --test-threads=1`。不删文件安全、迁移、取消或真实转换矩阵。窗口切换压力检查用两次完整往返，保留尺寸记忆、并发 resize、最大化恢复和重建断言。
- 共享 UI 用一组代表性组合覆盖四个规定视口、浅/深色、solid/Mica 预览、压缩窗口和歌词常用/64px 字号，不做所有排列的笛卡尔积。检查溢出、控件可达、键盘焦点、歌词移入/移出及高于 hover 阈值的设置。共享布局不按操作系统重复截图；两端原生 Release 仍分别核查字体、透明背景、主题、窗口按钮/动画和锁定穿透。
- 原生回归并入现有生命周期测试：Linux 同一用例覆盖 helper 显隐/点击、输入区域和重建；Windows 普通测试直接向自有 HWND 发送 hover/leave/重显消息，断言绘制状态与 alpha。实际鼠标命中、跨进程穿透、DPI 与 Windows Mica 保留短实机清单。浏览器预览不能宣布原生视觉通过。
- Linux 会话测试串行运行、每次新建 XDG 数据/缓存目录并保留桌面配置；已有实例占用时先退出，避免误测用户会话。更改后只重跑受影响层和必要构建，最终以双平台 CI 汇总；没有新变化不重复完整矩阵。

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
