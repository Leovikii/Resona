# ADR 0022：原生窗口材质与跨窗口表面层级

- 状态：Accepted
- 日期：2026-07-22
- 补充：[ADR 0006](0006-application-shell-theme-and-locale.md) 的主题实现
- 补充：[ADR 0016](0016-audio-compression-workspace-window.md) 的普通辅助窗口
- 保留：[ADR 0012](0012-desktop-lyrics-window-and-unlock-helper.md) 的桌面歌词透明窗口边界

## 背景

Resona 的功能布局和主要交互已经收口，但普通 WebView 使用完全不透明的统一页面背景，主窗口与音频压缩窗口仍有较强网页感。Windows 11 提供 Mica 应用背景，Tauri 2 已直接暴露 Window Effects；若用全局 CSS 模糊、自绘标题栏或第二套 Fluent 组件库实现类似效果，会增加 GPU 成本、窗口行为风险和跨平台维护面。

## 决定

### 平台材质

- 主窗口与音频压缩窗口在 Windows 11 客户端启用 Tauri 原生 Mica，保留系统标题栏、阴影、缩放和窗口菜单，不实现无边框自绘标题栏。
- Windows 11 通过真实系统构建号 `>= 22000` 且非 Server 判断。Windows 10、Windows Server、Linux 和效果不可用路径使用现有 Mantine 实色背景。
- Windows 主窗口的透明创建属性只存在于 `tauri.windows.conf.json`；通用 `tauri.conf.json` 保持跨平台不透明默认。动态压缩窗口由 `platform/window_material` 在构建时决定是否透明。
- Mica 初始化在主窗口显示前完成。失败必须记录错误并保持/恢复 `solid` 语义，不能留下透明黑窗、白闪或不可读界面。
- 主题偏好以 `auto`/`light`/`dark` 原样跨边界传递。`auto` 必须清除原生窗口主题覆盖并使用系统自适应 Mica；手动浅色/深色才同时固定 Mantine、原生标题栏和对应 Mica 变体。不得先把 `auto` 压成当前亮暗值，否则手动主题会反向污染 WebView 的系统主题查询。

### 表面层级

- 共享 CSS 只暴露 `window background`、`chrome surface`、`content surface`、`subtle surface` 和 `hairline` 等语义 token。普通组件继续消费 Mantine token，不感知 Mica。
- 普通窗口采用单层材质模型：Mica 是根窗口唯一的大面积背景。Mica 模式下主内容和完整播放器的 `content surface` 保持透明；侧栏、窄屏顶部导航、底栏、压缩窗口标题区和命令区使用一层低对比 `chrome surface` tonal tint 表达稳定应用区域。不得再用连续多层半透明黑色或白色表面覆盖根材质，也不得使用整高/整宽 `hairline` 切割应用壳。
- Mantine 的全局样式会为 `body` 写入 `--mantine-color-body`。Mica 模式必须以明确高于该全局规则的窗口材质选择器同时清除 `html`、`body` 和 `#root` 背景；只把应用壳或语义 token 设为透明不足以透出原生材质。不得把 `--mantine-color-body` 本身改成透明，因为 Paper、输入框和弹层仍需要实体主题表面。
- 每个区域最多增加一层有明确语义的局部实体表面。工具入口、状态带及需要从背景中抬起的紧凑工作单元可使用无描边 `subtle surface`，以项目间空隙而不是相邻边线分隔；菜单、Modal、Popover、选中态、错误提示和文字密集交互区继续保持有界实体表面与足够对比度。
- 设置页不是卡片集合。设置分组使用标题和节间距建立层级，分组本身保持透明、无阴影、无外框、无额外圆角，也不使用贯穿内容区的相邻分组 `hairline`；只有真正独立、可操作的重复项目才使用局部表面。
- `hairline` 只用于表格、输入控件、菜单、弹层或其他必须表达精确边界的内容，不作为侧栏、顶部导航、主内容和底栏之间的通用区域分隔方式。
- `solid` 回退仍将 `window`、`chrome` 和 `content` token 解析为完整不透明背景，保证 Windows 10/Linux 不依赖透明窗口或桌面采样也能完整阅读。组件不得为 Mica 和 solid 维护两套布局。
- 禁止使用全局 `backdrop-filter`、全窗口 Acrylic 或为每个列表项增加半透明/模糊层。材质变化不得触发播放、列表或歌词状态重建。

### 子窗口一致性

- 音频压缩窗口复用同一原生材质 adapter 和语义表面层级，但仍保持独立最小 capability、窗口生命周期和 Rust 权威转换状态。
- 桌面歌词窗口继续是透明、置顶、可穿透的专用窗口，固定使用 `solid` 材质标识，不叠加 Mica。它只同步字体、圆角、轻量 ActionIcon 状态、边界和动效语言；正文颜色与透明度仍由歌词偏好独立控制。

## 理由

- Tauri 内置 Window Effects 避免新增第三方材质插件；Mica 采样桌面背景且比实时 Acrylic 模糊稳定、节制。
- 保留系统窗口装饰避免承担命中测试、系统菜单、最大化、DPI、无障碍和多显示器窗口按钮的长期维护成本。
- 平台 adapter 与语义 token 将 Windows 能力限制在窗口边界，未来 Wayland 只需使用 solid 实现，不需要改页面组件。
- 单层根材质、稳定 tonal chrome 和少量局部实体表面不会为每行曲目创建合成层，也不会进入播放高频更新路径；相较连续遮罩或贯穿页面的描边，更容易保持 Mica 可见并形成清晰的 WinUI 式表面层级。

## 验收

- 浏览器使用 `material=mica` 预览验证宽屏 `1080×700`、窄屏 `360×600`、压缩窗口 `720×500` 和桌面歌词；覆盖浅色/深色、透明主内容、导航/播放器 tonal tint、无描边工具表面、设置节间距、零横向溢出、固定底栏/导航几何和控制可达性。浏览器计算样式不能证明 Windows WebView 已透出 Mica，Release 还必须对主内容空白区做原生截图/像素检查，禁止再次出现 solid 回退色。
- Windows Release 验证主窗口与压缩窗口冷/暖启动、主题切换、最大化/还原、阴影和首帧；不支持 Mica 的平台必须完整实色回退。
- 桌面歌词继续执行 ADR 0012/0019 的透明、锁定穿透和 helper 实机门禁，普通窗口材质不能改变该生命周期。
