import assert from "node:assert/strict";
import test from "node:test";
import { hookHost } from "../tests/hookHost.mjs";
import { emptySnapshot, mergePlaybackSnapshot, previewSnapshot } from "../src/shared/model/playback.ts";
import { emptyLyricsSnapshot, mergeLyricsSnapshot } from "../src/shared/model/lyrics.ts";

for (const [file, name, stateKey, commandKey] of [
  ["playback/usePlaybackController.ts", "usePlaybackController", "snapshot", "run"],
  ["lyrics/useDesktopLyricsNowPlaying.ts", "useDesktopLyricsNowPlaying", "playback", "runPlayback"],
]) {
  test(`${name} ignores old polling after a newer command and overlapping refresh`, async () => {
    const pending = [];
    const render = hookHost(new URL(`../src/features/${file}`, import.meta.url), name, {
      isTauriRuntime: () => true, useDocumentVisibility: () => true,
      previewSnapshot, emptySnapshot, mergePlaybackSnapshot, emptyLyricsSnapshot, mergeLyricsSnapshot,
      invokeTauri: (command) => new Promise((resolve, reject) => pending.push({ command, resolve, reject })),
      window: { setInterval: () => 1, clearInterval: () => {} },
    });
    const args = { run: (operation) => operation() };
    const refresh = () => {
      const state = render(args);
      if (state.refresh) return state.refresh();
      render.effects.at(-1)();
      return new Promise(setImmediate);
    };
    const old = refresh();
    const command = render(args)[commandKey]("pause_playback");
    pending[1].resolve({ ...emptySnapshot, status: "paused" });
    await command;
    pending[0].resolve({ playback: { ...emptySnapshot, status: "playing" }, lyrics: emptyLyricsSnapshot, activePlaylist: null });
    await old;
    assert.equal(render(args)[stateKey].status, "paused");
    const first = refresh();
    const second = refresh();
    pending[3].resolve({ playback: { ...emptySnapshot, positionMs: 200 }, lyrics: emptyLyricsSnapshot, activePlaylist: null });
    await second;
    pending[2].reject(new Error("stale refresh error"));
    await first;
    assert.equal(render(args)[stateKey].positionMs, 200);
    assert.equal(render(args).refreshError ?? render(args).error ?? null, null);
  });
}
