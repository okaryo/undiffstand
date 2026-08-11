import { describe, expect, it } from "vitest";
import { resolveApplicationShortcut } from "./shortcuts";

function keyboardEvent(init: KeyboardEventInit) {
  return new KeyboardEvent("keydown", init);
}

describe("resolveApplicationShortcut", () => {
  it.each([
    [{ key: "Escape" }, "dismiss-dialogs"],
    [{ key: ",", code: "Comma", metaKey: true }, "open-settings"],
    [{ key: "r", code: "KeyR", metaKey: true }, "refresh-change-detail"],
    [{ key: "b", code: "KeyB", metaKey: true }, "toggle-changed-files"],
    [
      { key: "∫", code: "KeyB", metaKey: true, altKey: true },
      "toggle-ai-panel",
    ],
  ] satisfies [
    KeyboardEventInit,
    ReturnType<typeof resolveApplicationShortcut>,
  ][])("resolves %o as %s", (init, shortcut) => {
    expect(resolveApplicationShortcut(keyboardEvent(init))).toBe(shortcut);
  });

  it.each([
    { key: "r", code: "KeyR" },
    { key: "r", code: "KeyR", metaKey: true, repeat: true },
    { key: "R", code: "KeyR", metaKey: true, shiftKey: true },
    { key: "r", code: "KeyR", metaKey: true, altKey: true },
    { key: "r", code: "KeyR", metaKey: true, ctrlKey: true },
  ])("ignores unsupported key combination %o", (init) => {
    expect(resolveApplicationShortcut(keyboardEvent(init))).toBeUndefined();
  });
});
