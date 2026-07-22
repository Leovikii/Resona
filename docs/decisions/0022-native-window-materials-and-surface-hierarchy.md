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

- 共享 CSS 只暴露 `window background`、`chrome surface`、`content surface`、`player surface`、`subtle surface` 和 `hairline` 等语义 token。普通组件继续消费 Mantine token，不感知 Mica。
- 普通窗口采用“纯 Mica 外壳 + 单一内容层”模型：标题栏、宽屏侧栏、窄屏顶部导航和稳定底栏直接透出同一根 Mica；右侧主内容覆盖唯一一层低 alpha `content surface`，形成 WinUI `NavigationView` 式内容面。完整播放器在主内容内部保持透明，不能再叠加第二层 content fill。不得用连续多层半透明表面覆盖根材质，也不得使用整高/整宽 `hairline` 切割应用壳。
- Mica 下 `chrome surface` 与 `player surface` 解析为透明，深色 content/subtle 分别使用 `18%` 黑和 `6%` 白，浅色分别使用 `72%` 白和 `52%` 白。`solid` 回退仍把 chrome/player/content 解析为不透明主题表面。这些值是窗口级语义 token，不得复制到 feature；调整时必须在原生 Mica 上验证可感知层级，不能只比较浏览器计算样式。
- Mantine 的全局样式会为 `body` 写入 `--mantine-color-body`。Mica 模式必须以明确高于该全局规则的窗口材质选择器同时清除 `html`、`body` 和 `#root` 背景；只把应用壳或语义 token 设为透明不足以透出原生材质。不得把 `--mantine-color-body` 本身改成透明，因为 Paper、输入框和弹层仍需要实体主题表面。
- 每个区域最多增加一层有明确语义的局部实体表面。工具入口、状态带及需要从背景中抬起的紧凑工作单元可使用无描边 `subtle surface`；播放列表标题和普通曲目行保持透明、无卡片描边，只在 hover、选中、播放中或拖放反馈时上色。菜单、Modal、Popover、错误提示和文字密集交互区继续保持有界实体表面与足够对比度。
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
- 纯 Mica 外壳、单层 content fill 和按状态上色的列表行不会为每行曲目创建常驻合成层，也不会进入播放高频更新路径；相较多块 tonal chrome、连续遮罩或逐行描边，更接近 WinUI 3 的 NavigationView 层级并保持整体感。

## 验收

- 浏览器使用 `material=mica` 预览验证宽屏 `1080×700`、窄屏 `360×600`/`420×720`、压缩窗口 `720×500` 和桌面歌词；覆盖浅色/深色、透明 Mica 外壳、单一 content layer、无描边列表行、设置节间距、零横向溢出、固定底栏/导航几何和控制可达性。浏览器计算样式不能证明 Windows WebView 已透出 Mica，也不能证明 alpha 表面在真实桌面采样后肉眼可分。
- Windows 11 Release 必须使用 DPI-aware 的原生窗口截图复核侧栏、主内容、命令区和底栏；至少记录同一截图中的区域采样值或等价对比证据。仅有 HWND、浏览器截图或不同的计算样式不构成材质验收通过。
- Windows Release 验证主窗口与压缩窗口冷/暖启动、主题切换、最大化/还原、阴影和首帧；不支持 Mica 的平台必须完整实色回退。
- 桌面歌词继续执行 ADR 0012/0019 的透明、锁定穿透和 helper 实机门禁，普通窗口材质不能改变该生命周期。
