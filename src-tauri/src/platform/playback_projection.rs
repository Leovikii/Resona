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
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
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
                let mut details = None;
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
                        let loaded = snapshot
                            .path
                            .as_deref()
                            .map(|path| metadata.load(std::path::Path::new(path)));
                        artwork = loaded.as_ref().and_then(|loaded| loaded.artwork.clone());
                        details = loaded.map(|loaded| loaded.details);
                    }
                    let (title, artist, album) = projected_tags(&snapshot, details.as_ref());
                    let projection = NativePlaybackSnapshot {
                        playback: snapshot,
                        artwork: artwork.clone(),
                        title,
                        artist,
                        album,
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

fn projected_tags(
    snapshot: &PlaybackSnapshot,
    details: Option<&crate::metadata::TrackDetails>,
) -> (Option<String>, Option<String>, Option<String>) {
    let cue = snapshot
        .current_item_id
        .and_then(|id| snapshot.queue.iter().find(|item| item.id == id))
        .and_then(|item| item.cue.as_ref());
    (
        cue.and_then(|cue| cue.title.clone())
            .or_else(|| details.and_then(|details| details.title.clone())),
        cue.and_then(|cue| cue.performer.clone())
            .or_else(|| details.and_then(|details| details.artist.clone())),
        cue.and_then(|cue| cue.album.clone())
            .or_else(|| details.and_then(|details| details.album.clone())),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cue_tags_follow_the_current_logical_track() {
        let mut snapshot = PlaybackSnapshot {
            current_item_id: Some(7),
            queue: vec![crate::playback::QueueItemSnapshot {
                id: 7,
                path: "album.flac".into(),
                display_name: "Track 2".into(),
                duration_ms: Some(1000),
                status: crate::playback::QueueItemStatus::Playing,
                error: None,
                cue: Some(crate::media_source::CueTrackSource {
                    cue_path: "album.cue".into(),
                    track_number: 2,
                    title: Some("Logical title".into()),
                    performer: Some("Performer".into()),
                    album: Some("Album".into()),
                    start_ms: 1000,
                    end_ms: Some(2000),
                }),
            }],
            ..Default::default()
        };
        assert_eq!(
            projected_tags(&snapshot, None),
            (
                Some("Logical title".into()),
                Some("Performer".into()),
                Some("Album".into())
            )
        );
        snapshot.current_item_id = Some(8);
        assert_eq!(projected_tags(&snapshot, None), (None, None, None));
    }
}
