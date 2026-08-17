/* Builds the development sound pack from the Elpy reference material.
 *
 * Everything this produces is DEV-ONLY. The Elpy audio and cover art belong to
 * Vane Jung; they exist here so the engine has something real to loop while
 * Nuru's own recordings are sourced. Every entry is written with
 * shippable:false, and the bundler refuses to package a pack in that state.
 *
 * Why FLAC: Nuru's looping is sample-exact, and AAC cannot be. Every .m4a
 * carries encoder delay and padding, and seeking lands on a 1024-sample frame
 * boundary rather than a sample. FLAC decodes to exactly the samples that went
 * in, which is the precondition for a seam-free loop.
 *
 *   node scripts/build-placeholder-pack.mjs
 */

import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, writeFileSync, statSync, copyFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const REF = join(root, 'dev-notes', 'elpy-reference');

// Lives under resources/ so it is bundled into the installer — without it an
// installed build has nothing to play. Kept in its own pack directory, separate
// from `builtin`, so the whole thing can be deleted in one move once Nuru has
// audio of its own. Its contents are gitignored.
const OUT = join(root, 'resources', 'packs', 'elpy-placeholder');

/** Long sources get trimmed — a 21-minute café bed proves nothing a 5-minute one
 *  doesn't, and two files stay long on purpose so the streaming path and its RAM
 *  ceiling actually get exercised. */
const MAX_SECONDS = 300;

/** Ambient beds are not authored to loop, so their head and tail don't match.
 *  An equal-power crossfade over this window makes the wrap inaudible. Sounds
 *  that are authored to loop get 0 and a butt-joint instead. */
const CROSSFADE_MS = 900;

/* id → presentation. Accents are Elpy's palette, which is genuinely well judged
 * for a dark UI: saturated enough to read as "lit", desaturated enough not to
 * vibrate against warm grey. A palette isn't protectable; the audio is. */
const SOUNDS = [
  { id: 'beach',       src: 'beach',          name: 'Beach',          accent: '#5AE3FF', tags: ['nature', 'water'],   nook: { channel: 'window',  state: 'beach',    weight: 0.8 } },
  { id: 'birds',       src: 'birds',          name: 'Birds',          accent: '#FCF44C', tags: ['nature', 'life'],    nook: { channel: 'sky',     state: 'morning',  weight: 0.9 } },
  { id: 'cafe',        src: 'cafe',           name: 'Café',           accent: '#F9CF05', tags: ['urban', 'people'],   nook: { channel: 'life',    state: 'cafe',     weight: 1.0 } },
  { id: 'campfire',    src: 'campfire',       name: 'Campfire',       accent: '#F7896B', tags: ['nature', 'fire'],    nook: { channel: 'hearth',  state: 'campfire', weight: 0.9 } },
  { id: 'city',        src: 'city',           name: 'City',           accent: '#33849F', tags: ['urban'],             nook: { channel: 'window',  state: 'city',     weight: 0.9 } },
  { id: 'fireplace',   src: 'fireplace',      name: 'Fireplace',      accent: '#E4B7B4', tags: ['indoor', 'fire'],    nook: { channel: 'hearth',  state: 'fireplace',weight: 1.0 } },
  { id: 'forest',      src: 'forest',         name: 'Forest',         accent: '#74DC74', tags: ['nature'],            nook: { channel: 'window',  state: 'forest',   weight: 0.9 } },
  { id: 'heavy-rain',  src: 'heavy-rain',     name: 'Heavy Rain',     accent: '#779CFF', tags: ['weather', 'water'],  nook: { channel: 'weather', state: 'downpour', weight: 1.0 } },
  { id: 'crickets',    src: 'night-crickets', name: 'Night Crickets', accent: '#D5B5F6', tags: ['nature', 'life'],    nook: { channel: 'sky',     state: 'night',    weight: 1.0 } },
  { id: 'rain',        src: 'rain',           name: 'Rain',           accent: '#B4E4FC', tags: ['weather', 'water'],  nook: { channel: 'weather', state: 'rain',     weight: 0.8 } },
  { id: 'rain-canvas', src: 'rain-camping',   name: 'Rain on Canvas', accent: '#62E6A7', tags: ['weather', 'indoor'], nook: { channel: 'weather', state: 'rain',     weight: 0.6 } },
  { id: 'rain-glass',  src: 'rain-car',       name: 'Rain on Glass',  accent: '#F4BF71', tags: ['weather', 'indoor'], nook: { channel: 'weather', state: 'rain',     weight: 0.6 } },
  { id: 'snow',        src: 'snow',           name: 'Snowfall',       accent: '#D6E1E6', tags: ['weather'],           nook: { channel: 'weather', state: 'snow',     weight: 1.0 } },
  { id: 'train',       src: 'train',          name: 'Train',          accent: '#F54D55', tags: ['urban', 'transit'],  nook: { channel: 'transit', state: 'train',    weight: 1.0 } },
  { id: 'thunder',     src: 'thunder',        name: 'Thunder',        accent: '#ABB5D5', tags: ['weather'],           nook: { channel: 'weather', state: 'storm',    weight: 1.2 } },
  { id: 'wind',        src: 'wind',           name: 'Wind',           accent: '#44FACC', tags: ['weather'],           nook: { channel: 'weather', state: 'wind',     weight: 0.5 } },
  { id: 'brown-noise', src: 'brown-noise',    name: 'Brown Noise',    accent: '#98896D', tags: ['noise'],             nook: { channel: 'none',    state: 'none',     weight: 0 } },
  { id: 'pink-noise',  src: 'pink-noise',     name: 'Pink Noise',     accent: '#F16B79', tags: ['noise'],             nook: { channel: 'none',    state: 'none',     weight: 0 } },
  { id: 'white-noise', src: 'white-noise',    name: 'White Noise',    accent: '#DDDDDD', tags: ['noise'],             nook: { channel: 'none',    state: 'none',     weight: 0 } },
];

/* Elpy reuses one noise cover for all three generators; give each its own later. */
const COVER_FOR = { 'brown-noise': 'noise', 'pink-noise': 'noise', 'white-noise': 'noise' };

function ffprobe(file) {
  const out = execFileSync(
    'ffprobe',
    ['-v', 'error', '-select_streams', 'a:0', '-show_entries',
     'stream=sample_rate,channels,duration_ts,time_base:format=duration', '-of', 'json', file],
    { encoding: 'utf8' },
  );
  const j = JSON.parse(out);
  const s = j.streams[0];
  return {
    sampleRate: Number(s.sample_rate),
    channels: Number(s.channels),
    duration: Number(j.format.duration),
  };
}

function sha256(file) {
  return createHash('sha256').update(readFileSync(file)).digest('hex');
}

function main() {
  if (!existsSync(join(REF, 'assets', 'audio'))) {
    console.error(`Missing ${join(REF, 'assets', 'audio')} — run the Elpy extraction first.`);
    process.exit(1);
  }

  mkdirSync(join(OUT, 'audio'), { recursive: true });
  mkdirSync(join(OUT, 'covers'), { recursive: true });

  const entries = [];

  for (const s of SOUNDS) {
    const inFile = join(REF, 'assets', 'audio', `${s.src}.m4a`);
    const outFile = join(OUT, 'audio', `${s.id}.flac`);
    const probe = ffprobe(inFile);
    const seconds = Math.min(probe.duration, MAX_SECONDS);

    process.stdout.write(`  ${s.id.padEnd(13)} ${seconds.toFixed(1)}s @ ${probe.sampleRate} … `);

    // -sample_fmt s16: the source is lossy AAC, so 16 bit is already transparent
    // to it. -compression_level 8 costs encode time we only pay once.
    execFileSync('ffmpeg', [
      '-hide_banner', '-loglevel', 'error', '-y',
      '-i', inFile,
      ...(probe.duration > MAX_SECONDS ? ['-t', String(MAX_SECONDS)] : []),
      '-map', 'a:0',
      '-c:a', 'flac', '-compression_level', '8', '-sample_fmt', 's16',
      outFile,
    ]);

    const after = ffprobe(outFile);
    const frames = Math.round(after.duration * after.sampleRate);
    const bytes = statSync(outFile).size;
    console.log(`${(bytes / 1e6).toFixed(1)} MB`);

    const coverSrc = join(REF, 'assets', 'coverimage', `${COVER_FOR[s.id] ?? s.src}.png`);
    const coverOut = join(OUT, 'covers', `${s.id}.png`);
    if (existsSync(coverSrc)) copyFileSync(coverSrc, coverOut);

    entries.push({
      id: s.id,
      name: s.name,
      accent: s.accent,
      tags: s.tags,
      cover: `covers/${s.id}.png`,
      audio: {
        file: `audio/${s.id}.flac`,
        bytes,
        sha256: sha256(outFile),
        sampleRate: after.sampleRate,
        channels: after.channels,
        frames,
        codec: 'flac',
      },
      loop: {
        startSample: 0,
        endSample: null,
        crossfadeMs: CROSSFADE_MS,
        method: 'crossfade',
      },
      nook: s.nook,
      provenance: {
        origin: 'placeholder:elpy-1.1.4.0',
        licence: 'UNLICENSED-DEV-PLACEHOLDER',
        attribution: 'Vane Jung — Elpy. Development reference only, must not ship.',
        shippable: false,
      },
    });
  }

  const catalogue = {
    schema: 1,
    pack: 'placeholder',
    packName: 'Development Placeholders',
    sounds: entries,
  };
  writeFileSync(join(OUT, 'catalogue.json'), JSON.stringify(catalogue, null, 2) + '\n');
  writeFileSync(
    join(OUT, 'DO-NOT-SHIP.md'),
    '# Development placeholders\n\n' +
      'Audio and cover art in this pack are extracted from Elpy 1.1.4.0 (© Vane Jung)\n' +
      'and are here only so the audio engine has real material to loop during\n' +
      'development. Every entry is marked `shippable: false`.\n\n' +
      'Nothing in this folder may appear in a public Nuru build.\n',
  );

  const total = entries.reduce((n, e) => n + e.audio.bytes, 0);
  console.log(`\n${entries.length} sounds · ${(total / 1e6).toFixed(0)} MB → ${OUT}`);
}

main();
