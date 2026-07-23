// SPDX-License-Identifier: GPL-3.0-only

#[cfg(target_os = "windows")]
mod windows_impl {
    use std::path::{Path, PathBuf};
    use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender};
    use std::sync::{Arc, Mutex, OnceLock};
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    use tauri::AppHandle;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::{
        ITaskbarList3, TaskbarList, TBPF_ERROR, TBPF_NOPROGRESS, TBPF_NORMAL, TBPF_PAUSED,
        THBF_DISABLED, THBF_ENABLED, THBN_CLICKED, THB_FLAGS, THB_ICON, THB_TOOLTIP, THUMBBUTTON,
        THUMBBUTTONMASK,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CallWindowProcW, DestroyIcon, GetWindowLongPtrW, LoadImageW, RegisterWindowMessageW,
        SetWindowLongPtrW, GWLP_WNDPROC, HICON, IMAGE_ICON, LR_LOADFROMFILE, WM_COMMAND, WNDPROC,
    };

    use crate::playback::{PlaybackEngine, PlaybackSnapshot, PlaybackStatus, RodioPlaybackEngine};

    const POLL_INTERVAL: Duration = Duration::from_millis(500);
    const PREVIOUS_BUTTON_ID: u32 = 4_001;
    const PLAY_BUTTON_ID: u32 = 4_002;
    const NEXT_BUTTON_ID: u32 = 4_003;

    #[derive(Clone, Copy)]
    enum TaskbarCommand {
        Previous,
        Toggle,
        Next,
        RecreateButtons,
        Shutdown,
    }

    pub struct TaskbarAdapter {
        app: AppHandle,
        hwnd: isize,
        commands: Sender<TaskbarCommand>,
        worker: Mutex<Option<JoinHandle<()>>>,
    }

    impl TaskbarAdapter {
        pub fn start(
            app: &AppHandle,
            hwnd: isize,
            engine: Arc<RodioPlaybackEngine>,
            resource_dir: PathBuf,
            projection: Receiver<PlaybackSnapshot>,
        ) -> Result<Self, String> {
            let (commands, receiver) = mpsc::channel();
            let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
            let worker = thread::Builder::new()
                .name("resona-taskbar".to_owned())
                .spawn(move || {
                    run_taskbar(
                        hwnd,
                        engine,
                        resource_dir,
                        receiver,
                        ready_sender,
                        projection,
                    )
                })
                .map_err(|error| format!("failed to start taskbar worker: {error}"))?;

            match ready_receiver.recv_timeout(Duration::from_secs(5)) {
                Ok(Ok(taskbar_created_message)) => {
                    if let Err(error) =
                        install_message_hook(app, hwnd, commands.clone(), taskbar_created_message)
                    {
                        let _ = commands.send(TaskbarCommand::Shutdown);
                        let _ = worker.join();
                        return Err(error);
                    }
                }
                Ok(Err(error)) => {
                    let _ = worker.join();
                    return Err(error);
                }
                Err(error) => {
                    let _ = commands.send(TaskbarCommand::Shutdown);
                    let _ = worker.join();
                    return Err(format!("taskbar worker did not initialize: {error}"));
                }
            }

            Ok(Self {
                app: app.clone(),
                hwnd,
                commands,
                worker: Mutex::new(Some(worker)),
            })
        }
    }

    impl Drop for TaskbarAdapter {
        fn drop(&mut self) {
            uninstall_message_hook(&self.app, self.hwnd);
            let _ = self.commands.send(TaskbarCommand::Shutdown);
            if let Ok(mut worker) = self.worker.lock() {
                if let Some(worker) = worker.take() {
                    let _ = worker.join();
                }
            }
        }
    }

    fn run_taskbar(
        hwnd: isize,
        engine: Arc<RodioPlaybackEngine>,
        resource_dir: PathBuf,
        receiver: Receiver<TaskbarCommand>,
        ready_sender: SyncSender<Result<u32, String>>,
        projection: Receiver<PlaybackSnapshot>,
    ) {
        let initialized = match unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) } {
            Ok(_) => true,
            Err(error) => {
                let _ = ready_sender.send(Err(format!(
                    "failed to initialize taskbar COM apartment: {error}"
                )));
                return;
            }
        };
        let taskbar: ITaskbarList3 =
            match unsafe { CoCreateInstance(&TaskbarList, None, CLSCTX_INPROC_SERVER) } {
                Ok(taskbar) => taskbar,
                Err(error) => {
                    let _ = ready_sender.send(Err(format!(
                        "failed to create Windows taskbar integration: {error}"
                    )));
                    unsafe { CoUninitialize() };
                    return;
                }
            };
        if let Err(error) = unsafe { taskbar.HrInit() } {
            let _ = ready_sender.send(Err(format!("failed to initialize taskbar API: {error}")));
            unsafe { CoUninitialize() };
            return;
        }
        let icons = match TaskbarIcons::load(&resource_dir) {
            Ok(icons) => icons,
            Err(error) => {
                let _ = ready_sender.send(Err(error));
                unsafe { CoUninitialize() };
                return;
            }
        };
        let taskbar_created_message =
            unsafe { RegisterWindowMessageW(windows::w!("TaskbarButtonCreated")) };
        if taskbar_created_message == 0 {
            let _ = ready_sender.send(Err(
                "failed to register the TaskbarButtonCreated message".to_owned()
            ));
            unsafe { CoUninitialize() };
            return;
        }
        if ready_sender.send(Ok(taskbar_created_message)).is_err() {
            unsafe { CoUninitialize() };
            return;
        }

        let hwnd = HWND(hwnd);
        let mut buttons_registered = false;
        let mut last_projection = None;
        let mut latest_snapshot = None;
        loop {
            match receiver.recv_timeout(POLL_INTERVAL) {
                Ok(TaskbarCommand::Previous) => run_playback_command("previous", engine.previous()),
                Ok(TaskbarCommand::Toggle) => {
                    let result = match engine.snapshot() {
                        Ok(snapshot) if snapshot.status == PlaybackStatus::Playing => {
                            engine.pause()
                        }
                        Ok(snapshot) if snapshot.status == PlaybackStatus::Paused => {
                            engine.resume()
                        }
                        Ok(snapshot) => snapshot
                            .current_item_id
                            .map_or(Err(crate::playback::PlaybackError::NothingLoaded), |id| {
                                engine.play_queue_item(id)
                            }),
                        Err(error) => Err(error),
                    };
                    run_playback_command("play/pause", result);
                }
                Ok(TaskbarCommand::Next) => run_playback_command("next", engine.next()),
                Ok(TaskbarCommand::RecreateButtons) => {
                    buttons_registered = false;
                    last_projection = None;
                }
                Ok(TaskbarCommand::Shutdown) | Err(RecvTimeoutError::Disconnected) => break,
                Err(RecvTimeoutError::Timeout) => {}
            }

            if let Some(snapshot) = projection.try_iter().last() {
                latest_snapshot = Some(snapshot);
            }
            let Some(snapshot) = latest_snapshot.as_ref() else {
                continue;
            };
            let projection = TaskbarProjection::from_snapshot(snapshot);
            if !buttons_registered {
                let buttons = buttons(&projection, &icons);
                buttons_registered = unsafe { taskbar.ThumbBarAddButtons(hwnd, &buttons) }.is_ok();
                if !buttons_registered {
                    continue;
                }
            }
            if last_projection.as_ref() != Some(&projection) {
                if let Err(error) = sync_taskbar(&taskbar, hwnd, &icons, &projection) {
                    eprintln!("Windows taskbar projection failed: {error}");
                } else {
                    last_projection = Some(projection);
                }
            }
        }

        let _ = unsafe { taskbar.SetProgressState(hwnd, TBPF_NOPROGRESS) };
        drop(icons);
        drop(taskbar);
        if initialized {
            unsafe { CoUninitialize() };
        }
    }

    fn run_playback_command(
        label: &str,
        result: Result<PlaybackSnapshot, crate::playback::PlaybackError>,
    ) {
        if let Err(error) = result {
            eprintln!(
                "taskbar {label} command ignored: {}",
                error.failure().message
            );
        }
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct TaskbarProjection {
        status: PlaybackStatus,
        has_current: bool,
        has_previous: bool,
        has_next: bool,
        position_ms: u64,
        duration_ms: Option<u64>,
        seekable: bool,
    }

    impl TaskbarProjection {
        fn from_snapshot(snapshot: &PlaybackSnapshot) -> Self {
            let current_index = snapshot
                .current_item_id
                .and_then(|id| snapshot.queue.iter().position(|item| item.id == id));
            Self {
                status: snapshot.status,
                has_current: snapshot.current_item_id.is_some(),
                has_previous: current_index.is_some_and(|index| index > 0),
                has_next: current_index.is_some_and(|index| index + 1 < snapshot.queue.len()),
                position_ms: snapshot.position_ms,
                duration_ms: snapshot.duration_ms,
                seekable: snapshot.seekable,
            }
        }
    }

    fn sync_taskbar(
        taskbar: &ITaskbarList3,
        hwnd: HWND,
        icons: &TaskbarIcons,
        projection: &TaskbarProjection,
    ) -> windows::core::Result<()> {
        let buttons = buttons(projection, icons);
        unsafe { taskbar.ThumbBarUpdateButtons(hwnd, &buttons)? };

        let progress = projection
            .duration_ms
            .filter(|duration| *duration > 0 && projection.seekable);
        let Some(duration) = progress else {
            return unsafe { taskbar.SetProgressState(hwnd, TBPF_NOPROGRESS) };
        };
        unsafe {
            taskbar.SetProgressValue(hwnd, projection.position_ms.min(duration), duration)?;
            taskbar.SetProgressState(
                hwnd,
                match projection.status {
                    PlaybackStatus::Playing => TBPF_NORMAL,
                    PlaybackStatus::Paused => TBPF_PAUSED,
                    PlaybackStatus::Failed => TBPF_ERROR,
                    PlaybackStatus::Idle | PlaybackStatus::Stopped => TBPF_NOPROGRESS,
                },
            )
        }
    }

    fn buttons(projection: &TaskbarProjection, icons: &TaskbarIcons) -> [THUMBBUTTON; 3] {
        [
            button(
                PREVIOUS_BUTTON_ID,
                icons.previous,
                "上一首",
                projection.has_previous,
            ),
            button(
                PLAY_BUTTON_ID,
                if projection.status == PlaybackStatus::Playing {
                    icons.pause
                } else {
                    icons.play
                },
                if projection.status == PlaybackStatus::Playing {
                    "暂停"
                } else {
                    "播放"
                },
                projection.has_current,
            ),
            button(NEXT_BUTTON_ID, icons.next, "下一首", projection.has_next),
        ]
    }

    fn button(id: u32, icon: HICON, tooltip_text: &str, enabled: bool) -> THUMBBUTTON {
        let mut tooltip = [0_u16; 260];
        let max_tooltip_units = tooltip.len() - 1;
        for (slot, value) in tooltip
            .iter_mut()
            .zip(tooltip_text.encode_utf16().take(max_tooltip_units))
        {
            *slot = value;
        }
        THUMBBUTTON {
            dwMask: THUMBBUTTONMASK(THB_ICON.0 | THB_TOOLTIP.0 | THB_FLAGS.0),
            iId: id,
            hIcon: icon,
            szTip: tooltip,
            dwFlags: if enabled { THBF_ENABLED } else { THBF_DISABLED },
            ..Default::default()
        }
    }

    struct TaskbarIcons {
        previous: HICON,
        play: HICON,
        pause: HICON,
        next: HICON,
    }

    impl TaskbarIcons {
        fn load(resource_dir: &Path) -> Result<Self, String> {
            let development_icons = Path::new(env!("CARGO_MANIFEST_DIR")).join("icons");
            let resolve = |name: &str| {
                let bundled = resource_dir.join("icons").join(name);
                if bundled.is_file() {
                    bundled
                } else {
                    development_icons.join(name)
                }
            };
            Ok(Self {
                previous: load_icon(&resolve("taskbar-previous.ico"))?,
                play: load_icon(&resolve("taskbar-play.ico"))?,
                pause: load_icon(&resolve("taskbar-pause.ico"))?,
                next: load_icon(&resolve("taskbar-next.ico"))?,
            })
        }
    }

    impl Drop for TaskbarIcons {
        fn drop(&mut self) {
            unsafe {
                DestroyIcon(self.previous);
                DestroyIcon(self.play);
                DestroyIcon(self.pause);
                DestroyIcon(self.next);
            }
        }
    }

    fn load_icon(path: &Path) -> Result<HICON, String> {
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let handle = unsafe {
            LoadImageW(
                None,
                PCWSTR(wide.as_ptr()),
                IMAGE_ICON,
                0,
                0,
                LR_LOADFROMFILE,
            )
        }
        .map_err(|error| format!("failed to load taskbar icon {}: {error}", path.display()))?;
        Ok(HICON(handle.0))
    }

    use std::os::windows::ffi::OsStrExt;

    struct HookState {
        hwnd: isize,
        previous: isize,
        commands: Sender<TaskbarCommand>,
        taskbar_created_message: u32,
    }

    fn hook_state() -> &'static Mutex<Option<HookState>> {
        static STATE: OnceLock<Mutex<Option<HookState>>> = OnceLock::new();
        STATE.get_or_init(|| Mutex::new(None))
    }

    fn install_message_hook(
        app: &AppHandle,
        hwnd: isize,
        commands: Sender<TaskbarCommand>,
        taskbar_created_message: u32,
    ) -> Result<(), String> {
        let (sender, receiver) = mpsc::sync_channel(1);
        app.run_on_main_thread(move || {
            let result = install_message_hook_on_main(hwnd, commands, taskbar_created_message);
            let _ = sender.send(result);
        })
        .map_err(|error| format!("failed to dispatch taskbar message hook: {error}"))?;
        receiver
            .recv_timeout(Duration::from_secs(5))
            .map_err(|error| format!("taskbar message hook did not initialize: {error}"))?
    }

    fn install_message_hook_on_main(
        hwnd: isize,
        commands: Sender<TaskbarCommand>,
        taskbar_created_message: u32,
    ) -> Result<(), String> {
        let mut state = hook_state()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.is_some() {
            return Err("taskbar message hook is already installed".to_owned());
        }
        let hook_pointer = taskbar_window_proc as *const () as isize;
        let previous = unsafe { SetWindowLongPtrW(HWND(hwnd), GWLP_WNDPROC, hook_pointer) };
        if previous == 0 {
            return Err(format!(
                "failed to subclass the main window for taskbar buttons: {}",
                windows::core::Error::from_win32()
            ));
        }
        *state = Some(HookState {
            hwnd,
            previous,
            commands,
            taskbar_created_message,
        });
        Ok(())
    }

    fn uninstall_message_hook(app: &AppHandle, hwnd: isize) {
        let (sender, receiver) = mpsc::sync_channel(1);
        if app
            .run_on_main_thread(move || {
                let mut state = hook_state()
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                if let Some(hook) = state.take() {
                    if hook.hwnd == hwnd
                        && unsafe { GetWindowLongPtrW(HWND(hwnd), GWLP_WNDPROC) }
                            == taskbar_window_proc as *const () as isize
                    {
                        unsafe { SetWindowLongPtrW(HWND(hwnd), GWLP_WNDPROC, hook.previous) };
                    }
                }
                let _ = sender.send(());
            })
            .is_ok()
        {
            let _ = receiver.recv_timeout(Duration::from_secs(2));
        }
    }

    unsafe extern "system" fn taskbar_window_proc(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        let (previous, command) = {
            let state = hook_state()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let Some(state) = state.as_ref().filter(|state| state.hwnd == hwnd.0) else {
                return windows::Win32::UI::WindowsAndMessaging::DefWindowProcW(
                    hwnd, message, wparam, lparam,
                );
            };
            let command = if message == state.taskbar_created_message {
                Some(TaskbarCommand::RecreateButtons)
            } else if message == WM_COMMAND && ((wparam.0 >> 16) & 0xffff) as u32 == THBN_CLICKED {
                match (wparam.0 & 0xffff) as u32 {
                    PREVIOUS_BUTTON_ID => Some(TaskbarCommand::Previous),
                    PLAY_BUTTON_ID => Some(TaskbarCommand::Toggle),
                    NEXT_BUTTON_ID => Some(TaskbarCommand::Next),
                    _ => None,
                }
            } else {
                None
            };
            if let Some(command) = command {
                let _ = state.commands.send(command);
            }
            (state.previous, command.is_some())
        };
        if command {
            return LRESULT(0);
        }
        let previous: WNDPROC = std::mem::transmute(previous);
        CallWindowProcW(previous, hwnd, message, wparam, lparam)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        fn snapshot() -> PlaybackSnapshot {
            PlaybackSnapshot {
                status: PlaybackStatus::Playing,
                path: Some("track.flac".to_owned()),
                position_ms: 4_000,
                duration_ms: Some(10_000),
                seekable: true,
                ..PlaybackSnapshot::default()
            }
        }

        #[test]
        fn progress_requires_a_seekable_duration() {
            let mut snapshot = snapshot();
            let projection = TaskbarProjection::from_snapshot(&snapshot);
            assert_eq!(projection.duration_ms, Some(10_000));
            assert!(projection.seekable);

            snapshot.seekable = false;
            assert!(!TaskbarProjection::from_snapshot(&snapshot).seekable);
            snapshot.seekable = true;
            snapshot.duration_ms = None;
            assert_eq!(
                TaskbarProjection::from_snapshot(&snapshot).duration_ms,
                None
            );
        }
    }
}

#[cfg(target_os = "windows")]
pub use windows_impl::TaskbarAdapter;
