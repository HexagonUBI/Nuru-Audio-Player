import { execFileSync, spawnSync } from 'node:child_process';
import { unlinkSync, writeFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { createHash } from 'node:crypto';
import { UPLOADS } from './db.mjs';
import { FFMPEG, FFPROBE } from './config.mjs';

export const ACCEPTED = ['.flac', '.wav', '.mp3', '.ogg', '.oga', '.m4a', '.aac', '.opus', '.aiff', '.aif'];

export const LICENCES = [
  { id: 'CC0-1.0', name: 'CC0 1.0', needsAttribution: false, note: 'Public domain dedication. Anyone may use it for anything.' },
  { id: 'CC-BY-4.0', name: 'CC BY 4.0', needsAttribution: true, note: 'Free to use with credit to the recordist.' },
  { id: 'CC-BY-SA-4.0', name: 'CC BY-SA 4.0', needsAttribution: true, note: 'Credit required, and derivatives must share the same licence.' },
  { id: 'CC-PDM-1.0', name: 'Public Domain Mark', needsAttribution: false, note: 'No known copyright. Asserted by the uploader, not the creator.' },
  { id: 'OWN-WORK', name: 'My own recording', needsAttribution: false, note: 'You recorded it and you are releasing it here.' },
];

export function haveFfmpeg() {
  const a = spawnSync(FFMPEG, ['-version'], { encoding: 'utf8' });
  const b = spawnSync(FFPROBE, ['-version'], { encoding: 'utf8' });
  return a.status === 0 && b.status === 0;
}

export function probe(file) {
  const out = execFileSync(
    FFPROBE,
    ['-v', 'quiet', '-print_format', 'json', '-show_streams', '-show_format', file],
    { encoding: 'utf8' },
  );
  const d = JSON.parse(out);
  const a = (d.streams ?? []).find((s) => s.codec_type === 'audio');
  if (!a) throw new Error('no audio stream in that file');
  return {
    sampleRate: Number(a.sample_rate ?? 48000),
    channels: Number(a.channels ?? 2),
    duration: Number(d.format?.duration ?? 0),
    codec: a.codec_name ?? 'unknown',
  };
}

export function toFlac(inputBuffer, slug, ext) {
  const tmp = join(UPLOADS, `.incoming-${slug}${ext}`);
  const out = join(UPLOADS, `${slug}.flac`);
  writeFileSync(tmp, inputBuffer);
  try {
    const info = probe(tmp);
    execFileSync(FFMPEG, [
      '-hide_banner', '-loglevel', 'error', '-y',
      '-i', tmp,
      '-map', 'a:0',
      '-ac', '2',
      '-ar', '48000',
      '-c:a', 'flac', '-compression_level', '8', '-sample_fmt', 's16',
      out,
    ]);
    const after = probe(out);
    return { sourceCodec: info.codec, sourceRate: info.sampleRate, ...after, path: out };
  } finally {
    if (existsSync(tmp)) unlinkSync(tmp);
  }
}

export function peaks(file, buckets = 480) {
  const raw = execFileSync(
    FFMPEG,
    ['-v', 'quiet', '-i', file, '-ac', '1', '-ar', '4000', '-f', 's16le', '-'],
    { maxBuffer: 1024 * 1024 * 64, encoding: 'buffer' },
  );
  const n = Math.floor(raw.length / 2);
  if (!n) return [];
  const per = Math.max(1, Math.floor(n / buckets));
  const out = [];
  for (let b = 0; b < buckets; b++) {
    let hi = 0;
    const start = b * per;
    if (start >= n) break;
    for (let i = start; i < Math.min(start + per, n); i++) {
      const v = Math.abs(raw.readInt16LE(i * 2));
      if (v > hi) hi = v;
    }
    out.push(Math.round((hi / 32768) * 1000) / 1000);
  }
  const top = Math.max(...out, 0.0001);
  return out.map((v) => Math.round((v / top) * 1000) / 1000);
}

export function coverFile(buffer, slug) {
  const tmp = join(UPLOADS, `.cover-${slug}`);
  const out = join(UPLOADS, `${slug}-cover.jpg`);
  writeFileSync(tmp, buffer);
  try {
    execFileSync(FFMPEG, [
      '-hide_banner', '-loglevel', 'error', '-y',
      '-i', tmp,
      '-vf', 'scale=480:480:force_original_aspect_ratio=increase,crop=480:480',
      '-q:v', '4',
      out,
    ]);
    return `${slug}-cover.jpg`;
  } finally {
    if (existsSync(tmp)) unlinkSync(tmp);
  }
}

export function sha256(buf) {
  return createHash('sha256').update(buf).digest('hex');
}
