# ADR 0029：Wayland 置顶桌面歌词与 KDE 媒体集成

- 状态：Accepted
- 日期：2026-10-25
- 授权：所有者明确要求 Linux 桌面歌词必须实现且窗口必须置顶，并接入 KDE 媒体控制。
- 修订 ADR 0028 对桌面歌词和 MPRIS 的延期；Windows 的既有实现继续有效。

## 决定

- Arch / KDE Wayland 桌面歌词使用同进程 Tauri 透明 WebView，继续消费既有 Rust 播放与歌词状态。隐藏时关闭并释放窗口，重新开启时按需重建；前端完成偏好和首个快照后才显示。隐藏的 WebKitGTK 必须先读取一次初始快照，ready 握手不等待会被隐藏状态暂停的动画帧；持续轮询仍只在可见时运行。
- 使用 `gtk-layer-shell 0.8.2` 绑定系统 `gtk-layer-shell`，在隐藏窗口尚未 realized 时初始化 `zwlr_layer_shell_v1`。请求 `overlay` 层、`KeyboardMode::None` 和 exclusive zone `-1`，由 compositor 保持在普通窗口之上，包括全屏窗口；不预留工作区，不抢键盘，不循环调整 Z-order。锁屏和系统安全界面遵循 compositor 边界。
- `supported` 由实际协议探测决定，`layerShell` 标记手势路径；无 layer-shell 时返回明确失败，不用普通浮窗替代置顶保障。本轮支持范围为 KDE Wayland，其他 compositor 未验收。
- GTK 对象只在事件线程保存和操作；命令主线程派发采用有限等待，内部状态锁不跨窗口创建、显示、关闭和尺寸调用。普通窗口继续不查询全局坐标。
- 歌词在当前主窗口所在输出中使用逻辑尺寸、锚点与边距定位。拖动和左右调整宽度通过 surface-local 指针坐标更新边距/宽度，不调用 xdg-toplevel 的移动接口。保存独立的 Wayland 几何，恢复时约束到当前输出；跨输出拖动和热拔插恢复暂未实现。
- 锁定时设置 GTK 空输入区域实现穿透，先显示独立原生 GTK 解锁按钮再提交锁定。该 helper 不含 WebView，也使用 overlay 层和无键盘交互；解锁先恢复歌词输入再隐藏 helper。主窗口仍保留解锁入口。不使用全局输入 hook、输入注入、位置轮询或跨进程事件转发。
- Linux 使用 `mpris-server 0.9.0` / 现有 `zbus 5` 注册 `org.mpris.MediaPlayer2.io.github.vki.resona`，Windows 保留 souvlaki。未选择 souvlaki 0.8.3 的 Linux 后端，因为其固定 track ID、定位信号签名及消息处理节奏不满足本轮定位与状态同步要求。
- MPRIS 工作线程消费既有 500 ms 原生投影；回调仅排队命令，由线程调用权威播放 actor。提供标题、艺术家、专辑、封面、时长、位置、可用操作、音量、循环与随机状态，以及播放/暂停/停止、前后曲、相对定位、带 track ID 校验的绝对定位、Raise 和 Quit。Play/Pause 幂等，负数或过期绝对定位忽略；不声明网络 URI 打开能力。
- 封面采用规范化 PNG 指纹文件，正确转义 file URL，每次会话最多保留 16 个不同封面；启动清理旧缓存，退出尝试清理。不联网、无激活或遥测，不增加第二播放引擎。

## 依赖审查与后果

- GTK 绑定与 sys crate：MIT；原生库：LGPL-3.0-or-later，动态链接 Arch 系统包。MPRIS 库：MPL-2.0，与本项目 GPL-3.0-only 兼容；许可证清单由锁文件重新生成。
- GTK 3 的 Rust layer-shell 包标记为不再维护；这里锁定与 Tauri GTK 0.18 ABI 匹配的版本，只使用稳定的薄协议 API。升级 GTK/Tauri 时需重新验证，不能为更新绑定引入第二 GTK 大版本。
- `gtk-layer-shell` 是 Linux 构建与运行依赖，安装由系统包管理器负责；应用不提权或自动下载。无会话总线时 MPRIS 有界失败并诊断，不影响音频。
- CI 增加 `gtk-layer-shell` 构建依赖。原生忽略测试独立运行，使用隔离 XDG 目录；协议日志必须证明 overlay 层、configure/ack、输入区域和销毁重建。D-Bus 实测与浏览器布局检查分别记录，不代替人工多屏、物理媒体键或真实鼠标操作验收。
