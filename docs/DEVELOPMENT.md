# 开发准则

## Windows 环境

0.0.1 使用以下本机工具链：

- Node.js 26 与 npm 11
- Rust 1.97.1（项目最低版本见 `src-tauri/Cargo.toml`）
- Visual Studio Build Tools 2022，包含 MSVC/C++ 桌面工具
- Microsoft Edge WebView2 Runtime

首次安装依赖使用 `npm ci`。Rust 命令应在已加载 MSVC 环境的 Developer PowerShell/Command Prompt 中运行。

常用验证命令：

```powershell
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run tauri build
```

需要真实默认音频设备的 smoke test 默认忽略，显式执行：

```powershell
cargo test --manifest-path src-tauri/Cargo.toml opens_default_output_and_accepts_a_wav -- --ignored
```

开发运行使用 `npm run tauri dev`。单独执行 `npm run dev` 只预览 Web 前端，没有 Tauri 文件对话框或 Rust 播放能力。

## 优先级

发生取舍时按以下顺序判断：

1. 数据与文件安全
2. 播放正确性和可诊断性
3. 用户体验完整性
4. 长期维护成本
5. 性能
6. 初始实现速度

“少写代码”指减少自有维护面，不意味着省略错误处理、测试或迁移。

## 依赖准入

允许商业生态，但免费核心必须满足：

- 不要求注册、申请许可证或注入密钥
- 不要求联网激活
- 不显示水印、遮罩或授权提示
- 不把许可证校验或强制遥测注入最终应用
- 免费能力足以承担选定职责
- 许可证与 GPL-3.0-only 项目分发兼容

引入核心依赖前记录许可证、维护活跃度、替代方案、二进制体积和退出成本。大版本升级不得自动合并，必须重新检查运行时行为和许可边界。

## 依赖使用

- 首先使用标准库、现有依赖或外部工具已提供的能力。
- 不同时引入两套完整 UI 组件库、状态库或数据库层。
- adapter 解决真实的变化点；禁止为“以后可能替换”预建空泛接口。
- FFmpeg sidecar 版本固定并记录来源、构建选项、许可证和校验值。
- Rodio、CPAL、Symphonia 使用稳定发布版和最小 feature 集；Cargo.lock 记录实际版本。
- 不因上游发布新版本自动升级音频栈，升级必须通过完整播放 fixture 回归。
- 外部参考快照放在 `docs/vendor`，必须带来源与获取日期。

## 性能预算原则

- 音频流不进入 WebView，也不跨 Tauri command/event 传输。
- 音频输出由专用 Rust actor 持有；音频回调不等待应用锁、不访问数据库、不发送 UI 事件。
- 长曲目列表必须虚拟化。
- 高频进度事件按视觉需要节流，避免每个音频 tick 触发全局 React 更新。
- 数据库扫描批量提交，封面和波形使用缓存。
- 性能优化以测量结果为依据，不通过牺牲边界和正确性换取不可见收益。

## 测试层次

- 纯领域逻辑：快速单元测试
- Rodio/CPAL/Symphonia、FFmpeg、SQLite adapter：固定 fixture 的集成测试
- Tauri command/event：契约测试
- 关键用户流程：桌面 E2E
- UI：暗色、缩放、空/加载/错误状态截图验证

音频 fixture 必须体积小、来源清晰并允许重新分发。

首版播放 fixture 至少覆盖：

- MP3、PCM WAV、FLAC
- 44.1 kHz/16-bit、48 kHz/24-bit、96 kHz/24-bit、192 kHz/24-bit
- 连续专辑、混合采样率队列、定位、暂停恢复和损坏文件
- 默认设备、指定设备、蓝牙断开和输出设备变化

AAC/M4A 只有在启用对应 feature 后才进入发布测试矩阵。

## 完成定义

一项工作只有同时满足以下条件才算完成：

- 行为符合验收条件
- 错误路径可诊断
- 相应测试或验证已执行
- 没有引入未说明的许可证或运行时验证机制
- `docs/STATUS.md` 已更新
- 长期架构决定已记录为 ADR
