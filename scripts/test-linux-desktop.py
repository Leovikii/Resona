#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""KDE session integration test; requires a Release build, busctl and FFmpeg."""
import os
import shlex
from pathlib import Path
import shutil
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
NAME = "org.mpris.MediaPlayer2.io.github.vki.resona"
PATH = "/org/mpris/MediaPlayer2"
PLAYER = "org.mpris.MediaPlayer2.Player"


def bus(*args):
    return subprocess.check_output(["busctl", "--user", "--", *args], text=True, timeout=5).strip()


def get(prop):
    return bus("get-property", NAME, PATH, PLAYER, prop)


def call(method, *args):
    return bus("call", NAME, PATH, PLAYER, method, *args)


def wait_property(prop, expected):
    deadline = time.monotonic() + 6
    while time.monotonic() < deadline:
        value = get(prop)
        if expected in value:
            return value
        time.sleep(0.1)
    raise AssertionError((prop, value, expected))


def check_lyrics(root):
    deadline = time.monotonic() + 6
    while True:
        items = shlex.split(bus("get-property", "org.kde.StatusNotifierWatcher", "/StatusNotifierWatcher", "org.kde.StatusNotifierWatcher", "RegisteredStatusNotifierItems"))
        item = next((item for item in items if item.endswith("/tray_icon_tray_app_resona_main")), None)
        if item is not None:
            break
        assert time.monotonic() < deadline, "Tray item did not register with KDE"
        time.sleep(0.1)
    owner, item_path = item.split("/", 1)
    menu = shlex.split(bus("get-property", owner, "/" + item_path, "org.kde.StatusNotifierItem", "Menu"))[1]
    layout = bus("call", owner, menu, "com.canonical.dbusmenu", "GetLayout", "iias", "0", "-1", "0")
    lyrics_id = next(group.split()[0] for group in layout.split("(ia{sv}av)")[1:] if '"checkmark"' in group)
    logs = root / "data/io.github.vki.resona/logs/resona.log"
    for count in (1, 2):
        bus("call", owner, menu, "com.canonical.dbusmenu", "Event", "isvu", lyrics_id, "clicked", "i", "0", "0")
        deadline = time.monotonic() + 12
        while not logs.exists() or logs.read_text().count("desktop lyrics layer surface shown") < count:
            assert time.monotonic() < deadline, "Lyrics frontend never completed its readiness handshake"
            time.sleep(0.1)
        assert "Wayland layer=overlay keyboard=none exclusive-zone=-1" in logs.read_text()
        bus("call", owner, menu, "com.canonical.dbusmenu", "Event", "isvu", lyrics_id, "clicked", "i", "0", "0")
        time.sleep(0.3)


def check_contract():
    metadata = wait_property("Metadata", "Wayland Lyric Test")
    assert all(key in metadata for key in ("Resona Test", "KDE Integration", "mpris:artUrl", "%20"))
    call("Pause")
    wait_property("PlaybackStatus", '"Paused"')
    call("Play")
    wait_property("PlaybackStatus", '"Playing"')
    call("Play")
    time.sleep(0.6)
    assert '"Playing"' in get("PlaybackStatus"), "Play must be idempotent"
    call("PlayPause")
    wait_property("PlaybackStatus", '"Paused"')
    bus("set-property", NAME, PATH, PLAYER, "Volume", "d", "0.35")
    deadline = time.monotonic() + 6
    while abs(float(get("Volume").split()[1]) - 0.35) > 0.001:
        assert time.monotonic() < deadline, "Volume did not reach the engine"
        time.sleep(0.1)
    track = get("Metadata").split('"mpris:trackid" o "')[1].split('"')[0]
    call("SetPosition", "ox", track, "12000000")
    wait_property("Position", "12000000")
    call("SetPosition", "ox", "/stale/track", "30000000")
    time.sleep(0.6)
    assert get("Position") == "x 12000000"
    call("Seek", "x", "-2000000")
    wait_property("Position", "10000000")
    bus("set-property", NAME, PATH, PLAYER, "LoopStatus", "s", "Track")
    wait_property("LoopStatus", '"Track"')
    bus("set-property", NAME, PATH, PLAYER, "Shuffle", "b", "true")
    wait_property("Shuffle", "true")
    call("Next")
    deadline = time.monotonic() + 6
    while track in get("Metadata"):
        assert time.monotonic() < deadline, "Next did not change the track"
        time.sleep(0.1)
    call("Previous")
    wait_property("Metadata", track)
    bus("call", NAME, PATH, "org.mpris.MediaPlayer2", "Raise")
    call("Stop")
    wait_property("PlaybackStatus", '"Stopped"')
    bus("call", NAME, PATH, "org.mpris.MediaPlayer2", "Quit")


def check_file_associations(root, first, environment, binary):
    desktop = Path(os.environ.get("RESONA_TEST_DESKTOP", ROOT / "packaging/arch/io.github.vki.resona.desktop"))
    subprocess.run(["desktop-file-validate", str(desktop)], check=True, timeout=5)
    launch_env = dict(environment, PATH=str(binary.parent) + os.pathsep + os.environ["PATH"])
    # Exercise the installed desktop-entry command and %F expansion, including Unicode/spaces.
    targets = [first]
    association_dir = root / "Open with 测试"
    association_dir.mkdir()
    for name, codec in [("关联 WAV 测试.wav", "pcm_s16le"), ("关联 MP3 测试.mp3", "libmp3lame")]:
        target = association_dir / name
        subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-i", str(first), "-c:a", codec, str(target)], check=True, timeout=30)
        targets.append(target)
    for target in [*targets, first]:
        subprocess.run(["gio", "launch", str(desktop), str(target)], env=launch_env, check=True, timeout=10)
        wait_property("Metadata", target.as_uri())
    assert "second instance activation received" in (root / "data/io.github.vki.resona/logs/resona.log").read_text()


def check_cold_desktop_open(first, environment, binary):
    desktop = Path(os.environ.get("RESONA_TEST_DESKTOP", ROOT / "packaging/arch/io.github.vki.resona.desktop"))
    launch_env = dict(environment, PATH=str(binary.parent) + os.pathsep + os.environ["PATH"])
    subprocess.run(["gio", "launch", str(desktop), str(first)], env=launch_env, check=True, timeout=10)
    try:
        deadline = time.monotonic() + 12
        while NAME not in bus("call", "org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus", "ListNames"):
            assert time.monotonic() < deadline, "Cold desktop launch did not register MPRIS"
            time.sleep(0.1)
        wait_property("Metadata", first.as_uri())
    finally:
        if NAME in bus("call", "org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus", "ListNames"):
            bus("call", NAME, PATH, "org.mpris.MediaPlayer2", "Quit")
    deadline = time.monotonic() + 10
    while NAME in bus("call", "org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus", "ListNames"):
        assert time.monotonic() < deadline, "Cold desktop instance did not quit"
        time.sleep(0.1)


def main():
    names = bus("call", "org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus", "ListNames")
    if NAME in names or "io.github.vki.resona.SingleInstance" in names:
        raise SystemExit("Close the running Resona instance before this isolated test.")
    with tempfile.TemporaryDirectory(prefix="resona-mpris-") as temporary:
        root = Path(temporary)
        music = root / "Music 测试"
        music.mkdir()
        first = music / "01 test.flac"
        subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "anullsrc=r=44100:cl=stereo", "-t", "180", "-c:a", "flac", "-metadata", "title=Wayland Lyric Test", "-metadata", "artist=Resona Test", "-metadata", "album=KDE Integration", str(first)], check=True, timeout=30)
        first.with_suffix(".lrc").write_text("[00:00.00]Wayland 桌面歌词\n[00:30.00]KDE 媒体控制同步\n")
        shutil.copyfile(first, music / "02 test.flac")
        shutil.copyfile(ROOT / "src-tauri/icons/default-artwork.png", music / "cover.png")
        desktop_fonts = Path(os.environ.get("XDG_DATA_HOME", Path.home() / ".local/share")) / "fonts"
        (root / "data").mkdir()
        if desktop_fonts.is_dir():
            (root / "data/fonts").symlink_to(desktop_fonts, target_is_directory=True)
        environment = dict(os.environ, GDK_BACKEND="wayland", XDG_DATA_HOME=str(root / "data"), XDG_CACHE_HOME=str(root / "cache"))
        with (root / "native.log").open("w+") as log:
            binary = Path(os.environ.get("RESONA_TEST_BINARY", ROOT / "src-tauri/target/release/resona")).resolve()
            process = subprocess.Popen([str(binary), str(first)], env=environment, stdout=log, stderr=subprocess.STDOUT)
            try:
                deadline = time.monotonic() + 15
                while NAME not in bus("call", "org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus", "ListNames"):
                    assert process.poll() is None, "Resona exited before MPRIS registered"
                    assert time.monotonic() < deadline, "MPRIS did not register"
                    time.sleep(0.1)
                check_lyrics(root)
                check_file_associations(root, first, environment, binary)
                check_contract()
                assert "application update check started" not in (root / "data/io.github.vki.resona/logs/resona.log").read_text()
                assert process.wait(timeout=10) == 0
                assert NAME not in bus("call", "org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus", "ListNames")
                check_cold_desktop_open(first, environment, binary)
            except BaseException:
                log.seek(0)
                print(log.read())
                diagnostic = root / "data/io.github.vki.resona/logs/resona.log"
                if diagnostic.exists():
                    print(diagnostic.read_text())
                raise
            finally:
                if process.poll() is None:
                    process.terminate()
                    try:
                        process.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait()
    print("PASS: desktop-entry MP3/WAV/FLAC opening and single instance; no Linux startup update check; native overlay lyrics readiness/recreation; MPRIS metadata/artwork, transport, seek/stale IDs, volume, modes, Raise and Quit")


if __name__ == "__main__":
    main()
