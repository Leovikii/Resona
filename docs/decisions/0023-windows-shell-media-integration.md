# ADR 0023：Windows Shell 媒体与任务栏集成

- 状态：Accepted
- 日期：2026-07-23

## 背景

ADR 0007 已将 Windows SMTC 隔离在 `platform/media_session`，并使用 `souvlaki 0.8.3` 接收媒体键和发布基础播放状态。0.0.19 需要进一步提供完整系统媒体信息、任务栏缩略图播放按钮、任务栏播放状态和进度。

这些能力都消费同一份正在播放状态，但来自两套平台 API：SMTC 继续由 souvlaki 封装，任务栏缩略图与进度需要 Windows `ITaskbarList3`。若分别轮询 Rodio 或把状态经 WebView 转发，会增加重复状态、锁竞争和不可诊断故障。

## 提议

- 在 Windows platform 边界建立单一原生媒体投影，输入只包含框架无关的曲目元数据、播放状态、位置、时长和可用动作。
- `PlaybackService` 或现有播放 actor 通过可合并通道发布低频投影；SMTC、任务栏缩略图和托盘菜单消费该投影，不分别轮询，也不经过 WebView IPC。
- souvlaki 继续只存在于 SMTC adapter。补全标题、艺术家、专辑、时长、封面、播放状态、时间轴和可用控件；缺失元数据保持缺失，不由平台层猜测。
- SMTC 标题以 Rust 权威队列项的显示名为准。封面属于可选增强；封面读取或提交失败时必须重试无封面元数据，且只有成功提交后才能缓存本次元数据，避免品牌图失败连带丢失曲名。
- 任务栏通过 `windows` crate 调用 `ITaskbarList3`。缩略图工具栏固定注册“上一曲 / 播放或暂停 / 下一曲”三个槽位，运行时只更新图标、提示、enabled 和 hidden flags。
- 缩略图按钮资源必须保持透明背景，因为 Windows 会为按钮绘制自己的灰色交互表面。`ITaskbarList3` 只接受静态 `HICON`，没有主题变化通知或动态前景色；图形采用尽量填满图标画布的白色符号与细深灰外轮廓，在黑白缩略图背景下均保持可读，且不形成嵌套底板。
- 监听并处理 `TaskbarButtonCreated`，Explorer 重启或任务栏按钮重建后重新注册工具栏。按钮事件通过 typed command channel 进入播放服务，Win32 回调不得直接调用 Rodio 或等待播放锁。
- 任务栏进度只在确定时长且可定位时显示：播放使用 normal，暂停使用 paused，空闲/停止/未知时长清除；只有权威真实播放失败使用 error。
- Windows API 初始化或更新失败记录可诊断错误并按子能力降级，不阻止应用启动、音频播放或其他原生媒体能力。
- 不引入第二套媒体会话库。若 souvlaki 无法满足 SMTC 元数据或时间轴需求，再用新 ADR 评估以 `windows` crate 替换同一 adapter，而不是并行维护两个实现。

## 理由

- 单一投影避免 SMTC、任务栏和托盘各自维护不同的播放状态与定时器。
- 固定缩略图槽位符合 Windows 注册后不能增删或重排按钮的约束。
- typed channel 保持系统回调、音频 actor 和 UI 之间的故障隔离。
- 逐能力降级允许旧 Windows、Explorer 重启和局部 COM 失败时继续作为普通播放器使用。

## 后果与验证

- `windows` crate 需要补充 COM、Shell taskbar、图标和消息相关 feature；实施前审查实际增量、许可证和与当前 `windows 0.44` 的兼容性，不自动升级大版本。
- 主窗口 HWND 需要安全安装和移除消息处理；重复初始化、Explorer 重启和退出清理必须幂等。
- 自动测试覆盖平台无关投影映射、权威标题选择和元数据失败重试边界；真实 SMTC、媒体键、缩略图工具栏和任务栏进度必须在 Windows 10/11 实机验收。
- 本 ADR 接受后扩展 ADR 0007；不改变 Rodio 仍是唯一播放引擎的决定。
