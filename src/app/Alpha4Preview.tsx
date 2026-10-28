// SPDX-License-Identifier: GPL-3.0-only
// Isolated design review: synthetic data, no Tauri commands or persisted preferences.
import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { createRoot } from "react-dom/client";
import {
  ActionIcon, Button, createTheme, Group, MantineProvider, Menu, Modal,
  NativeSelect, ScrollArea, SegmentedControl, Slider, Switch, TextInput, Tooltip,
} from "@mantine/core";
import {
  AudioLines, Check, ChevronDown, Copy, Disc3, FolderOpen, ListMusic, Square,
  Mic2, Minus, Music2, Pause, Play, Plus, Repeat2, Settings, SkipBack, SkipForward,
  SlidersHorizontal, Volume2, Wrench, X,
} from "lucide-react";
import icon from "../../assets/resona-icon.svg";
import artwork from "../../assets/resona-default-artwork.svg";
import "@mantine/core/styles.css";
import "./alpha4Preview.css";

const params = new URLSearchParams(location.search);
const isCanvas = params.get("canvas") === "1";
const presets = [
  { value: "wide", label: "宽屏 · 1080 × 700", width: 1080, height: 700 },
  { value: "wide-min", label: "宽屏最小 · 760 × 520", width: 760, height: 520 },
  { value: "compact", label: "窄屏 · 420 × 720", width: 420, height: 720 },
  { value: "compact-min", label: "窄屏最小 · 360 × 600", width: 360, height: 600 },
];
const theme = createTheme({
  primaryColor: "violet", primaryShade: { light: 7, dark: 5 }, autoContrast: true,
  luminanceThreshold: 0.3, defaultRadius: "sm", activeClassName: "a4-active",
  fontFamily: "system-ui, sans-serif", headings: { fontFamily: "system-ui, sans-serif" },
});
const tracks = [
  ["夜航", "Resona Studio", "04:32"], ["微光", "Resona Studio", "03:48"],
  ["雨落之后", "Resona Studio", "05:16"], ["静谧的海岸", "Resona Studio", "04:08"],
  ["缓慢的清晨", "Resona Studio", "03:26"], ["归途", "Resona Studio", "03:26"],
];

function IconButton({ label, children, active, onClick, className = "", size = 36 }: {
  label: string; children: ReactNode; active?: boolean; onClick?: () => void;
  className?: string; size?: number;
}) {
  return <Tooltip label={label} openDelay={400} events={{ hover: true, focus: true, touch: false }}>
    <ActionIcon aria-label={label} aria-pressed={active} variant={active ? "light" : "subtle"}
      color={active ? "violet" : "gray"} size={size} className={className} onClick={onClick}>
      {children}
    </ActionIcon>
  </Tooltip>;
}

function Review() {
  const [preset, setPreset] = useState("wide");
  const [colorScheme, setColorScheme] = useState<"dark" | "light">("dark");
  const [material, setMaterial] = useState("mica");
  const [content, setContent] = useState("tracks");
  const host = useRef<HTMLDivElement>(null);
  const [available, setAvailable] = useState(1080);
  const dimensions = presets.find((item) => item.value === preset)!;
  const scale = Math.min(1, available / dimensions.width);
  useEffect(() => {
    const observer = new ResizeObserver(([entry]) => setAvailable(entry.contentRect.width));
    if (host.current) observer.observe(host.current);
    return () => observer.disconnect();
  }, []);
  const query = new URLSearchParams({ canvas: "1", layout: preset.startsWith("compact") ? "compact" : "wide",
    theme: colorScheme, material, content });
  const production = params.get("production") === "1";
  if (production) query.set("material", material === "kde" ? "kde_blur" : material);
  const canvasUrl = `${production ? "/index.html" : "/alpha4-preview.html"}?${query}`;
  return <MantineProvider theme={theme} forceColorScheme={colorScheme}>
    <main className="a4-review">
      <header className="a4-review-heading">
        <div><span className="a4-eyebrow">RESONA / 0.3.0-alpha.4</span><h1>标题栏与布局提案</h1>
          <p>移除重复品牌行，让音乐列表更靠上。</p></div>
        <a href={canvasUrl} target="_blank" rel="noreferrer">打开独立窗口 ↗</a>
      </header>
      <div className="a4-review-controls">
        <NativeSelect label="窗口尺寸" data={presets.map(({ value, label }) => ({ value, label }))}
          value={preset} onChange={(event) => setPreset(event.currentTarget.value)} />
        <div><label>主题</label><SegmentedControl aria-label="主题" value={colorScheme}
          onChange={(value) => setColorScheme(value as "dark" | "light")}
          data={[{ label: "深色", value: "dark" }, { label: "浅色", value: "light" }]} /></div>
        <div><label>材质示意</label><SegmentedControl aria-label="材质示意" value={material} onChange={setMaterial}
          data={[{ label: "Mica", value: "mica" }, { label: "KDE 模糊", value: "kde" }, { label: "实色", value: "solid" }]} /></div>
        <div><label>内容</label><SegmentedControl aria-label="内容" value={content} onChange={setContent}
          data={[{ label: "示例列表", value: "tracks" }, { label: "空列表", value: "empty" }]} /></div>
      </div>
      <div className="a4-review-stage" ref={host}>
        <div className="a4-scaled-canvas" style={{ width: dimensions.width * scale, height: dimensions.height * scale }}>
          <iframe title="alpha.4 交互设计预览" src={canvasUrl}
            style={{ width: dimensions.width, height: dimensions.height, transform: `scale(${scale})` }} />
        </div>
      </div>
      <div className="a4-review-caption"><span>{dimensions.width} × {dimensions.height} · 显示比例 {Math.round(scale * 100)}%</span>
        <span>可点击工具、设置、列表和播放控件；所有内容均为演示。</span></div>
      <section className="a4-review-notes">
        <div><h2>一行标题栏</h2><p>48px 高。窄屏的工具、设置与窗口控件共用一行；在最小宽度下仍留出拖动空间。</p></div>
        <div><h2>一个连续外壳</h2><p>标题栏、导航、底栏共享材质；内容只叠加一层背景，避免多重卡片。</p></div>
        <div><h2>原生行为单独验收</h2><p>这里展示布局和配色。Mica、KWin 模糊、窗口拖动及系统吸附需要原生实现与实机验证。</p></div>
      </section>
      <footer className="a4-review-sources">设计参考：
        <a href="https://learn.microsoft.com/en-us/windows/apps/design/basics/titlebar-design" target="_blank" rel="noreferrer">WinUI 标题栏</a>
        <a href="https://learn.microsoft.com/en-us/windows/apps/design/basics/content-basics" target="_blank" rel="noreferrer">布局与间距</a>
        <a href="https://mantine.dev/core/action-icon/" target="_blank" rel="noreferrer">Mantine ActionIcon</a>
        <a href="https://mantine.dev/core/menu/" target="_blank" rel="noreferrer">Menu</a>
      </footer>
    </main>
  </MantineProvider>;
}

function Canvas() {
  const compact = params.get("layout") === "compact";
  const initialTheme = params.get("theme") === "light" ? "light" : "dark";
  const [scheme, setScheme] = useState<"dark" | "light">(initialTheme);
  const material = params.get("material") ?? "mica";
  const empty = params.get("content") === "empty";
  const [page, setPage] = useState("playlist");
  const [playlists, setPlaylists] = useState(["默认", "夜间聆听", "通勤"]);
  const [selected, setSelected] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [track, setTrack] = useState(0);
  const [repeat, setRepeat] = useState(false);
  const [lyrics, setLyrics] = useState(false);
  const [progress, setProgress] = useState(34);
  const [newList, setNewList] = useState(false);
  const [listName, setListName] = useState("");
  const [notice, setNotice] = useState<string | null>(null);
  const [maximized, setMaximized] = useState(false);
  const [focused, setFocused] = useState(true);
  const selectList = (index: number) => { setSelected(index); setPage("playlist"); };
  const addList = () => {
    if (!listName.trim()) return;
    setPlaylists([...playlists, listName.trim()]); setSelected(playlists.length); setPage("playlist");
    setListName(""); setNewList(false);
  };
  const globalActions = <>
    <IconButton label="工具" active={page === "tools"} className="a4-title-action" size={44}
      onClick={() => setPage(page === "tools" ? "playlist" : "tools")}><Wrench size={18} /></IconButton>
    <IconButton label="设置" active={page === "settings"} className="a4-title-action" size={44}
      onClick={() => setPage(page === "settings" ? "playlist" : "settings")}><Settings size={18} /></IconButton>
  </>;
  return <MantineProvider theme={theme} forceColorScheme={scheme}>
    <div className="a4-canvas" data-scheme={scheme}>
      <div className="a4-window" data-layout={compact ? "compact" : "wide"} data-material={material}
        data-maximized={maximized || undefined} data-inactive={!focused || undefined}>
        <header className="a4-titlebar" aria-label="窗口标题栏">
          <div className="a4-identity"><img src={icon} alt="" /><span>Resona</span></div>
          <div className="a4-drag-space" aria-label="标题栏拖动区" onDoubleClick={() => setMaximized(!maximized)} />
          {compact && <nav className="a4-title-app-actions" aria-label="应用导航">{globalActions}</nav>}
          <div className="a4-caption-actions">
            <IconButton label="最小化" size={44} className="a4-caption-button" onClick={() => setNotice("最小化由原生窗口处理；当前为浏览器布局预览。") }><Minus size={14} /></IconButton>
            <IconButton label={maximized ? "还原" : "最大化"} size={44} className="a4-caption-button"
              onClick={() => setMaximized(!maximized)}>{maximized ? <Copy size={12} /> : <Square size={12} strokeWidth={1.3} />}</IconButton>
            <IconButton label="关闭" size={44} className="a4-caption-button a4-close" onClick={() => setNotice("正式版关闭按钮沿用托盘、退出确认和安装保护；预览不会关闭网页。") }><X size={16} /></IconButton>
          </div>
        </header>
        {compact ? <nav className="a4-compact-navigation" aria-label="播放列表">
          <ScrollArea type="scroll" scrollbars="x" className="a4-tab-scroll">
            <div className="a4-tabs">{playlists.map((name, index) => <Button key={index} variant={page === "playlist" && selected === index ? "light" : "subtle"}
              color={page === "playlist" && selected === index ? "violet" : "gray"} onClick={() => selectList(index)}
              aria-pressed={page === "playlist" && selected === index} title={name} size="compact-sm"><span className="a4-order">{index + 1}</span><span className="a4-list-name">{name}</span></Button>)}</div>
          </ScrollArea>
          <IconButton label="新建播放列表" onClick={() => setNewList(true)}><Plus size={19} /></IconButton>
        </nav> : <aside className="a4-sidebar">
          <div className="a4-sidebar-heading"><span>播放列表</span><IconButton label="新建播放列表" onClick={() => setNewList(true)}><Plus size={18} /></IconButton></div>
          <ScrollArea type="scroll" className="a4-playlists">{playlists.map((name, index) => <Button fullWidth key={index}
            className="a4-playlist" justify="start" variant={page === "playlist" && selected === index ? "light" : "subtle"}
            color={page === "playlist" && selected === index ? "violet" : "gray"}
            aria-pressed={page === "playlist" && selected === index} onClick={() => selectList(index)}>
            <span className="a4-order">{index + 1}</span><span className="a4-list-name">{name}</span>
          </Button>)}</ScrollArea>
          <nav className="a4-bottom-nav" aria-label="应用导航">
            <Button variant={page === "tools" ? "light" : "subtle"} color={page === "tools" ? "violet" : "gray"} justify="start" leftSection={<Wrench size={18} />} onClick={() => setPage("tools")}>工具</Button>
            <Button variant={page === "settings" ? "light" : "subtle"} color={page === "settings" ? "violet" : "gray"} justify="start" leftSection={<Settings size={18} />} onClick={() => setPage("settings")}>设置</Button>
          </nav>
        </aside>}
        <main className="a4-main">
          {page === "playlist" && <>
            <div className="a4-page-heading"><div><h1 title={playlists[selected]}>{playlists[selected]}</h1>
              <p>{empty ? "打开或拖入音频" : "6 首歌曲 · 24 分 36 秒"}</p></div>
              <Menu position="bottom-end" withinPortal><Menu.Target><Button variant="default" size="sm" leftSection={<Plus size={17} />} rightSection={<ChevronDown size={14} />}>添加</Button></Menu.Target>
                <Menu.Dropdown><Menu.Item leftSection={<Music2 size={16} />} onClick={() => setNotice("正式版在此打开原生音频文件选择器。")}>添加音频</Menu.Item>
                  <Menu.Item leftSection={<FolderOpen size={16} />} onClick={() => setNotice("正式版在此打开原生文件夹选择器。")}>添加文件夹</Menu.Item></Menu.Dropdown>
              </Menu>
            </div>
            {empty ? <div className="a4-empty"><div><Disc3 size={34} strokeWidth={1.5} /></div><h2>播放列表为空</h2><p>添加音频，开始聆听</p></div> : <>
              <div className="a4-table-heading"><span>#</span><span>标题</span><span className="a4-album-column">专辑</span><span>时长</span></div>
              <ScrollArea type="scroll" className="a4-track-scroll"><div className="a4-track-list">{tracks.map(([name, artist, duration], index) => <button key={name}
                className="a4-track" data-current={index === track || undefined} aria-label={`播放 ${name}`} aria-pressed={index === track && playing}
                onClick={() => { setTrack(index); setPlaying(true); }}>
                <span className="a4-track-number">{index === track && playing ? <AudioLines size={16} /> : index + 1}</span>
                <span className="a4-track-copy"><strong>{name}</strong><span>{artist}</span></span>
                <span className="a4-album-column">演示专辑</span><span className="a4-duration">{duration}</span>
              </button>)}</div></ScrollArea>
              <div className="a4-list-footer">FLAC · 本地音频</div>
            </>}
          </>}
          {page === "tools" && <div className="a4-subpage"><h1>工具</h1><p className="a4-muted">音频处理</p>
            <Button variant="subtle" color="gray" className="a4-tool-entry" leftSection={<SlidersHorizontal size={24} />}
              onClick={() => setNotice("音频压缩窗口将沿用相同的标题栏和材质；转换逻辑保持现有实现。")}>WAV → FLAC 无损压缩</Button>
          </div>}
          {page === "settings" && <ScrollArea type="scroll" className="a4-settings-scroll"><div className="a4-subpage"><h1>设置</h1>
            <h2>外观</h2><div className="a4-setting"><div><strong>颜色模式</strong><p>标题栏与内容同步切换</p></div>
              <SegmentedControl value={scheme} onChange={(value) => setScheme(value as "dark" | "light")} data={[{ label: "浅色", value: "light" }, { label: "深色", value: "dark" }]} /></div>
            <div className="a4-setting"><div><strong>窗口焦点</strong><p>查看失焦时的标题栏状态</p></div><Switch aria-label="窗口焦点" checked={focused} onChange={(event) => setFocused(event.currentTarget.checked)} /></div>
            <h2>播放</h2><div className="a4-setting"><div><strong>输出设备</strong><p>系统默认输出</p></div><Volume2 size={20} /></div>
          </div></ScrollArea>}
        </main>
        <footer className="a4-player" aria-label="播放器">
          <div className="a4-player-info"><img src={artwork} alt="" /><div><strong>{empty ? "未在播放" : tracks[track][0]}</strong>
            <span>{empty ? "选择一首歌曲" : tracks[track][1]}</span><small>{empty ? "0:00 / 0:00" : "1:32 / " + tracks[track][2]} {!empty && <b>FLAC</b>}</small></div></div>
          <div className="a4-playback"><IconButton label="上一首" size={44} onClick={() => setTrack((track + tracks.length - 1) % tracks.length)}><SkipBack size={20} /></IconButton>
            <ActionIcon aria-label={playing ? "暂停" : "播放"} size={44} radius="xl" variant="filled" disabled={empty} onClick={() => setPlaying(!playing)}>
              {playing ? <Pause size={21} fill="currentColor" /> : <Play size={21} fill="currentColor" />}</ActionIcon>
            <IconButton label="下一首" size={44} onClick={() => setTrack((track + 1) % tracks.length)}><SkipForward size={20} /></IconButton></div>
          <div className="a4-player-left"><IconButton label="音量" size={44} onClick={() => setNotice("预览音量控件；不会修改系统音量。") }><Volume2 size={20} /></IconButton>
            <IconButton label="列表循环" size={44} active={repeat} onClick={() => setRepeat(!repeat)}><Repeat2 size={20} /></IconButton></div>
          <div className="a4-player-right"><IconButton label="桌面歌词" size={44} active={lyrics} onClick={() => setLyrics(!lyrics)}><Mic2 size={20} /></IconButton>
            <IconButton label="定位当前歌曲" size={44} onClick={() => setPage("playlist")}><ListMusic size={20} /></IconButton></div>
          <Slider className="a4-progress" aria-label="播放进度" value={empty ? 0 : progress} onChange={setProgress} disabled={empty} size={3} thumbSize={10} />
        </footer>
      </div>
      <Modal opened={newList} onClose={() => setNewList(false)} title="新建播放列表" centered size="xs">
        <form onSubmit={(event) => { event.preventDefault(); addList(); }}><TextInput label="名称" value={listName} onChange={(event) => setListName(event.currentTarget.value)} data-autofocus />
          <Group justify="end" mt="md"><Button variant="default" onClick={() => setNewList(false)}>取消</Button><Button type="submit" disabled={!listName.trim()}>创建</Button></Group></form>
      </Modal>
      <Modal opened={notice !== null} onClose={() => setNotice(null)} title="设计预览" centered size="xs"><p>{notice}</p><Button fullWidth onClick={() => setNotice(null)} leftSection={<Check size={16} />}>知道了</Button></Modal>
    </div>
  </MantineProvider>;
}

createRoot(document.getElementById("root")!).render(isCanvas ? <Canvas /> : <Review />);
