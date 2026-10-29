// SPDX-License-Identifier: GPL-3.0-only

use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::{DragDropEvent, Emitter, Runtime, WebviewWindow, WindowEvent};

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum FileDropEvent {
    Enter {
        paths: Vec<String>,
        position: Position,
    },
    Over {
        position: Position,
    },
    Drop {
        paths: Vec<String>,
        position: Position,
    },
    Leave,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
struct Position {
    x: f64,
    y: f64,
}

fn position(point: &tauri::PhysicalPosition<f64>, scale: f64) -> Position {
    // Locked Wry's GTK backend reports widget coordinates as PhysicalPosition.
    let divisor = if cfg!(target_os = "linux") {
        1.0
    } else {
        scale
    };
    Position {
        x: point.x / divisor,
        y: point.y / divisor,
    }
}

pub fn install<R: Runtime>(window: &WebviewWindow<R>) {
    let uris: Arc<Mutex<Option<Vec<String>>>> = Arc::new(Mutex::new(None));
    #[cfg(target_os = "linux")]
    {
        use gtk::prelude::*;
        let uris = Arc::clone(&uris);
        if let Err(error) = window.with_webview(move |native| {
            // Wry percent-decodes remote URIs into PathBuf, losing #/% boundaries.
            native
                .inner()
                .connect_drag_data_received(move |_, _, _, _, data, info, _| {
                    if info == 2 {
                        *uris
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner) =
                            Some(data.uris().iter().map(ToString::to_string).collect());
                    }
                });
        }) {
            log::error!("Native file-drop URI capture unavailable: {error}");
        }
    }
    let emitter = window.clone();
    window.on_window_event(move |event| {
        let WindowEvent::DragDrop(event) = event else {
            return;
        };
        let scale = emitter.scale_factor().unwrap_or(1.0);
        let payload = {
            let mut captured = uris
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            convert_event(event, &mut captured, scale)
        };
        let Some(payload) = payload else {
            return;
        };
        if let FileDropEvent::Drop { paths, .. } = &payload {
            log::info!(
                "Native file drop received: window={}, count={}",
                emitter.label(),
                paths.len()
            );
        }
        if let Err(error) = emitter.emit_to(emitter.label(), "resona://file-drag-drop", payload) {
            log::error!("Native file drop delivery failed: {error}");
        }
    });
}

fn convert_event(
    event: &DragDropEvent,
    captured: &mut Option<Vec<String>>,
    scale: f64,
) -> Option<FileDropEvent> {
    let paths = |fallback: &[std::path::PathBuf], raw: Option<Vec<String>>| {
        raw.unwrap_or_else(|| {
            fallback
                .iter()
                .map(|path| path.to_string_lossy().into_owned())
                .collect()
        })
    };
    Some(match event {
        DragDropEvent::Enter {
            paths: fallback,
            position: point,
        } => FileDropEvent::Enter {
            paths: paths(fallback, captured.clone()),
            position: position(point, scale),
        },
        DragDropEvent::Over { position: point } => FileDropEvent::Over {
            position: position(point, scale),
        },
        DragDropEvent::Drop {
            paths: fallback,
            position: point,
        } => FileDropEvent::Drop {
            paths: paths(fallback, captured.take()),
            position: position(point, scale),
        },
        DragDropEvent::Leave => {
            *captured = None;
            FileDropEvent::Leave
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_drop_preserves_raw_uri_and_clears_it_after_drop_or_cancel() {
        let uri = "nfs://host/export/中文%20%23%2520.flac";
        let mut captured = Some(vec![uri.to_owned()]);
        let event = DragDropEvent::Drop {
            paths: vec!["nfs://host/export/中文 #%20.flac".into()],
            position: tauri::PhysicalPosition::new(300.0, 150.0),
        };
        let payload = serde_json::to_value(convert_event(&event, &mut captured, 1.5)).unwrap();
        assert_eq!(payload["type"], "drop");
        assert_eq!(payload["paths"][0], uri);
        assert!(captured.is_none());
        let x = if cfg!(target_os = "linux") {
            300.0
        } else {
            200.0
        };
        assert_eq!(payload["position"]["x"], x);
        captured = Some(vec![uri.to_owned()]);
        convert_event(&DragDropEvent::Leave, &mut captured, 1.5);
        let local = DragDropEvent::Drop {
            paths: vec!["local # %20.flac".into()],
            position: tauri::PhysicalPosition::new(0.0, 0.0),
        };
        let payload = serde_json::to_value(convert_event(&local, &mut captured, 1.5)).unwrap();
        assert_eq!(payload["paths"][0], "local # %20.flac");
    }
}
