# ADR 0033：无边框标题栏与 KDE 原生模糊外壳

- 状态：Accepted；所有者于 2026-10-28 批准方案并授权实现；本机测试通过后，进一步授权 dev → main PR 合并、分支同步与发布。
- 日期：2026-10-28。
- 替代 ADR 0020/0021/0022 的顶部结构、原生标题栏和 Linux 实色限制。
- 详细布局、尺寸、参考、交互和验证见 [alpha.4 方案](../plans/0.3.0-alpha.4.md)。

## 背景

Windows 的 Mica 与原生标题栏能形成连续外壳；Linux 的 GTK 标题栏与 Mantine 实色内容存在视觉差异。所有者希望把工具与设置移入窄屏标题栏，并移除重复的大号品牌行，提升内容利用率。

## 决定

1. 主窗口采用无边框布局，应用拥有标题栏内容与视觉；系统窗口动作仍经 Tauri/平台 adapter 执行。Windows 原生 Snap、系统菜单和缩放能力必须有单独验收，不能用纯网页点击动作替代。
2. 宽屏保留标题栏小图标/名称、右侧窗口控件；列表直接从侧栏顶部开始。窄屏将工具、设置置于窗口控件左侧，删除单独的品牌/工具行，保留横向播放列表。
3. React 只消费类型化窗口/材质快照并提交用户意图；关闭复用生命周期服务。播放、列表、压缩、更新状态和数据迁移不因改壳而重建。
4. Windows 11 继续由现有 adapter 提供 Mica；KDE Wayland 增加原生背景效果 capability 与 adapter，复用外壳/内容语义 token，失败使用实色。具体协议随实际 compositor 能力探测，不另建 Qt 窗口栈、不安装强制模糊插件、不使用全局 CSS `backdrop-filter`。
5. 主窗口与压缩窗口共享普通窗口壳策略；桌面歌词保持独立透明与 layer-shell/Win32 边界。
6. Mantine 继续承担按钮、菜单、提示、键盘焦点与弹层；以 WinUI 3 规范参考布局、命中区域和状态。保留 system-ui、明暗/失焦/高对比度和可靠回退，不引入第二组件库。

## 代价与验证

无边框后应用需要承担窗口控件布局、拖动区划分和额外平台适配。必须验证原生菜单、边缘缩放、DPI、吸附、最大化、阴影、圆角及退出保护。KWin 模糊需要独立测量 GPU/功耗，不能由网页预览或 CPU 使用率证明。

## 实现边界

- 共享 48px 标题栏；窄屏 36px 标签行，五个 44×48px 按钮。宽屏侧栏 208px，删除字标和旧品牌行。压缩复用普通标题栏；歌词保持独立。
- 所有者于 2026-10-28 实测后批准标题栏视觉统一：品牌图标 16px，标题宽窄均为 13px/400，右侧图标统一 16px 画布和 1.5px 实际线宽，默认使用同一中性灰。保留 8px 工具/窗口控件分组、主题色选中、红色关闭悬停及高对比度覆盖；尺寸只改变图形，不缩小命中区域。
- Windows 使用 Tauri 原生操作、Win32 HTMAXBUTTON 与系统菜单，按每次命中时的 DPI 计算几何。可访问按钮、窗口状态事件、八方向原生 resize 保留。
- KDE 使用官方生成的 `wayland-client` / `wayland-protocols` / `wayland-protocols-plasma` 绑定和 GTK 同连接的借用 surface。优先 ext-background-effect-v1 capability，兼容 org_kde_kwin_blur_manager；不支持的 compositor 使用实色。新版 capability 撤回或旧 global 移除通知前端回退。
- 正文使用不透明内容面；只请求正文矩形之外的外壳区域。ResizeObserver 仅在几何变化提交区域，不进入播放轮询。KDE 样式不使用 backdrop-filter 或整体 opacity。
- GTK 负责读取共享 socket；adapter 每 200ms 仅 drain 自己的 pending protocol queue，不阻塞 GTK、不读第二连接、不轮询桌面画面。此低频事件队列调度是已知简化，原生性能测试后再决定是否改用专用 GLib source。
- 初始化失败/超时记录诊断并回退；窗口 unrealize 清理协议对象，不能销毁 GTK 拥有的 display/surface。新协议由 GTK 下一次绘制提交，不直接 commit 借用 surface。
- 初次实施在无 WSL/KDE 的 Windows 主机完成；后续 CachyOS/KDE 原生补验见 [Linux 本机记录](../plans/0.3.0-alpha.4-linux-validation.md)。编译、纯逻辑、真实音频、协议和桌面集成已执行；所有者已确认最终测试程序的功能和标题栏视觉通过；压缩最小化动画自动观察仍失败，系统菜单/鼠标未逐项记录，GPU/功耗未测。
