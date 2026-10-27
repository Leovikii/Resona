// SPDX-License-Identifier: GPL-3.0-only

use tauri::{LogicalSize, PhysicalPosition, Runtime, WebviewWindow};

pub fn observe_size<R: Runtime>(
    window: &WebviewWindow<R>,
    observe: impl Fn(LogicalSize<u32>, bool) + Send + Sync + 'static,
) -> tauri::Result<()> {
    #[cfg(target_os = "linux")]
    {
        query_gtk(window, move |native| {
            use gtk::prelude::*;
            let content = native.child()?;
            let native = native.downgrade();
            content.connect_size_allocate(move |_, bounds| {
                if let Some(native) = native.upgrade() {
                    observe(
                        LogicalSize::new(
                            bounds.width().max(1) as u32,
                            bounds.height().max(1) as u32,
                        ),
                        native.is_maximized(),
                    );
                }
            });
            Some(())
        })?
        .ok_or(tauri::Error::InvalidWindowHandle)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (window, observe);
        Ok(())
    }
}

pub fn resize<R: Runtime>(
    window: &WebviewWindow<R>,
    size: LogicalSize<u32>,
    minimum: LogicalSize<u32>,
) -> tauri::Result<()> {
    #[cfg(target_os = "linux")]
    {
        query_gtk(window, move |native| {
            use gtk::prelude::*;
            // The child owns the minimum so GTK can account for changing CSD margins.
            native.set_geometry_hints(None::<&gtk::Widget>, None, gtk::gdk::WindowHints::empty());
            if let Some(content) = native.child() {
                content.set_size_request(minimum.width as i32, minimum.height as i32);
            }
            if native.is_maximized() {
                native.unmaximize();
            }
            native.resize(size.width as i32, size.height as i32);
        })
    }
    #[cfg(not(target_os = "linux"))]
    {
        if window.is_maximized()? {
            window.unmaximize()?;
        }
        window.set_min_size(Some(minimum))?;
        window.set_size(size)
    }
}

pub fn inner_size<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<LogicalSize<u32>> {
    #[cfg(target_os = "linux")]
    {
        query_gtk(window, |native| {
            use gtk::prelude::*;
            // Tao's configure-event cache includes GTK's titlebar and shadow margins.
            if native.is_mapped() {
                if let Some(content) = native.child() {
                    let bounds = content.allocation();
                    return LogicalSize::new(
                        bounds.width().max(1) as u32,
                        bounds.height().max(1) as u32,
                    );
                }
            }
            let (width, height) = native.size();
            LogicalSize::new(width.max(1) as u32, height.max(1) as u32)
        })
    }
    #[cfg(not(target_os = "linux"))]
    Ok(window.inner_size()?.to_logical(window.scale_factor()?))
}

pub fn max_inner_size<R: Runtime>(
    window: &WebviewWindow<R>,
) -> tauri::Result<Option<LogicalSize<u32>>> {
    #[cfg(target_os = "linux")]
    {
        query_gtk(window, |native| {
            use gtk::prelude::*;
            let monitor = native
                .window()
                .and_then(|surface| native.display().monitor_at_window(&surface))
                .or_else(|| native.display().primary_monitor())?;
            let area = monitor.workarea();
            let title_height = native.titlebar().map_or(0, |bar| bar.preferred_height().1);
            let border = native.style_context().border(gtk::StateFlags::NORMAL);
            Some(LogicalSize::new(
                (area.width() - i32::from(border.left) - i32::from(border.right)).max(1) as u32,
                (area.height() - title_height - i32::from(border.top) - i32::from(border.bottom))
                    .max(1) as u32,
            ))
        })
    }
    #[cfg(not(target_os = "linux"))]
    {
        let Some(monitor) = window.current_monitor()?.or(window.primary_monitor()?) else {
            return Ok(None);
        };
        let inner = window.inner_size()?;
        let outer = window.outer_size()?;
        let area = monitor.work_area();
        Ok(Some(
            tauri::PhysicalSize::new(
                area.size
                    .width
                    .saturating_sub(outer.width.saturating_sub(inner.width))
                    .max(1),
                area.size
                    .height
                    .saturating_sub(outer.height.saturating_sub(inner.height))
                    .max(1),
            )
            .to_logical(window.scale_factor()?),
        ))
    }
}

#[cfg(target_os = "linux")]
fn query_gtk<R: Runtime, T: Send + 'static>(
    window: &WebviewWindow<R>,
    read: impl FnOnce(&gtk::ApplicationWindow) -> T + Send + 'static,
) -> tauri::Result<T> {
    let native = window.clone();
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    window.run_on_main_thread(move || {
        let value = native.gtk_window().map(|native| read(&native));
        let _ = sender.send(value);
    })?;
    receiver
        .recv()
        .map_err(|_| tauri::Error::FailedToReceiveMessage)?
}

pub fn supports_absolute_position() -> bool {
    // Wayland owns placement; asking GTK for global coordinates is unsupported.
    !cfg!(target_os = "linux")
}

pub fn position<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<PhysicalPosition<i32>> {
    if supports_absolute_position() {
        window.outer_position()
    } else {
        Ok(PhysicalPosition::new(0, 0))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn linux_leaves_global_placement_to_the_compositor() {
        assert_eq!(
            super::supports_absolute_position(),
            !cfg!(target_os = "linux")
        );
    }
}
