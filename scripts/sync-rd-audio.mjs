import { copyFile, mkdir, readdir, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const AUDIO_EXTENSIONS = new Set([".ogg", ".mp3", ".wav", ".aif", ".aiff", ".flac"]);
const defaultRdResources =
  "C:\\Users\\lizi\\Documents\\Doc\\Unity\\RDFucked_1\\Assets\\Resources";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDir, "..");
const publicAudioRoot = path.join(projectRoot, "public", "audio");
const adofaiRoot = path.join(publicAudioRoot, "adofai");
const rdRoot = path.join(publicAudioRoot, "rhythm-doctor");
const rdResourcesRoot = path.join(rdRoot, "resources");
const rdSourceRoot = process.env.RD_RESOURCES_ROOT || defaultRdResources;
const tauriAssetRoot = path.join(projectRoot, "src-tauri", "assets");
const RD_AUDIO_SOURCE_PREFIXES = ["sfx/"];

const GAME_SOUND_TYPES = {
  0: "ClapSoundP1Classic",
  1: "ClapSoundP2Classic",
  2: "ClapSoundP1Oneshot",
  3: "ClapSoundP2Oneshot",
  20: "SmallMistake",
  21: "BigMistake",
  22: "Hand1PopSound",
  23: "Hand2PopSound",
  24: "HeartExplosion",
  25: "HeartExplosion2",
  26: "HeartExplosion3",
  27: "ClapSoundHoldLongEnd",
  28: "ClapSoundHoldLongStart",
  29: "ClapSoundHoldShortEnd",
  30: "ClapSoundHoldShortStart",
  31: "PulseSoundHoldStart",
  32: "PulseSoundHoldShortEnd",
  33: "PulseSoundHoldEnd",
  34: "PulseSoundHoldStartAlt",
  35: "PulseSoundHoldShortEndAlt",
  36: "PulseSoundHoldEndAlt",
  37: "ClapSoundCPUClassic",
  38: "ClapSoundCPUOneshot",
  39: "ClapSoundHoldLongEndP2",
  40: "ClapSoundHoldLongStartP2",
  41: "ClapSoundHoldShortEndP2",
  42: "ClapSoundHoldShortStartP2",
  43: "PulseSoundHoldStartP2",
  44: "PulseSoundHoldShortEndP2",
  45: "PulseSoundHoldEndP2",
  46: "PulseSoundHoldStartAltP2",
  47: "PulseSoundHoldShortEndAltP2",
  48: "PulseSoundHoldEndAltP2",
  49: "FreezeshotSoundCueLow",
  50: "FreezeshotSoundCueHigh",
  51: "FreezeshotSoundRiser",
  52: "FreezeshotSoundCymbal",
  53: "BurnshotSoundCueLow",
  54: "BurnshotSoundCueHigh",
  55: "BurnshotSoundRiser",
  56: "BurnshotSoundCymbal",
  63: "Skipshot",
  65: "HoldshotSoundCue",
  66: "HoldshotSoundClapStart",
  67: "HoldshotSoundClapLongEnd",
  68: "HoldshotSoundClapShortEnd",
};

await mkdir(adofaiRoot, { recursive: true });
await syncRhythmDoctorResources();
await writeRhythmDoctorMetadata();
await writeManifest("adofai", adofaiRoot, adofaiRoot);
await writeManifest("rhythmDoctor", rdRoot, rdRoot);

async function syncRhythmDoctorResources() {
  await rm(rdResourcesRoot, { recursive: true, force: true });
  await mkdir(rdResourcesRoot, { recursive: true });
  const files = (await collectAudioFiles(rdSourceRoot)).filter(isRuntimeRhythmDoctorAudio);
  for (const source of files) {
    const relative = path.relative(rdSourceRoot, source);
    const target = path.join(rdResourcesRoot, relative);
    await mkdir(path.dirname(target), { recursive: true });
    await copyFile(source, target);
  }
  console.log(`已复制 RD 音频 ${files.length} 个`);
}

function isRuntimeRhythmDoctorAudio(file) {
  const relative = slash(path.relative(rdSourceRoot, file)).toLowerCase();
  return RD_AUDIO_SOURCE_PREFIXES.some((prefix) => relative.startsWith(prefix));
}

async function writeRhythmDoctorMetadata() {
  const songOffsetsPath = path.join(rdSourceRoot, "RDSongOffsets.asset");
  const gameSoundsPath = path.join(rdSourceRoot, "RDGameSounds.prefab");
  const [songOffsetsText, gameSoundsText] = await Promise.all([
    readTextIfExists(songOffsetsPath),
    readTextIfExists(gameSoundsPath),
  ]);
  const metadata = {
    generatedAt: new Date().toISOString(),
    soundOffsets: parseSongOffsets(songOffsetsText),
    gameSounds: parseGameSounds(gameSoundsText),
  };
  await mkdir(tauriAssetRoot, { recursive: true });
  await mkdir(rdRoot, { recursive: true });
  const text = `${JSON.stringify(metadata, null, 2)}\n`;
  await writeFile(path.join(tauriAssetRoot, "rhythm-doctor-audio-metadata.json"), text, "utf8");
  await writeFile(path.join(rdRoot, "audio-metadata.json"), text, "utf8");
  console.log(
    `已生成 RD 音频元数据，offset ${Object.keys(metadata.soundOffsets).length} 个，game sound ${Object.keys(metadata.gameSounds).length} 个`,
  );
}

async function writeManifest(game, scanRoot, manifestRoot) {
  const files = await collectAudioFiles(scanRoot);
  const entries = files.map((file) => manifestEntry(file, manifestRoot));
  const aliases = {};
  for (const entry of entries) {
    for (const alias of entry.aliases) {
      const normalized = normalizeAlias(alias);
      aliases[normalized] ??= [];
      aliases[normalized].push(entry.path);
    }
  }
  for (const paths of Object.values(aliases)) {
    paths.sort(compareManifestPath);
  }
  const manifest = {
    game,
    generatedAt: new Date().toISOString(),
    files: entries.sort((left, right) => left.path.localeCompare(right.path)),
    aliases,
  };
  await writeFile(path.join(scanRoot, "manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`, "utf8");
  console.log(`已生成 ${game} manifest，资源 ${entries.length} 个`);
}

async function collectAudioFiles(root) {
  const output = [];
  async function walk(current) {
    let entries;
    try {
      entries = await readdir(current, { withFileTypes: true });
    } catch {
      return;
    }
    for (const entry of entries) {
      const fullPath = path.join(current, entry.name);
      if (entry.isDirectory()) {
        await walk(fullPath);
      } else if (entry.isFile() && AUDIO_EXTENSIONS.has(path.extname(entry.name).toLowerCase())) {
        output.push(fullPath);
      }
    }
  }
  await walk(root);
  output.sort((left, right) => left.localeCompare(right));
  return output;
}

function manifestEntry(file, root) {
  const parsed = path.parse(file);
  const relative = slash(path.relative(root, file));
  const category = slash(path.dirname(relative));
  const aliases = buildAliases(relative, parsed.name, parsed.base);
  return {
    name: parsed.base,
    stem: parsed.name,
    extension: parsed.ext.slice(1).toLowerCase(),
    category: category === "." ? "" : category,
    path: relative,
    aliases,
  };
}

function buildAliases(relative, stem, filename) {
  const noExtension = relative.replace(/\.[^.]+$/, "");
  const aliases = new Set([filename, stem, noExtension, relative]);
  if (stem.startsWith("snd") && stem.length > 3) {
    aliases.add(stem.slice(3));
  } else {
    aliases.add(`snd${stem}`);
  }
  return [...aliases].filter(Boolean).sort((left, right) => left.localeCompare(right));
}

function compareManifestPath(left, right) {
  const leftScore = manifestPathScore(left);
  const rightScore = manifestPathScore(right);
  if (leftScore !== rightScore) {
    return leftScore - rightScore;
  }
  return left.localeCompare(right);
}

function manifestPathScore(value) {
  if (value.startsWith("resources/sfx/")) {
    return 0;
  }
  return 1;
}

function normalizeAlias(alias) {
  return slash(alias).trim().toLowerCase();
}

function slash(value) {
  return value.replace(/\\/g, "/");
}

async function readTextIfExists(file) {
  try {
    return await import("node:fs/promises").then(({ readFile }) => readFile(file, "utf8"));
  } catch {
    return "";
  }
}

function parseSongOffsets(text) {
  const offsets = {};
  let current = null;
  for (const rawLine of text.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (line.startsWith("- name:")) {
      if (current?.name) {
        offsets[current.name] = current;
      }
      current = {
        name: valueAfterColon(line),
        offsetMs: 0,
        volume: 1,
        folder: "",
      };
      continue;
    }
    if (!current || !line.includes(":")) {
      continue;
    }
    const key = line.slice(0, line.indexOf(":")).trim();
    const value = valueAfterColon(line);
    if (key === "offset") {
      current.offsetMs = Math.round(Number(value || 0) * 1000);
    } else if (key === "volume") {
      current.volume = Number(value || 1);
    } else if (key === "folder") {
      current.folder = value;
    }
  }
  if (current?.name) {
    offsets[current.name] = current;
  }
  return offsets;
}

function parseGameSounds(text) {
  const byType = {};
  let current = null;
  for (const rawLine of text.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (line.startsWith("- type:")) {
      if (current && GAME_SOUND_TYPES[current.type] !== undefined) {
        byType[GAME_SOUND_TYPES[current.type]] = current;
      }
      current = {
        type: Number(valueAfterColon(line)),
        filename: "",
        volume: 1,
        minPitch: 1,
        maxPitch: 1,
        pan: 0,
      };
      continue;
    }
    if (!current || !line.includes(":")) {
      continue;
    }
    const key = line.slice(0, line.indexOf(":")).trim();
    const value = valueAfterColon(line);
    if (key === "filename") {
      current.filename = value;
    } else if (key === "volume" || key === "minPitch" || key === "maxPitch" || key === "pan") {
      current[key] = Number(value || 0);
    }
  }
  if (current && GAME_SOUND_TYPES[current.type] !== undefined) {
    byType[GAME_SOUND_TYPES[current.type]] = current;
  }
  return byType;
}

function valueAfterColon(line) {
  return line.slice(line.indexOf(":") + 1).trim();
}
