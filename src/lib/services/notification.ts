import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";

export type ReviewNotificationKind = "inline" | "file" | "change";

const notificationCopy: Record<
  ReviewNotificationKind,
  { title: string; body: string }
> = {
  inline: {
    title: "Inline answer ready",
    body: "Codex finished answering your question.",
  },
  file: {
    title: "File explanation ready",
    body: "Codex finished explaining the file changes.",
  },
  change: {
    title: "Change Review ready",
    body: "Codex finished reviewing the selected comparison.",
  },
};

export async function notifyReviewComplete(
  kind: ReviewNotificationKind,
): Promise<void> {
  if (typeof window === "undefined" || !("__TAURI_INTERNALS__" in window))
    return;

  try {
    let permissionGranted = await isPermissionGranted();
    if (!permissionGranted)
      permissionGranted = (await requestPermission()) === "granted";
    if (permissionGranted) sendNotification(notificationCopy[kind]);
  } catch {
    // Notification failures must not turn a completed review into an error.
  }
}
