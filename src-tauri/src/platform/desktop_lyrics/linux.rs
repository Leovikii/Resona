// SPDX-License-Identifier: GPL-3.0-only

use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use gtk::prelude::*;
use gtk_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use super::{DesktopLyricsWindowFailure as Failure, DesktopLyricsWindowSnapshot as Snapshot};

// GTK objects stay on the event thread; commands carry only owned data across threads.
thread_local! { static WINDOWS: RefCell<Option<Resources>> = const { RefCell::new(None) }; }

struct Resources {
    webview: WebviewWindow,
    window: gtk::ApplicationWindow,
    helper: gtk::Window,
    geometry: Geometry,
    area: (i32, i32),
    gesture: Option<Gesture>,
    path: PathBuf,
    font_size: u32,
    ready: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct Geometry {
    x: i32,
    y: i32,
    width: i32,
}
impl Default for Geometry {
    fn default() -> Self {
        Self {
            x: 100,
            y: 600,
            width: 760,
        }
    }
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GestureKind {
    Move,
    East,
    West,
}
struct Gesture {
    kind: GestureKind,
    x: f64,
    y: f64,
    width: i32,
}

#[derive(Default)]
pub struct DesktopLyricsWindowService {
    app: Mutex<Option<AppHandle>>,
    state: Arc<Mutex<Snapshot>>,
}

impl DesktopLyricsWindowService {
    pub fn initialize(&self, app: &AppHandle) {
        *self.app.lock().unwrap() = Some(app.clone());
        let supported = gtk_layer_shell::is_supported();
        *self.state.lock().unwrap() = Snapshot {
            supported,
            layer_shell: supported,
            ..Snapshot::default()
        };
        if !supported {
            log::warn!("desktop lyrics require a Wayland compositor with layer-shell");
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        *self.state.lock().unwrap()
    }

    fn on_main<T: Send + 'static>(
        &self,
        action: impl FnOnce() -> Result<T, Failure> + Send + 'static,
    ) -> Result<T, Failure> {
        let app = self
            .app
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| failure("desktop lyrics service is not initialized"))?;
        let (sender, receiver) = mpsc::sync_channel(1);
        app.run_on_main_thread(move || {
            let _ = sender.send(action());
        })
        .map_err(failure)?;
        receiver
            .recv_timeout(Duration::from_secs(5))
            .map_err(failure)?
    }

    pub fn show(
        self: &Arc<Self>,
        app: &AppHandle,
        font_size: Option<u32>,
    ) -> Result<Snapshot, Failure> {
        if !self.snapshot().supported {
            return Err(failure("Wayland layer-shell is unavailable"));
        }
        let app = app.clone();
        let service = Arc::clone(self);
        self.on_main(move || WINDOWS.with(|slot| {
            if slot.borrow().is_some() { return Ok(service.snapshot()); }
            let font_size = font_size.unwrap_or(28).clamp(16, 64);
            let path = app.path().app_local_data_dir().map_err(failure)?.join("desktop-lyrics-wayland.json");
            let saved = match std::fs::read(&path) {
                Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|error| { log::warn!("invalid Wayland lyrics geometry: {error}"); Geometry::default() }),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Geometry::default(),
                Err(error) => { log::warn!("cannot read Wayland lyrics geometry: {error}"); Geometry::default() }
            };
            let webview = WebviewWindowBuilder::new(&app, "desktop-lyrics", WebviewUrl::App("index.html?window=desktop-lyrics".into()))
                .title("Resona Desktop Lyrics").inner_size(760.0, height(font_size) as f64)
                .decorations(false).transparent(true).background_color((0,0,0,0).into())
                .skip_taskbar(true).focused(false).visible(false).resizable(true).build().map_err(failure)?;
            let result = (|| {
                let window = webview.gtk_window().map_err(failure)?;
                // A realized xdg_toplevel cannot be changed into a layer surface.
                if window.is_realized() { return Err(failure("lyrics window was realized before layer-shell initialization")); }
                let display = window.display();
                let monitor = app.get_webview_window("main").and_then(|main| main.gtk_window().ok())
                    .and_then(|main| main.window()).and_then(|main| display.monitor_at_window(&main))
                    .or_else(|| display.primary_monitor()).or_else(|| display.monitor(0))
                    .ok_or_else(|| failure("no Wayland output is available"))?;
                let bounds = monitor.geometry();
                let area = (bounds.width(), bounds.height());
                let geometry = constrain(saved, area, height(font_size));
                configure_layer(&window, &monitor, "io.github.vki.resona.lyrics");
                let entering = app.clone();
                let leaving = app.clone();
                webview.with_webview(move |view| {
                    view.inner().add_events(gtk::gdk::EventMask::ENTER_NOTIFY_MASK | gtk::gdk::EventMask::LEAVE_NOTIFY_MASK);
                    view.inner().connect_enter_notify_event(move |_, event| {
                        if event.detail() != gtk::gdk::NotifyType::Inferior {
                            if let Err(error) = entering.emit_to("desktop-lyrics", "desktop-lyrics-pointer-inside", true) {
                                log::warn!("lyrics pointer enter notification failed: {error}");
                            }
                        }
                        gtk::glib::Propagation::Proceed
                    });
                    view.inner().connect_leave_notify_event(move |_, event| {
                        if event.detail() != gtk::gdk::NotifyType::Inferior {
                            if let Err(error) = leaving.emit_to("desktop-lyrics", "desktop-lyrics-pointer-inside", false) {
                                log::warn!("lyrics pointer leave notification failed: {error}");
                            }
                        }
                        gtk::glib::Propagation::Proceed
                    });
                }).map_err(failure)?;
                let helper = gtk::Window::new(gtk::WindowType::Toplevel);
                helper.set_decorated(false);
                helper.set_accept_focus(false);
                helper.set_resizable(false);
                helper.set_default_size(56, 38);
                configure_layer(&helper, &monitor, "io.github.vki.resona.lyrics-unlock");
                let button = gtk::Button::with_label("🔓");
                configure_helper_hover(&helper, &button);
                button.set_tooltip_text(Some("解锁桌面歌词 / Unlock desktop lyrics"));
                let weak = Arc::downgrade(&service);
                button.connect_clicked(move |_| { if let Some(service) = weak.upgrade() {
                    if let Err(error) = service.unlock() { log::warn!("lyrics unlock failed: {}", error.message); }
                }});
                helper.add(&button);
                let resources = Resources { webview: webview.clone(), window, helper, geometry, area, gesture: None, path, font_size, ready: false };
                resources.apply_geometry();
                log::info!("desktop lyrics configured: Wayland layer=overlay keyboard=none exclusive-zone=-1");
                Ok(resources)
            })();
            match result {
                Ok(resources) => {
                    *slot.borrow_mut() = Some(resources);
                    service.state.lock().unwrap().visible = true;
                    Ok(service.snapshot())
                }
                Err(error) => { let _ = webview.close(); Err(error) }
            }
        }))
    }

    pub fn window_ready(&self) -> Result<Snapshot, Failure> {
        self.on_main(|| {
            WINDOWS.with(|slot| {
                if let Some(resources) = slot.borrow_mut().as_mut() {
                    resources.window.show_all();
                    resources.ready = true;
                    log::info!("desktop lyrics layer surface shown");
                }
                Ok(())
            })
        })?;
        Ok(self.snapshot())
    }

    pub fn hide(&self, _app: &AppHandle) -> Result<Snapshot, Failure> {
        let state = Arc::clone(&self.state);
        self.on_main(move || {
            // Take resources before closing: Tauri's destruction events may reenter the service.
            let resources = WINDOWS.with(|slot| slot.borrow_mut().take());
            if let Some(resources) = resources {
                resources.persist();
                resources.helper.close();
                resources.webview.close().map_err(failure)?;
            }
            let mut state = state.lock().unwrap();
            state.visible = false;
            state.locked = false;
            Ok(*state)
        })
    }

    pub fn lock(&self) -> Result<Snapshot, Failure> {
        self.set_locked(true)
    }
    pub fn unlock(&self) -> Result<Snapshot, Failure> {
        self.set_locked(false)
    }
    fn set_locked(&self, locked: bool) -> Result<Snapshot, Failure> {
        let state = Arc::clone(&self.state);
        self.on_main(move || {
            WINDOWS.with(|slot| {
                let mut slot = slot.borrow_mut();
                let resources = slot
                    .as_mut()
                    .ok_or_else(|| failure("desktop lyrics are not visible"))?;
                if !resources.ready {
                    return Err(failure("desktop lyrics surface is not ready"));
                }
                if locked {
                    // Make recovery reachable before removing the lyrics input region.
                    resources.helper.child().unwrap().set_opacity(0.0);
                    resources.helper.show_all();
                    resources
                        .window
                        .input_shape_combine_region(Some(&gtk::cairo::Region::create()));
                } else {
                    resources.window.input_shape_combine_region(None);
                    resources.helper.hide();
                }
                resources.gesture = None;
                let mut state = state.lock().unwrap();
                state.locked = locked;
                Ok(*state)
            })
        })
    }

    pub fn pointer(&self, phase: String, kind: GestureKind, x: f64, y: f64) -> Result<(), Failure> {
        if !x.is_finite() || !y.is_finite() || x.abs() > 32768.0 || y.abs() > 32768.0 {
            return Err(failure("invalid lyrics pointer coordinates"));
        }
        let state = Arc::clone(&self.state);
        self.on_main(move || {
            if state.lock().unwrap().locked {
                return Ok(());
            }
            WINDOWS.with(|slot| {
                let mut slot = slot.borrow_mut();
                let Some(resources) = slot.as_mut() else {
                    return Ok(());
                };
                match phase.as_str() {
                    "begin" => {
                        resources.gesture = Some(Gesture {
                            kind,
                            x,
                            y,
                            width: resources.geometry.width,
                        })
                    }
                    "update" => {
                        if let Some(gesture) = &resources.gesture {
                            let dx = (x - gesture.x).round() as i32;
                            let dy = (y - gesture.y).round() as i32;
                            resources.geometry =
                                gesture_geometry(resources.geometry, gesture, dx, dy);
                            resources.geometry = constrain(
                                resources.geometry,
                                resources.area,
                                height(resources.font_size),
                            );
                            resources.apply_geometry();
                        }
                    }
                    "end" => {
                        resources.gesture = None;
                        resources.persist();
                    }
                    _ => return Err(failure("invalid lyrics gesture phase")),
                }
                Ok(())
            })
        })
    }

    pub fn fit_height(&self, font_size: u32) -> Result<Snapshot, Failure> {
        self.on_main(move || {
            WINDOWS.with(|slot| {
                if let Some(resources) = slot.borrow_mut().as_mut() {
                    resources.font_size = font_size.clamp(16, 64);
                    resources.geometry = constrain(
                        resources.geometry,
                        resources.area,
                        height(resources.font_size),
                    );
                    resources.apply_geometry();
                }
                Ok(())
            })
        })?;
        Ok(self.snapshot())
    }
    pub fn start_dragging(&self) -> Result<(), Failure> {
        Err(failure("layer surfaces use pointer gestures"))
    }
    pub fn persist_geometry(&self, _app: &AppHandle) {
        if let Err(error) = self.on_main(|| {
            WINDOWS.with(|slot| {
                if let Some(resources) = slot.borrow().as_ref() {
                    resources.persist();
                }
                Ok(())
            })
        }) {
            log::warn!("lyrics geometry: {}", error.message);
        }
    }
    // The compositor maintains the overlay layer; no focus, timer, or Z-order repair.
    pub fn maintain_topmost(&self) {}
    pub fn enforce_height(&self) {}
}

fn configure_layer(window: &impl IsA<gtk::Window>, monitor: &gtk::gdk::Monitor, namespace: &str) {
    window.init_layer_shell();
    window.set_namespace(namespace);
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(KeyboardMode::None);
    window.set_exclusive_zone(-1);
    window.set_monitor(monitor);
    window.set_anchor(Edge::Left, true);
    window.set_anchor(Edge::Top, true);
}

fn configure_helper_hover(helper: &gtk::Window, button: &gtk::Button) {
    // Keep the mapped surface and its input region alive; hide only its drawing.
    helper.set_app_paintable(true);
    if let Some(screen) = gtk::prelude::WidgetExt::screen(helper) {
        helper.set_visual(screen.rgba_visual().as_ref());
    }
    helper.connect_draw(|_, context| {
        context.set_operator(gtk::cairo::Operator::Source);
        context.set_source_rgba(0.0, 0.0, 0.0, 0.0);
        if let Err(error) = context.paint() {
            log::warn!("lyrics helper clear failed: {error}");
        }
        context.set_operator(gtk::cairo::Operator::Over);
        gtk::glib::Propagation::Proceed
    });
    button.set_opacity(0.0);
    button.add_events(
        gtk::gdk::EventMask::ENTER_NOTIFY_MASK | gtk::gdk::EventMask::LEAVE_NOTIFY_MASK,
    );
    button.connect_enter_notify_event(|button, _| {
        button.set_opacity(1.0);
        gtk::glib::Propagation::Proceed
    });
    button.connect_leave_notify_event(|button, event| {
        if event.detail() != gtk::gdk::NotifyType::Inferior {
            button.set_opacity(0.0);
        }
        gtk::glib::Propagation::Proceed
    });
}
impl Resources {
    fn apply_geometry(&self) {
        self.window
            .set_layer_shell_margin(Edge::Left, self.geometry.x);
        self.window
            .set_layer_shell_margin(Edge::Top, self.geometry.y);
        self.window
            .set_size_request(self.geometry.width, height(self.font_size));
        self.window.resize(1, 1);
        self.helper
            .set_layer_shell_margin(Edge::Left, self.geometry.x + (self.geometry.width - 56) / 2);
        self.helper
            .set_layer_shell_margin(Edge::Top, self.geometry.y);
    }
    fn persist(&self) {
        if let Err(error) = serde_json::to_vec(&self.geometry)
            .map_err(std::io::Error::other)
            .and_then(|bytes| std::fs::write(&self.path, bytes))
        {
            log::warn!("cannot save Wayland lyrics geometry: {error}");
        }
    }
}
fn height(font_size: u32) -> i32 {
    (56.0 + 2.24 * f64::from(font_size.clamp(16, 64)))
        .clamp(130.0, 240.0)
        .round() as i32
}
fn constrain(mut geometry: Geometry, area: (i32, i32), height: i32) -> Geometry {
    geometry.width = geometry.width.clamp(480.min(area.0), 1600.min(area.0));
    geometry.x = geometry.x.clamp(0, (area.0 - geometry.width).max(0));
    geometry.y = geometry.y.clamp(0, (area.1 - height).max(0));
    geometry
}
fn gesture_geometry(mut geometry: Geometry, gesture: &Gesture, dx: i32, dy: i32) -> Geometry {
    match gesture.kind {
        GestureKind::Move => {
            geometry.x = geometry.x.saturating_add(dx);
            geometry.y = geometry.y.saturating_add(dy);
        }
        GestureKind::East => geometry.width = gesture.width.saturating_add(dx),
        GestureKind::West => {
            let width = geometry.width.saturating_sub(dx).clamp(480, 1600);
            geometry.x += geometry.width - width;
            geometry.width = width;
        }
    }
    geometry
}
fn failure(error: impl std::fmt::Display) -> Failure {
    Failure::new("desktop_lyrics_wayland_failed", error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::Listener;
    #[test]
    fn geometry_stays_on_output_and_font_height_is_bounded() {
        let restored = constrain(
            Geometry {
                x: -200,
                y: 9000,
                width: 9000,
            },
            (1920, 1080),
            height(64),
        );
        assert_eq!((restored.x, restored.y, restored.width), (0, 881, 1600));
        assert_eq!(height(0), 130);
        assert_eq!(height(999), 199);
        let small = constrain(Geometry::default(), (400, 300), 130);
        assert_eq!((small.width, small.x, small.y), (400, 0, 170));
    }
    #[test]
    fn move_and_resize_use_surface_local_coordinates() {
        let geometry = Geometry {
            x: 100,
            y: 200,
            width: 760,
        };
        let moving = Gesture {
            kind: GestureKind::Move,
            x: 10.0,
            y: 10.0,
            width: 760,
        };
        let moved = gesture_geometry(geometry, &moving, 20, -30);
        assert_eq!((moved.x, moved.y), (120, 170));
        let east = Gesture {
            kind: GestureKind::East,
            ..moving
        };
        let resized = gesture_geometry(geometry, &east, 50, 0);
        assert_eq!(resized.width, 810);
        let west = Gesture {
            kind: GestureKind::West,
            ..moving
        };
        let resized = gesture_geometry(geometry, &west, 50, 0);
        assert_eq!((resized.x, resized.width), (150, 710));
    }
    #[test]
    #[ignore = "requires a live KDE Wayland session; run alone with isolated XDG data directories"]
    fn native_layer_surface_lifecycle() {
        let (finished, result) = mpsc::sync_channel(1);
        let service = Arc::new(DesktopLyricsWindowService::default());
        let app = tauri::Builder::default()
            .any_thread()
            .setup(move |app| {
                service.initialize(app.handle());
                let app = app.handle().clone();
                std::thread::spawn(move || {
                    let checked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        assert!(service.snapshot().supported);
                        assert!(service.show(&app, Some(28)).unwrap().visible);
                        service.window_ready().unwrap();
                        std::thread::sleep(Duration::from_millis(250));
                        service
                            .on_main(|| {
                                WINDOWS.with(|slot| {
                                    assert_eq!(
                                        slot.borrow().as_ref().unwrap().window.size().1,
                                        height(28)
                                    );
                                    Ok(())
                                })
                            })
                            .unwrap();
                        service.fit_height(64).unwrap();
                        std::thread::sleep(Duration::from_millis(250));
                        service
                            .on_main(|| {
                                WINDOWS.with(|slot| {
                                    assert_eq!(
                                        slot.borrow().as_ref().unwrap().window.size().1,
                                        height(64)
                                    );
                                    Ok(())
                                })
                            })
                            .unwrap();
                        service
                            .pointer("begin".into(), GestureKind::Move, 10.0, 10.0)
                            .unwrap();
                        service
                            .pointer("update".into(), GestureKind::Move, 50.0, 20.0)
                            .unwrap();
                        service
                            .pointer("end".into(), GestureKind::Move, 0.0, 0.0)
                            .unwrap();
                        assert!(service.lock().unwrap().locked);
                        let (pointer_sent, pointer_received) = mpsc::channel();
                        let pointer_window = app.get_webview_window("desktop-lyrics").unwrap();
                        let listener =
                            pointer_window.listen("desktop-lyrics-pointer-inside", move |event| {
                                pointer_sent.send(event.payload().to_owned()).unwrap();
                            });
                        std::thread::sleep(Duration::from_millis(250));
                        service
                            .on_main(|| {
                                WINDOWS.with(|slot| {
                                    let slot = slot.borrow();
                                    let resources = slot.as_ref().unwrap();
                                    assert!(resources.window.is_layer_window());
                                    assert_eq!(resources.window.layer(), Layer::Overlay);
                                    assert_eq!(
                                        resources.window.keyboard_mode(),
                                        KeyboardMode::None
                                    );
                                    assert_eq!(resources.window.exclusive_zone(), -1);
                                    resources
                                        .webview
                                        .with_webview(|view| {
                                            let inner = view.inner();
                                            // Real GTK crossings run outside the runtime's webview borrow.
                                            gtk::glib::idle_add_local_once(move || {
                                                let enter = gtk::gdk::Event::new(
                                                    gtk::gdk::EventType::EnterNotify,
                                                );
                                                inner.emit_by_name::<bool>(
                                                    "enter-notify-event",
                                                    &[&enter],
                                                );
                                                let leave = gtk::gdk::Event::new(
                                                    gtk::gdk::EventType::LeaveNotify,
                                                );
                                                inner.emit_by_name::<bool>(
                                                    "leave-notify-event",
                                                    &[&leave],
                                                );
                                            });
                                        })
                                        .unwrap();
                                    assert!(resources.helper.is_visible());
                                    assert!(resources.helper.is_mapped());
                                    assert!(!resources.helper.accepts_focus());
                                    let button = resources
                                        .helper
                                        .child()
                                        .unwrap()
                                        .downcast::<gtk::Button>()
                                        .unwrap();
                                    for _ in 0..2 {
                                        assert_eq!(button.opacity(), 0.0);
                                        let enter =
                                            gtk::gdk::Event::new(gtk::gdk::EventType::EnterNotify);
                                        button
                                            .emit_by_name::<bool>("enter-notify-event", &[&enter]);
                                        assert_eq!(button.opacity(), 1.0);
                                        let leave =
                                            gtk::gdk::Event::new(gtk::gdk::EventType::LeaveNotify);
                                        button
                                            .emit_by_name::<bool>("leave-notify-event", &[&leave]);
                                        assert_eq!(button.opacity(), 0.0);
                                        assert!(resources.helper.is_mapped());
                                    }
                                    Ok(())
                                })
                            })
                            .unwrap();
                        for expected in ["true", "false"] {
                            assert_eq!(
                                pointer_received
                                    .recv_timeout(Duration::from_secs(2))
                                    .unwrap(),
                                expected
                            );
                        }
                        pointer_window.unlisten(listener);
                        service
                            .on_main(|| {
                                let button = WINDOWS.with(|slot| {
                                    slot.borrow()
                                        .as_ref()
                                        .unwrap()
                                        .helper
                                        .child()
                                        .unwrap()
                                        .downcast::<gtk::Button>()
                                        .unwrap()
                                });
                                button.emit_clicked();
                                Ok(())
                            })
                            .unwrap();
                        assert!(!service.snapshot().locked);
                        std::thread::sleep(Duration::from_millis(250));
                        assert!(!service.hide(&app).unwrap().visible);
                        assert!(!service.window_ready().unwrap().visible);
                        assert!(service.show(&app, Some(16)).unwrap().visible);
                        service.window_ready().unwrap();
                        std::thread::sleep(Duration::from_millis(250));
                        service.hide(&app).unwrap();
                    }));
                    finished.send(checked.is_ok()).unwrap();
                    app.exit(0);
                });
                Ok(())
            })
            .build(tauri::generate_context!())
            .unwrap();
        assert_eq!(app.run_return(|_, _| {}), 0);
        assert!(result.recv_timeout(Duration::from_secs(30)).unwrap());
    }
}
