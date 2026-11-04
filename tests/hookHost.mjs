import { readFileSync } from "node:fs";
import { stripTypeScriptTypes } from "node:module";

export function hookHost(path, name, bindings) {
  let cursor = 0;
  const effects = [];
  const slots = [];
  const state = (initial) => {
    const index = cursor++;
    if (!(index in slots)) slots[index] = typeof initial === "function" ? initial() : initial;
    return [slots[index], (next) => { slots[index] = typeof next === "function" ? next(slots[index]) : next; }];
  };
  const ref = (initial) => {
    const index = cursor++;
    return slots[index] ??= { current: initial };
  };
  const source = stripTypeScriptTypes(readFileSync(path, "utf8"))
    .replace(/^import .*?;\s*/gms, "").replaceAll("export function", "function").replaceAll("import.meta.env.DEV", "false");
  const environment = {
    useState: state, useRef: ref, useCallback: (callback) => callback,
    useMemo: (factory) => factory(), useEffect: (effect) => effects.push(effect), ...bindings,
  };
  const hook = new Function(...Object.keys(environment), `${source}\nreturn ${name};`)(...Object.values(environment));
  const render = (...args) => { cursor = 0; effects.length = 0; return hook(...args); };
  render.effects = effects;
  return render;
}
