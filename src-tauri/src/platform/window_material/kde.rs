// SPDX-License-Identifier: GPL-3.0-only
// GTK owns the display and surface. Generated protocol bindings borrow those
// objects on the GTK thread; they never open a second Wayland connection.

use super::WindowMaterial;
use glib::translate::ToGlibPtr;
use gtk::{gdk, glib, prelude::*};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    sync::mpsc,
    time::Duration,
};
use tauri::{Emitter, Manager, Runtime, WebviewWindow};
use wayland_client::{
    backend::{Backend, ObjectId},
    protocol::{
        wl_callback, wl_compositor::WlCompositor, wl_region::WlRegion, wl_registry,
        wl_surface::WlSurface,
    },
    Connection, Dispatch, EventQueue, Proxy, QueueHandle,
};
use wayland_protocols::ext::background_effect::v1::client::{
    ext_background_effect_manager_v1::{self, ExtBackgroundEffectManagerV1},
    ext_background_effect_surface_v1::ExtBackgroundEffectSurfaceV1,
};
use wayland_protocols_plasma::blur::client::{
    org_kde_kwin_blur::OrgKdeKwinBlur, org_kde_kwin_blur_manager::OrgKdeKwinBlurManager,
};

mod shadow;

#[derive(Clone, Copy, Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

thread_local! {
    static SESSIONS: RefCell<HashMap<String, Rc<RefCell<Session>>>> = RefCell::new(HashMap::new());
}

struct Session {
    queue: EventQueue<BlurState>,
    state: BlurState,
}
struct BlurState {
    connection: Connection,
    registry: wl_registry::WlRegistry,
    compositor: WlCompositor,
    surface: WlSurface,
    manager: Option<(u32, OrgKdeKwinBlurManager)>,
    blur: Option<OrgKdeKwinBlur>,
    effect_manager: Option<(u32, ExtBackgroundEffectManagerV1)>,
    effect: Option<ExtBackgroundEffectSurfaceV1>,
    blur_capability: bool,
    capabilities_received: bool,
    shadow: shadow::Shadow,
    normal: bool,
    content: ContentRect,
    size: (i32, i32),
    ready: bool,
    waiting: Vec<mpsc::SyncSender<Result<WindowMaterial, String>>>,
    notify: Box<dyn Fn(WindowMaterial)>,
}

impl BlurState {
    fn material(&self) -> WindowMaterial {
        if (self.effect_manager.is_some() && self.blur_capability && self.effect.is_some())
            || (self.effect_manager.is_none() && self.blur.is_some())
        {
            WindowMaterial::KdeBlur
        } else {
            WindowMaterial::Solid
        }
    }
    fn update(&mut self, qh: &QueueHandle<Self>) {
        self.shadow.update(&self.surface, self.normal, qh);
        if let Some((_, manager)) = &self.effect_manager {
            if let Some(blur) = self.blur.take() {
                if let Some((_, legacy)) = &self.manager {
                    legacy.unset(&self.surface);
                }
                blur.release();
            }
            if self.effect.is_none() {
                self.effect = Some(manager.get_background_effect(&self.surface, qh, ()));
            }
        }
        if self.effect_manager.is_none() {
            if let Some((_, manager)) = &self.manager {
                if self.blur.is_none() {
                    self.blur = Some(manager.create(&self.surface, qh, ()));
                }
            }
        }
        if self.blur.is_some() || self.effect.is_some() {
            let region = self.compositor.create_region(qh, ());
            for (x, y, w, h) in shell_regions(self.size, self.content) {
                if w > 0 && h > 0 {
                    region.add(x, y, w, h);
                }
            }
            if let Some(effect) = &self.effect {
                effect.set_blur_region(if self.blur_capability {
                    Some(&region)
                } else {
                    None
                });
            }
            if let Some(blur) = &self.blur {
                blur.set_region(Some(&region));
                blur.commit();
            }
            region.destroy();
        }
        if let Err(error) = self.connection.flush() {
            log::warn!("KDE blur flush failed: {error}");
        }
        if self.ready && (self.effect_manager.is_none() || self.capabilities_received) {
            let material = self.material();
            (self.notify)(material);
            for sender in self.waiting.drain(..) {
                let _ = sender.send(Ok(material));
            }
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for BlurState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } if matches!(interface.as_str(), "org_kde_kwin_shadow_manager" | "wl_shm") => {
                state.shadow.bind(registry, name, &interface, version, qh);
                state.update(qh);
            }
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } if interface == "ext_background_effect_manager_v1" => {
                state.effect_manager = Some((name, registry.bind(name, version.min(1), qh, ())));
                state.update(qh);
            }
            wl_registry::Event::GlobalRemove { name }
                if state
                    .effect_manager
                    .as_ref()
                    .is_some_and(|(id, _)| *id == name) =>
            {
                if let Some(effect) = state.effect.take() {
                    effect.destroy();
                }
                if let Some((_, manager)) = state.effect_manager.take() {
                    manager.destroy();
                }
                state.blur_capability = false;
                state.capabilities_received = false;
                state.update(qh);
            }
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } if interface == "org_kde_kwin_blur_manager" => {
                state.manager = Some((name, registry.bind(name, version.min(1), qh, ())));
                state.update(qh);
            }
            wl_registry::Event::GlobalRemove { name }
                if state.manager.as_ref().is_some_and(|(id, _)| *id == name) =>
            {
                if let Some(blur) = state.blur.take() {
                    blur.release();
                }
                state.manager = None;
                state.update(qh);
                log::info!("KWin blur capability removed; using solid shell");
            }
            wl_registry::Event::GlobalRemove { name } => {
                state.shadow.remove_global(name, &state.surface);
                state.update(qh);
            }
            _ => {}
        }
    }
}
impl Dispatch<ExtBackgroundEffectManagerV1, ()> for BlurState {
    fn event(
        state: &mut Self,
        _: &ExtBackgroundEffectManagerV1,
        event: ext_background_effect_manager_v1::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let ext_background_effect_manager_v1::Event::Capabilities { flags } = event {
            state.blur_capability = matches!(flags, wayland_client::WEnum::Value(flags) if flags.contains(ext_background_effect_manager_v1::Capability::Blur));
            state.capabilities_received = true;
            state.update(qh);
        }
    }
}
wayland_client::delegate_noop!(BlurState: ignore ExtBackgroundEffectSurfaceV1);
impl Dispatch<wl_callback::WlCallback, ()> for BlurState {
    fn event(
        state: &mut Self,
        _: &wl_callback::WlCallback,
        _: wl_callback::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        state.ready = true;
        state.update(qh);
        log::info!("KDE shell material: {:?}", state.material());
    }
}
wayland_client::delegate_noop!(BlurState: ignore OrgKdeKwinBlurManager);
wayland_client::delegate_noop!(BlurState: ignore OrgKdeKwinBlur);
wayland_client::delegate_noop!(BlurState: ignore WlRegion);
wayland_client::delegate_noop!(BlurState: ignore wayland_client::protocol::wl_shm::WlShm);
wayland_client::delegate_noop!(BlurState: ignore wayland_client::protocol::wl_shm_pool::WlShmPool);
wayland_client::delegate_noop!(BlurState: ignore wayland_client::protocol::wl_buffer::WlBuffer);
wayland_client::delegate_noop!(BlurState: ignore wayland_protocols_plasma::shadow::client::org_kde_kwin_shadow_manager::OrgKdeKwinShadowManager);
wayland_client::delegate_noop!(BlurState: ignore wayland_protocols_plasma::shadow::client::org_kde_kwin_shadow::OrgKdeKwinShadow);

pub(super) fn sync<R: Runtime>(window: &WebviewWindow<R>) -> Result<WindowMaterial, String> {
    if !std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .split(':')
        .any(|name| name.eq_ignore_ascii_case("KDE"))
    {
        return Ok(WindowMaterial::Solid);
    }
    let (sender, receiver) = mpsc::sync_channel(1);
    let native = window.clone();
    window
        .run_on_main_thread(move || {
            let result = SESSIONS.with(|sessions| {
                if let Some(session) = sessions.borrow().get(native.label()) {
                    let mut session = session.borrow_mut();
                    if session.state.ready
                        && (session.state.effect_manager.is_none()
                            || session.state.capabilities_received)
                    {
                        let _ = sender.send(Ok(session.state.material()));
                    } else {
                        session.state.waiting.push(sender.clone());
                    }
                    return Ok(());
                }
                initialize(&native, sender.clone())
            });
            if let Err(error) = result {
                let _ = sender.send(Err(error));
            }
        })
        .map_err(|error| error.to_string())?;
    match receiver.recv_timeout(Duration::from_secs(3)) {
        Ok(Ok(material)) => Ok(material),
        result => {
            log::warn!("KDE blur unavailable; using solid shell: {result:?}");
            Ok(WindowMaterial::Solid)
        }
    }
}

fn start_session<R: Runtime>(
    window: &WebviewWindow<R>,
    sender: mpsc::SyncSender<Result<WindowMaterial, String>>,
    content: Option<ContentRect>,
) -> Result<(), String> {
    let native = window.gtk_window().map_err(|error| error.to_string())?;
    native.realize();
    let surface = gtk::prelude::WidgetExt::window(&native).ok_or("GTK surface unavailable")?;
    let display = surface.display();
    if display.type_().name() != "GdkWaylandDisplay" {
        return Err("GTK is not using Wayland".into());
    }
    // SAFETY: borrowed GDK objects are used only during this surface's GTK lifetime.
    // Unmap cleanup never sends a request referencing GTK's destroyed surface.
    let (connection, surface, compositor) = unsafe {
        let display_ptr: *mut gdk::ffi::GdkDisplay = display.to_glib_none().0;
        let surface_ptr: *mut gdk::ffi::GdkWindow = surface.to_glib_none().0;
        let raw_display = gdk_wayland_sys::gdk_wayland_display_get_wl_display(display_ptr.cast());
        let raw_surface = gdk_wayland_sys::gdk_wayland_window_get_wl_surface(surface_ptr.cast());
        let raw_compositor =
            gdk_wayland_sys::gdk_wayland_display_get_wl_compositor(display_ptr.cast());
        if raw_display.is_null() || raw_surface.is_null() || raw_compositor.is_null() {
            return Err("Wayland handles unavailable".into());
        }
        let connection =
            Connection::from_backend(Backend::from_foreign_display(raw_display.cast()));
        let surface = WlSurface::from_id(
            &connection,
            ObjectId::from_ptr(WlSurface::interface(), raw_surface.cast())
                .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let compositor = WlCompositor::from_id(
            &connection,
            ObjectId::from_ptr(WlCompositor::interface(), raw_compositor.cast())
                .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        (connection, surface, compositor)
    };
    let queue = connection.new_event_queue();
    let qh = queue.handle();
    let registry = connection.display().get_registry(&qh, ());
    connection.display().sync(&qh, ());
    connection.flush().map_err(|error| error.to_string())?;
    let size = (native.allocated_width(), native.allocated_height());
    let notify_window = window.clone();
    let session = Rc::new(RefCell::new(Session {
        queue,
        state: BlurState {
            connection,
            registry,
            surface,
            compositor,
            manager: None,
            blur: None,
            effect_manager: None,
            effect: None,
            blur_capability: false,
            capabilities_received: false,
            shadow: shadow::Shadow::default(),
            normal: !native.is_maximized()
                && !native
                    .window()
                    .is_some_and(|window| window.state().contains(gdk::WindowState::FULLSCREEN)),
            content: content.unwrap_or(ContentRect {
                x: 0,
                y: 48,
                width: size.0,
                height: (size.1 - 48).max(0),
            }),
            size,
            ready: false,
            waiting: vec![sender],
            notify: Box::new(move |material| {
                // GTK owns surface commits; schedule a frame instead of committing
                // its borrowed wl_surface while GDK may have pending buffer state.
                if let Ok(window) = notify_window.gtk_window() {
                    window.queue_draw();
                }
                if let Err(error) = notify_window.emit_to(
                    notify_window.label(),
                    "resona://window-material",
                    material,
                ) {
                    log::warn!("material notification failed: {error}");
                }
            }),
        },
    }));
    let label = window.label().to_owned();
    SESSIONS.with(|sessions| {
        sessions.borrow_mut().insert(label.clone(), session.clone());
    });
    let weak = Rc::downgrade(&session);
    // ponytail: GDK reads this shared socket and queues our protocol events. A
    // bounded 5 Hz pending-queue drain avoids a second reader/thread and blocking
    // roundtrips on GTK. Replace with a custom GLib source if measurements warrant.
    glib::timeout_add_local(Duration::from_millis(200), move || {
        let Some(session) = weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        let mut session = session.borrow_mut();
        let Session { queue, state } = &mut *session;
        if let Err(error) = queue.dispatch_pending(state) {
            log::warn!("KDE protocol dispatch failed; using solid shell: {error}");
            (state.notify)(WindowMaterial::Solid);
            return glib::ControlFlow::Break;
        }
        glib::ControlFlow::Continue
    });
    Ok(())
}

fn initialize<R: Runtime>(
    window: &WebviewWindow<R>,
    sender: mpsc::SyncSender<Result<WindowMaterial, String>>,
) -> Result<(), String> {
    start_session(window, sender, None)?;
    let native = window.gtk_window().map_err(|error| error.to_string())?;
    let label = window.label().to_owned();
    let resized_label = label.clone();
    native.connect_size_allocate(move |_, rect| {
        SESSIONS.with(|sessions| {
            if let Some(session) = sessions.borrow().get(&resized_label) {
                let mut session = session.borrow_mut();
                session.state.size = (rect.width(), rect.height());
                let qh = session.queue.handle();
                session.state.update(&qh);
            }
        });
    });
    let state_label = label.clone();
    native.connect_window_state_event(move |native, event| {
        SESSIONS.with(|sessions| {
            if let Some(session) = sessions.borrow().get(&state_label) {
                let mut session = session.borrow_mut();
                session.state.normal = !event
                    .new_window_state()
                    .intersects(gdk::WindowState::MAXIMIZED | gdk::WindowState::FULLSCREEN);
                let qh = session.queue.handle();
                session.state.update(&qh);
                native.queue_draw();
            }
        });
        glib::Propagation::Proceed
    });
    let content = Rc::new(Cell::new(None));
    let unmapped_content = content.clone();
    let unmapped_label = label.clone();
    native.connect_unmap(move |_| {
        if let Some(rect) = cleanup(&unmapped_label) {
            unmapped_content.set(Some(rect));
        }
    });
    let unrealized_content = content.clone();
    let unrealized_label = label.clone();
    native.connect_unrealize(move |_| {
        if let Some(rect) = cleanup(&unrealized_label) {
            unrealized_content.set(Some(rect));
        }
    });
    let app = window.app_handle().clone();
    native.connect_map(move |_| {
        if SESSIONS.with(|sessions| sessions.borrow().contains_key(&label)) {
            return;
        }
        if let Some(window) = app.get_webview_window(&label) {
            let (sender, _) = mpsc::sync_channel(1);
            if let Err(error) = start_session(&window, sender, content.get()) {
                log::warn!("KDE material remap failed: {error}");
            }
        }
    });
    Ok(())
}

fn cleanup(label: &str) -> Option<ContentRect> {
    let session = SESSIONS.with(|sessions| sessions.borrow_mut().remove(label));
    if let Some(session) = session {
        let mut session = session.borrow_mut();
        let state = &mut session.state;
        // GDK destroys wl_surface during unmap, before these GTK signals.
        state.shadow.clear();
        if let Some(blur) = state.blur.take() {
            blur.release();
        }
        if let Some(effect) = state.effect.take() {
            effect.destroy();
        }
        if let Some((_, manager)) = state.manager.take() {
            state
                .connection
                .backend()
                .destroy_object(&manager.id())
                .unwrap_or_else(|error| {
                    log::warn!("KDE legacy blur manager cleanup failed: {error}");
                });
        }
        if let Some((_, manager)) = state.effect_manager.take() {
            manager.destroy();
        }
        if let Err(error) = state
            .connection
            .backend()
            .destroy_object(&state.registry.id())
        {
            log::warn!("KDE registry cleanup failed: {error}");
        }
        if let Err(error) = state.connection.flush() {
            log::warn!("KDE material cleanup failed: {error}");
        }
        return Some(state.content);
    }
    None
}

pub fn set_content_rect<R: Runtime>(
    window: &WebviewWindow<R>,
    rect: ContentRect,
) -> Result<(), String> {
    if !matches!(window.label(), "main" | "audio-compression")
        || [rect.x, rect.y, rect.width, rect.height]
            .iter()
            .any(|value| !(0..=32768).contains(value))
    {
        return Err("Invalid blur content rectangle".into());
    }
    let label = window.label().to_owned();
    window
        .run_on_main_thread(move || {
            SESSIONS.with(|sessions| {
                if let Some(session) = sessions.borrow().get(&label) {
                    let mut session = session.borrow_mut();
                    session.state.content = rect;
                    let qh = session.queue.handle();
                    session.state.update(&qh);
                }
            })
        })
        .map_err(|error| error.to_string())
}

fn shell_regions((width, height): (i32, i32), content: ContentRect) -> [(i32, i32, i32, i32); 4] {
    let x = content.x.clamp(0, width.max(0));
    let y = content.y.clamp(0, height.max(0));
    let right = (x + content.width).clamp(x, width.max(x));
    let bottom = (y + content.height).clamp(y, height.max(y));
    [
        (0, 0, width, y),
        (0, y, x, bottom - y),
        (right, y, width - right, bottom - y),
        (0, bottom, width, height - bottom),
    ]
    .map(|(x, y, w, h)| {
        // Keep blur one surface-local logical pixel off the edge to avoid KWin move trails.
        let left = x.max(1);
        let top = y.max(1);
        let right = (x + w).min(width - 1);
        let bottom = (y + h).min(height - 1);
        (left, top, (right - left).max(0), (bottom - top).max(0))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires an isolated KDE Wayland session; checks protocol lifecycle, not visual appearance"]
    fn native_shadow_lifecycle() {
        use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

        fn wait_shadow(app: &tauri::AppHandle, label: &str, expected: Option<bool>) {
            let deadline = std::time::Instant::now() + Duration::from_secs(5);
            loop {
                let (done, received) = mpsc::sync_channel(1);
                let label = label.to_owned();
                app.run_on_main_thread(move || {
                    let actual = SESSIONS.with(|sessions| {
                        sessions.borrow().get(&label).map(|session| {
                            let session = session.borrow();
                            (session.state.shadow.attached(), session.state.normal)
                        })
                    });
                    done.send(actual).unwrap();
                })
                .unwrap();
                let actual = received.recv_timeout(Duration::from_secs(5)).unwrap();
                if actual == expected.map(|attached| (attached, attached)) {
                    return;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "shadow state {actual:?}, expected {expected:?}"
                );
                std::thread::sleep(Duration::from_millis(50));
            }
        }

        crate::platform::initialize_process();
        let mut context = tauri::generate_context!();
        context.config_mut().app.windows.clear();
        let (finished, result) = mpsc::sync_channel(1);
        let app = tauri::Builder::default()
            .any_thread()
            .setup(move |app| {
                let app = app.handle().clone();
                std::thread::spawn(move || {
                    let checked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        for label in ["main", "audio-compression", "audio-compression"] {
                            let handle = app.clone();
                            let (done, received) = mpsc::sync_channel(1);
                            app.run_on_main_thread(move || {
                                let window = WebviewWindowBuilder::new(
                                    &handle,
                                    label,
                                    WebviewUrl::External("about:blank".parse().unwrap()),
                                )
                                .decorations(false)
                                .transparent(true)
                                .inner_size(720.0, 500.0)
                                .build();
                                done.send(window).unwrap();
                            })
                            .unwrap();
                            let window = received.recv_timeout(Duration::from_secs(5)).unwrap().unwrap();
                            assert_eq!(sync(&window).unwrap(), WindowMaterial::KdeBlur);
                            wait_shadow(&app, label, Some(true));
                            window.maximize().unwrap();
                            wait_shadow(&app, label, Some(false));
                            window.unmaximize().unwrap();
                            wait_shadow(&app, label, Some(true));
                            window.set_fullscreen(true).unwrap();
                            wait_shadow(&app, label, Some(false));
                            window.set_fullscreen(false).unwrap();
                            wait_shadow(&app, label, Some(true));
                            set_content_rect(&window, ContentRect { x: 12, y: 48, width: 696, height: 356 }).unwrap();
                            window.hide().unwrap();
                            wait_shadow(&app, label, None);
                            assert!(app.get_webview_window(label).is_some());
                            window.show().unwrap();
                            wait_shadow(&app, label, Some(true));
                            let (done, received) = mpsc::sync_channel(1);
                            app.run_on_main_thread(move || {
                                done.send(SESSIONS.with(|sessions| sessions.borrow().get(label).unwrap().borrow().state.content.width)).unwrap();
                            }).unwrap();
                            assert_eq!(received.recv_timeout(Duration::from_secs(5)).unwrap(), 696);
                            if label == "audio-compression" {
                                window.destroy().unwrap();
                                wait_shadow(&app, label, None);
                            }
                        }
                        app.get_webview_window("main").unwrap().hide().unwrap();
                        wait_shadow(&app, "main", None);
                        println!("PASS: main/compression shadow attach, maximize/fullscreen removal, hide/show with content preservation, and compression recreation");
                    }));
                    finished.send(checked).unwrap();
                    app.exit(0);
                });
                Ok(())
            })
            .build(context)
            .expect("build shadow lifecycle test");
        assert_eq!(app.run_return(|_, _| {}), 0);
        result
            .recv_timeout(Duration::from_secs(40))
            .unwrap()
            .unwrap();
    }

    #[test]
    fn blur_region_excludes_content_and_clamps_stale_geometry() {
        assert_eq!(
            shell_regions(
                (1080, 700),
                ContentRect {
                    x: 208,
                    y: 48,
                    width: 860,
                    height: 548
                }
            ),
            [
                (1, 1, 1078, 47),
                (1, 48, 207, 548),
                (1068, 48, 11, 548),
                (1, 596, 1078, 103)
            ]
        );
        assert_eq!(
            shell_regions(
                (360, 600),
                ContentRect {
                    x: 8,
                    y: 84,
                    width: 900,
                    height: 900
                }
            ),
            [
                (1, 1, 358, 83),
                (1, 84, 7, 515),
                (360, 84, 0, 515),
                (1, 600, 358, 0)
            ]
        );
    }

    #[test]
    fn blur_stays_inside_perimeter_and_outside_content_across_layouts() {
        for ((width, height), content) in [
            (
                (420, 720),
                ContentRect {
                    x: 8,
                    y: 84,
                    width: 404,
                    height: 516,
                },
            ),
            (
                (720, 500),
                ContentRect {
                    x: 0,
                    y: 141,
                    width: 720,
                    height: 359,
                },
            ),
            ((1080, 700), ContentRect::default()),
            ((0, 0), ContentRect::default()),
            ((1, 1), ContentRect::default()),
            ((2, 2), ContentRect::default()),
        ] {
            let regions = shell_regions((width, height), content);
            for (x, y, w, h) in regions {
                assert!(w >= 0 && h >= 0);
                if w == 0 || h == 0 {
                    continue;
                }
                assert!(x >= 1 && y >= 1 && x + w < width && y + h < height);
                assert!(
                    x + w <= content.x
                        || x >= content.x + content.width
                        || y + h <= content.y
                        || y >= content.y + content.height
                );
            }
            // Check coverage too: excluding the content must leave no interior gaps.
            for y in 1..height - 1 {
                for x in 1..width - 1 {
                    let in_content = x >= content.x
                        && x < content.x + content.width
                        && y >= content.y
                        && y < content.y + content.height;
                    let blurred = regions.iter().any(|&(left, top, w, h)| {
                        x >= left && x < left + w && y >= top && y < top + h
                    });
                    assert_eq!(blurred, !in_content, "pixel ({x}, {y}) in {width}x{height}");
                }
            }
        }
    }
}
