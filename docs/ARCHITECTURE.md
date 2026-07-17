# 系统架构

## 原则

1. 成熟实现优先：播放使用 Rodio/CPAL/Symphonia，转码使用 FFmpeg，标签使用 lofty-rs。
2. Rust 后端是播放、媒体库、任务和持久状态的权威来源。
3. 音频库、外部进程、平台 API、数据库和 UI 框架均位于适配边界之外。
4. 边界用于控制变化和故障，不为了抽象本身增加层级。
5. 性能敏感路径以批量事件、虚拟列表和后台任务处理，避免高频跨 WebView IPC。

## 总体结构

```text
React windows
  main / desktop-lyrics
          |
   typed Tauri commands and events
          |
Rust application services
  playback / library / lyrics / metadata / conversion
          |
domain contracts and models
          |
adapters
  Rodio audio / SQLite / filesystem / FFmpeg / platform windows
```

依赖方向只能从外向内：UI 和 adapter 依赖应用契约，领域层不依赖 Tauri、Rodio、CPAL、Symphonia、SQLite、Mantine 或操作系统 API。

应用服务按当前产品需求和变化边界拆分：播放协调、队列、媒体库、元数据、转换、歌词和设置相互独立。首版使用内部 Rust 契约，不建立没有明确需求的动态插件 ABI 或皮肤引擎。

## 前端边界

### 状态归属

- Rust：播放状态、队列、媒体库、扫描任务、转换任务、持久设置。
- Zustand：窗口临时状态、选择状态、未提交表单和纯 UI 偏好。
- React component：生命周期很短且不跨组件共享的交互状态。

前端不得自行推断播放成功或长期维护一份与后端竞争的队列。command 返回接受/拒绝结果，后端事件提供权威状态。

### Mantine 解耦

Mantine 是 UI 实现依赖，不是领域 API。采用三层策略：

1. `app/theme` 保存 Resona 设计令牌和 Mantine provider 配置。
2. `shared/ui` 保存具有统一行为或领域语义的组件，例如 `TrackRow`、`PlayerControls`、`AppDialog`。
3. `features` 组合项目组件，不传播 Mantine 类型到状态、服务或 Tauri 契约。

不包装纯布局和一次性展示控件。简单包装不能隔离真实变化，反而增加组件深度和维护成本。需要包装的判断条件至少满足一项：

- 统一默认行为、键盘或无障碍规则
- 统一错误、加载或空状态
- 屏蔽第三方类型或 API
- 承载 Resona 领域语义
- 隔离平台差异或已知缺陷

这些函数组件不会进入音频处理路径；避免深层 provider 和无意义包装即可保持 UI 开销可忽略。

## 播放边界

应用层依赖 `PlaybackEngine` 契约，首个且唯一的首版 adapter 为 `RodioPlaybackEngine`。UI、应用服务和 Tauri command/event 契约不得接触 Rodio、CPAL 或 Symphonia 类型。

核心职责：

- `PlaybackService`：队列、播放模式、命令序列、状态归一化和恢复策略
- `AudioEngineActor`：在专用 Rust 线程持有输出流、播放器和混音器，通过 typed channel 串行处理命令
- `RodioPlaybackEngine`：实现领域播放契约，配置 Rodio 播放、队列、定位、音量和结束通知
- `AudioDeviceManager`：通过 CPAL 枚举输出端点、选择默认/指定设备并在设备变化后重建输出
- `DecoderFactory`：限制并配置 Symphonia 解码能力，首版强制 MP3/WAV/FLAC，AAC/M4A 可选
- `PlaybackClock`：生成权威播放位置并按 UI 实际需要节流

音频回调不获取应用全局锁，不执行文件、数据库或 WebView IPC。解码、设备恢复和队列准备在 Rust 后台完成；音频样本不经过 Tauri IPC 或 WebView。前端在后端时间锚点之间做视觉插值。

Rodio 使用精确锁定的稳定版本和最小 feature 集。首版不实现 mpv fallback，也不同时维护第二套播放路径。

## 平台能力

平台差异由能力接口表达，而不是假设所有平台都支持相同行为：

```text
AudioOutputCapabilities
DesktopLyricsCapabilities
GlobalShortcutCapabilities
```

Windows 首版通过 CPAL 实现 WASAPI Shared 和桌面歌词窗口。板载声卡与蓝牙均视为系统输出端点；设备断开、默认设备变化和休眠恢复触发输出流重建。未来 Linux adapter 可以报告 Wayland/PipeWire 实际能力并降级，通用 UI 根据 capability 显示可用功能。

## 数据与文件安全

- SQLite schema 通过版本化 migration 演进。
- 扫描、标签写入和转换不得阻塞 Tauri 主线程。
- 文件写入默认使用临时文件、验证后原子替换。
- 转换不得默认覆盖源文件，也不得把有损来源标记为无损提升。
- 跨边界错误使用稳定错误码和可诊断上下文；内部日志保留原始原因。

## 故障隔离

- 播放 actor 意外退出时由服务监督并进入可诊断失败状态；可恢复时重建输出，不静默继续。
- 单曲解码失败只跳过或停止当前曲目，不污染队列和媒体库。
- FFmpeg 异常退出只使对应转换任务失败，不应终止主应用。
- 单文件扫描失败不终止整个目录扫描。
- 单个损坏标签不污染其他媒体记录。
- 单个窗口渲染失败不改变播放服务状态。
- adapter 错误不得把第三方异常类型扩散到领域层和 UI。

## 播放依赖持续性

- Cargo.lock 是实际版本的权威记录；Rodio、CPAL、Symphonia 大版本不自动升级。
- 用固定音频 fixture 验证版本升级，覆盖格式、位深、采样率、Gapless、定位和坏文件。
- 上游停止维护本身不触发迁移；只有出现无法修补的阻塞缺陷或平台不兼容时才评估替换。
- 替换顺序为：固定/修补现有版本、项目 fork、直接 CPAL + Symphonia、GStreamer adapter、mpv adapter。
- 任何替换都通过新的 ADR 决定，产品中不预装未使用的备用引擎。
