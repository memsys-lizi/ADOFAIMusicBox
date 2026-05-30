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

await mkdir(adofaiRoot, { recursive: true });
await syncRhythmDoctorResources();
await writeManifest("adofai", adofaiRoot, adofaiRoot);
await writeManifest("rhythmDoctor", rdRoot, rdRoot);

async function syncRhythmDoctorResources() {
  await rm(rdResourcesRoot, { recursive: true, force: true });
  await mkdir(rdResourcesRoot, { recursive: true });
  const files = await collectAudioFiles(rdSourceRoot);
  for (const source of files) {
    const relative = path.relative(rdSourceRoot, source);
    const target = path.join(rdResourcesRoot, relative);
    await mkdir(path.dirname(target), { recursive: true });
    await copyFile(source, target);
  }
  console.log(`已复制 RD 音频 ${files.length} 个`);
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
  if (value.startsWith("resources/music/")) {
    return 1;
  }
  if (value.startsWith("resources/internallevels/")) {
    return 2;
  }
  return 3;
}

function normalizeAlias(alias) {
  return slash(alias).trim().toLowerCase();
}

function slash(value) {
  return value.replace(/\\/g, "/");
}
