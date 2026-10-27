# Signed update manifest fixture

`latest.json` is a fixed preview manifest signed by the Tauri CLI with a disposable,
passwordless test key. `public-key.txt` and `latest.json.sig` use Tauri's base64
Minisign envelope. The private key was discarded and is never a release credential.
The installer signature is a placeholder: this fixture tests manifest verification,
identity binding and tamper rejection without downloading or installing anything.
