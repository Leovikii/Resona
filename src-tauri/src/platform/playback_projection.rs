// SPDX-License-Identifier: GPL-3.0-only

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::playback::{PlaybackEngine, PlaybackSnapshot, RodioPlaybackEngine};

const PUBLISH_INTERVAL: Duration = Duration::from_millis(500);

enum ProjectionCommand {
    Shutdown,
}

pub struct NativePlaybackProjection {
    subscribers: Arc<Mutex<Vec<Sender<PlaybackSnapshot>>>>,
    commands: Sender<ProjectionCommand>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl NativePlaybackProjection {
    pub fn start(engine: Arc<RodioPlaybackEngine>) -> Result<Self, String> {
        let subscribers = Arc::new(Mutex::new(Vec::<Sender<PlaybackSnapshot>>::new()));
        let subscribers_for_worker = Arc::clone(&subscribers);
        let (commands, receiver) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("resona-native-playback-projection".to_owned())
            .spawn(move || loop {
                match receiver.recv_timeout(PUBLISH_INTERVAL) {
                    Ok(ProjectionCommand::Shutdown) | Err(RecvTimeoutError::Disconnected) => break,
                    Err(RecvTimeoutError::Timeout) => {}
                }
                let Ok(snapshot) = engine.snapshot() else {
                    continue;
                };
                subscribers_for_worker
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .retain(|subscriber| subscriber.send(snapshot.clone()).is_ok());
            })
            .map_err(|error| format!("failed to start native playback projection: {error}"))?;
        Ok(Self {
            subscribers,
            commands,
            worker: Mutex::new(Some(worker)),
        })
    }

    pub fn subscribe(&self) -> Receiver<PlaybackSnapshot> {
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
