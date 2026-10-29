// A UI gate rejects duplicate intents; Rust still owns the cross-window file-task lock.
export function createImportGate(onChange: (active: boolean) => void) {
  let active = false;
  return async function run<T>(operation: () => Promise<T>): Promise<T | null> {
    if (active) return null;
    active = true;
    onChange(true);
    try { return await operation(); }
    finally { active = false; onChange(false); }
  };
}
