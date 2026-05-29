import { convertFileSrc } from "@tauri-apps/api/core";

export function toAssetUrl(path?: string | null): string | null {
  if (!path) {
    return null;
  }
  if (path.startsWith("http") || path.startsWith("/") || path.startsWith("asset:")) {
    return path;
  }
  return convertFileSrc(path);
}

export function fallbackCoverSeed(title: string): string {
  let hash = 0;
  for (const char of title) {
    hash = (hash * 31 + char.charCodeAt(0)) >>> 0;
  }
  return `${hash % 360}deg`;
}
