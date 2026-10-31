# ADR 0033：无边框标题栏与 KDE 原生模糊外壳

- 状态：Accepted；所有者于 2026-10-28 批准方案并授权实现；本机测试通过后，进一步授权 dev → main PR 合并、分支同步与发布。
- 日期：2026-10-28。
- 替代 ADR 0020/0021/0022 的顶部结构、原生标题栏和 Linux 实色限制。
- rc.1 的修复和阴影范围见 [当前计划](../plans/0.3.0-rc.1.md)；旧布局方案与验收历史保存在 Git。

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
- rc.1：外壳模糊区域与窗口四周保持 1 个 surface-local 逻辑像素距离；同一几何函数覆盖新旧协议及主/压缩窗口，尺寸和输入区域不变。KWin 6.8.2 / Wayland 双屏 150% 下，所有者确认关闭模糊及内收边界均消除拖动残线，选择内收以保留现有外观。这是已验证场景的兼容规避，未认定 compositor 或驱动内部根因；不恢复 GTK 装饰、不增加重绘轮询或关闭硬件加速。
- rc.1 阴影：所有者进一步批准参照 Windows 的柔和外围阴影。主/压缩窗口复用同连接 `org_kde_kwin_shadow_manager`，以一套固定 24 逻辑像素边距的黑色渐隐贴片交给 KWin 合成；不扩展网页透明边距、不改变输入区域、不新增窗口或设置。最大化/全屏时 unset，恢复普通状态复用缓冲区；阴影与 blur 独立，保留 1px 模糊内收。要求 shadow manager 与 wl_shm 至少 v2，以便释放拥有的协议对象；缺失/失败保留无阴影窗口，版本不足及资源创建失败记录诊断，不影响模糊能力。
- GTK 负责读取共享 socket；adapter 每 200ms 仅 drain 自己的 pending protocol queue，不阻塞 GTK、不读第二连接、不轮询桌面画面。此低频事件队列调度是已知简化，原生性能测试后再决定是否改用专用 GLib source。
- 初始化失败/超时记录诊断并回退；GTK 可能在 unmap 信号前销毁 wl_surface，清理只释放应用拥有的效果/缓冲区，不再向旧 surface 发送 unset。unmap/unrealize 清理、map 重建会话并保留正文矩形；不能销毁 GTK 拥有的 display/surface。新协议由 GTK 下一次绘制提交，不直接 commit 借用 surface。
- 所有者于 2026-10-31 确认 rc.1 最终阴影测试版通过并授权发布。协议/生命周期测试不能替代物理命中、动画或功耗验证；未测与失败项集中保留在 [KNOWN_ISSUES](../KNOWN_ISSUES.md)。
