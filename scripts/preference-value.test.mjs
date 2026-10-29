import assert from "node:assert/strict";
import test from "node:test";
import { resolveLocalePreference } from "../src/shared/i18n/localePreference.ts";

test("system language accepts native POSIX and Windows UI language names", () => {
  for (const language of ["zh_CN.UTF-8", "zh_CN", "zh-Hant-HK", "ZH-cn"]) {
    assert.equal(resolveLocalePreference("system", [language, "C"]), "zh-CN");
  }
  for (const language of ["en_US.UTF-8", "en-US", "C.UTF-8", "POSIX"]) {
    assert.equal(resolveLocalePreference("system", [language, "zh_CN"]), "en");
  }
});

test("explicit language wins over native and WebView language differences", () => {
  assert.equal(resolveLocalePreference("zh-CN", ["C"]), "zh-CN");
  assert.equal(resolveLocalePreference("en", ["zh_CN.UTF-8"]), "en");
  assert.equal(resolveLocalePreference("system", ["zh_CN.UTF-8", "C"]), "zh-CN");
});

test("unsupported and empty system language lists fall back to English", () => {
  assert.equal(resolveLocalePreference("system", []), "en");
  assert.equal(resolveLocalePreference("system", ["fr-FR"]), "en");
});

import {
  normalizeSteppedPreference,
  parseNumberPreference,
} from "../src/app/preferenceValue.ts";

test("missing and blank numeric preferences use their defaults", () => {
  assert.equal(parseNumberPreference(null, 30), 30);
  assert.equal(parseNumberPreference("", 100), 100);
  assert.equal(parseNumberPreference("   ", 20), 20);
});

test("stored numeric preferences remain authoritative", () => {
  assert.equal(parseNumberPreference("28", 30), 28);
  assert.equal(parseNumberPreference("0", 20), 0);
});

test("invalid numeric preferences use their defaults", () => {
  assert.equal(parseNumberPreference("not-a-number", 30), 30);
});

test("stepped percentage preferences snap and clamp to their supported range", () => {
  assert.equal(normalizeSteppedPreference(14, 10, 100, 10, 100), 10);
  assert.equal(normalizeSteppedPreference(86, 10, 100, 10, 100), 90);
  assert.equal(normalizeSteppedPreference(-20, 0, 90, 10, 20), 0);
  assert.equal(normalizeSteppedPreference(100, 0, 90, 10, 20), 90);
  assert.equal(normalizeSteppedPreference(Number.NaN, 0, 90, 10, 20), 20);
});
