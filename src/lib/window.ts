import { getCurrentWindow } from "@tauri-apps/api/window";

export type WindowAction = "minimize" | "maximize" | "close";

export async function runWindowAction(action: WindowAction) {
  const appWindow = currentTauriWindow();
  if (!appWindow) {
    return;
  }
  if (action === "minimize") {
    await appWindow.minimize();
  } else if (action === "maximize") {
    await appWindow.toggleMaximize();
  } else {
    await appWindow.close();
  }
}

export function startWindowDrag() {
  const appWindow = currentTauriWindow();
  if (!appWindow) {
    return;
  }
  void appWindow.startDragging();
}

function currentTauriWindow() {
  if (typeof window === "undefined" || !("__TAURI_INTERNALS__" in window)) {
    return null;
  }
  return getCurrentWindow();
}
