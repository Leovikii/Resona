import assert from "node:assert/strict";
import test from "node:test";
import { hookHost } from "../tests/hookHost.mjs";

// Execute the real hook with deferred IPC and a minimal hook host; no browser or new test dependency.
function mountLibrary() {
  const pending = [];
  const hook = hookHost(new URL("../src/features/library/useLibrary.ts", import.meta.url), "useLibrary", {
    isTauriRuntime: () => true,
    invokeTauri: (command, args) => new Promise((resolve, reject) => pending.push({ command, args, resolve, reject })),
  });
  return {
    pending,
    render: () => hook({ run: (operation) => operation() }),
  };
}

test("playlist selection ignores stale success, failure and loading completion", async () => {
  const host = mountLibrary();
  const first = host.render().selectPlaylist(1);
  const second = host.render().selectPlaylist(2);
  host.pending[0].resolve([{ id: 11, playlistId: 1 }]);
  await first;
  assert.equal(host.render().selectedPlaylistId, 2);
  assert.deepEqual(host.render().selectedItems, []);
  assert.equal(host.render().itemsLoading, true);
  host.pending[1].resolve([{ id: 22, playlistId: 2 }]);
  await second;
  assert.equal(host.render().selectedItems[0].id, 22);
  const obsolete = host.render().selectPlaylist(1);
  await host.render().selectPlaylist(null);
  host.pending[2].reject(new Error("old failure"));
  await obsolete;
  assert.deepEqual(host.render().selectedItems, []);
  assert.equal(host.render().itemsLoading, false);
  assert.equal(host.render().error, null);
});

test("an edit completing after navigation does not replace the newly selected list", async () => {
  const host = mountLibrary();
  const selected = host.render().selectPlaylist(1);
  host.pending[0].resolve([{ id: 11, playlistId: 1 }]);
  await selected;
  const edit = host.render().clearItems(1);
  const navigation = host.render().selectPlaylist(2);
  host.pending[2].resolve([{ id: 22, playlistId: 2 }]);
  await navigation;
  host.pending[1].resolve([]);
  await Promise.resolve();
  host.pending[3].resolve([{ id: 1 }, { id: 2 }]);
  await edit;
  assert.equal(host.render().selectedPlaylistId, 2);
  assert.equal(host.render().selectedItems[0].id, 22);
});

test("a committed edit with failed playback sync reloads saved rows and keeps the error", async () => {
  const host = mountLibrary();
  const selection = host.render().selectPlaylist(1);
  host.pending[0].resolve([{ id: 11, playlistId: 1 }]);
  await selection;
  const edit = host.render().clearItems(1);
  host.pending[1].reject({ code: "playback_sequence_sync_failed", message: "execution detached" });
  await Promise.resolve();
  host.pending[2].resolve([{ id: 1 }]);
  await new Promise(setImmediate);
  host.pending[3].resolve([]);
  assert.equal(await edit, false);
  assert.deepEqual(host.render().selectedItems, []);
  assert.equal(host.render().error.code, "playback_sequence_sync_failed");
});
