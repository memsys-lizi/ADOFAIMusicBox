import { toAssetUrl } from "../lib/assets";
import type { GameMode } from "../types/domain";

interface AudioManifest {
  game: string;
  files: AudioManifestFile[];
  aliases: Record<string, string[]>;
}

interface AudioManifestFile {
  name: string;
  stem: string;
  extension: string;
  category: string;
  path: string;
  aliases: string[];
}

const manifestCache = new Map<GameMode, Promise<AudioManifest | null>>();

export async function resolveAudioSource(game: GameMode, soundName: string): Promise<string | null> {
  if (!soundName) {
    return null;
  }
  if (isLocalLikePath(soundName)) {
    return toAssetUrl(soundName);
  }

  const cleanName = stripGamePrefix(game, soundName);
  const manifest = await loadManifest(game);
  const path = manifest ? resolveFromManifest(game, manifest, cleanName) : null;
  if (path) {
    return path;
  }

  const basePath = game === "rhythmDoctor" ? "/audio/rhythm-doctor" : "/audio/adofai";
  const stem = cleanName.replace(/\.[^.]+$/, "");
  for (const ext of [".ogg", ".wav", ".mp3"]) {
    return `${basePath}/${stem}${ext}`;
  }
  return null;
}

function stripGamePrefix(game: GameMode, soundName: string) {
  if (game === "rhythmDoctor" && soundName.startsWith("rd:")) {
    return soundName.slice(3);
  }
  if (game === "adofai" && soundName.startsWith("adofai:")) {
    return soundName.slice(7);
  }
  return soundName;
}

function isLocalLikePath(soundName: string) {
  return (
    soundName.startsWith("/") ||
    soundName.startsWith("http") ||
    soundName.startsWith("asset:") ||
    /^[a-zA-Z]:[\\/]/.test(soundName) ||
    soundName.includes("\\")
  );
}

function loadManifest(game: GameMode) {
  const existing = manifestCache.get(game);
  if (existing) {
    return existing;
  }
  const path = game === "rhythmDoctor" ? "/audio/rhythm-doctor/manifest.json" : "/audio/adofai/manifest.json";
  const promise = fetch(path)
    .then((response) => (response.ok ? response.json() as Promise<AudioManifest> : null))
    .catch(() => null);
  manifestCache.set(game, promise);
  return promise;
}

function resolveFromManifest(game: GameMode, manifest: AudioManifest, soundName: string) {
  const normalized = normalizeAlias(soundName);
  const candidates = manifest.aliases[normalized] ?? manifest.aliases[normalizeAlias(soundName.replace(/\.[^.]+$/, ""))];
  const chosen = chooseCandidate(game, candidates ?? []);
  if (!chosen) {
    return null;
  }
  const root = game === "rhythmDoctor" ? "/audio/rhythm-doctor" : "/audio/adofai";
  return `${root}/${encodeAssetPath(chosen)}`;
}

function chooseCandidate(game: GameMode, candidates: string[]) {
  if (candidates.length === 0) {
    return null;
  }
  if (game === "rhythmDoctor") {
    return (
      candidates.find((path) => path.startsWith("resources/sfx/")) ??
      candidates.find((path) => path.startsWith("resources/music/")) ??
      candidates[0]
    );
  }
  return candidates[0];
}

function normalizeAlias(alias: string) {
  return alias.trim().replace(/\\/g, "/").toLowerCase();
}

function encodeAssetPath(path: string) {
  return path
    .split("/")
    .map((part) => encodeURIComponent(part))
    .join("/");
}
