// SPDX-License-Identifier: GPL-3.0-only

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::metadata::{Artwork, MetadataService};
use crate::playback::{PlaybackEngine, PlaybackSnapshot, RodioPlaybackEngine};

const PUBLISH_INTERVAL: Duration = Duration::from_millis(500);

enum ProjectionCommand {
    Shutdown,
}

pub struct NativePlaybackProjection {
    subscribers: Arc<Mutex<Vec<Sender<NativePlaybackSnapshot>>>>,
    commands: Sender<ProjectionCommand>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

#[derive(Clone)]
pub struct NativePlaybackSnapshot {
    pub playback: PlaybackSnapshot,
    pub artwork: Option<Arc<Artwork>>,
}

impl NativePlaybackProjection {
    pub fn start(
        engine: Arc<RodioPlaybackEngine>,
        metadata: Arc<MetadataService>,
    ) -> Result<Self, String> {
        let subscribers = Arc::new(Mutex::new(Vec::<Sender<NativePlaybackSnapshot>>::new()));
        let subscribers_for_worker = Arc::clone(&subscribers);
        let (commands, receiver) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("resona-native-playback-projection".to_owned())
            .spawn(move || {
                let mut artwork_path = None;
                let mut artwork = None;
                loop {
                    match receiver.recv_timeout(PUBLISH_INTERVAL) {
                        Ok(ProjectionCommand::Shutdown) | Err(RecvTimeoutError::Disconnected) => {
                            break
                        }
                        Err(RecvTimeoutError::Timeout) => {}
                    }
                    let Ok(snapshot) = engine.snapshot() else {
                        continue;
                    };
                    if snapshot.path != artwork_path {
                        artwork_path.clone_from(&snapshot.path);
                        artwork = snapshot
                            .path
                            .as_deref()
                            .and_then(|path| metadata.load(std::path::Path::new(path)).artwork);
                    }
                    let projection = NativePlaybackSnapshot {
                        playback: snapshot,
                        artwork: artwork.clone(),
                    };
                    subscribers_for_worker
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .retain(|subscriber| subscriber.send(projection.clone()).is_ok());
                }
            })
            .map_err(|error| format!("failed to start native playback projection: {error}"))?;
        Ok(Self {
            subscribers,
            commands,
            worker: Mutex::new(Some(worker)),
        })
    }

    pub fn subscribe(&self) -> Receiver<NativePlaybackSnapshot> {
        let (sender, receiver) = mpsc::channel();
        self.subscribers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(sender);
        receiver
    }
}

impl Drop for NativePlaybackProjection {
    fn drop(&mut self) {
        let _ = self.commands.send(ProjectionCommand::Shutdown);
        if let Ok(mut worker) = self.worker.lock() {
            if let Some(worker) = worker.take() {
                let _ = worker.join();
            }
        }
    }
}
