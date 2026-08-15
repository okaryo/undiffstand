export type ApplicationShortcut =
  | "dismiss-dialogs"
  | "open-settings"
  | "find-in-changes"
  | "refresh-change-detail"
  | "toggle-changed-files"
  | "toggle-ai-panel";

export function resolveApplicationShortcut(
  event: KeyboardEvent,
): ApplicationShortcut | undefined {
  if (event.key === "Escape") return "dismiss-dialogs";
  if (event.repeat || !event.metaKey || event.ctrlKey || event.shiftKey)
    return undefined;

  if (matchesKey(event, "Comma", ",")) {
    return event.altKey ? undefined : "open-settings";
  }
  if (matchesKey(event, "KeyR", "r")) {
    return event.altKey ? undefined : "refresh-change-detail";
  }
  if (matchesKey(event, "KeyF", "f")) {
    return event.altKey ? undefined : "find-in-changes";
  }
  if (matchesKey(event, "KeyB", "b")) {
    return event.altKey ? "toggle-ai-panel" : "toggle-changed-files";
  }

  return undefined;
}

function matchesKey(event: KeyboardEvent, code: string, key: string) {
  return event.code === code || event.key.toLowerCase() === key;
}
