import { readFileSync, existsSync, copyFileSync } from 'node:fs';
import { join } from 'node:path';
import { open, one, run, setTags, UPLOADS, ROOT } from './src/db.mjs';
import { hashPassword } from './src/auth.mjs';
import { peaks } from './src/media.mjs';

const PACKS = join(ROOT, '..', 'resources', 'packs');

const PEOPLE = [
  {
    email: 'mod@nuru.local',
    name: 'Moderator',
    handle: 'moderator',
    bio: 'Keeps the queue moving. Local development account.',
    password: 'nuru-dev-moderator',
    role: 'moderator',
  },
  {
    email: 'sam@nuru.local',
    name: 'Sam Rivers',
    handle: 'sam-rivers',
    bio: 'Field recordist. Rain, water and anything with a long tail.',
    password: 'nuru-dev-uploader',
    role: 'user',
  },
];

function upsertUser(p) {
  const found = one('SELECT id FROM users WHERE email = :email', { email: p.email });
  if (found) return found.id;
  const { hash, salt } = hashPassword(p.password);
  run(
    `INSERT INTO users (email, name, handle, bio, pass_hash, pass_salt, role, created_at)
     VALUES (:email, :name, :handle, :bio, :hash, :salt, :role, :now)`,
    {
      email: p.email,
      name: p.name,
      handle: p.handle,
      bio: p.bio,
      hash,
      salt,
      role: p.role,
      now: Date.now(),
    },
  );
  return one('SELECT id FROM users WHERE email = :email', { email: p.email }).id;
}

function backfill(row, dir, s) {
  if (!row.peaks) {
    const audio = join(UPLOADS, `${s.id}.flac`);
    if (existsSync(audio)) {
      try {
        run('UPDATE sounds SET peaks = :w, source_codec = :c WHERE id = :id', {
          w: JSON.stringify(peaks(audio)),
          c: 'flac',
          id: row.id,
        });
      } catch {
        void 0;
      }
    }
  }
  if (!row.cover_path && s.cover) {
    const cov = join(PACKS, dir, s.cover);
    if (existsSync(cov)) {
      const cname = `${s.id}-cover.jpg`;
      copyFileSync(cov, join(UPLOADS, cname));
      run('UPDATE sounds SET cover_path = :p WHERE id = :id', { p: cname, id: row.id });
    }
  }
}

function seedPack(dir, uploaderId, status) {
  const manifest = join(PACKS, dir, 'catalogue.json');
  if (!existsSync(manifest)) return 0;
  const cat = JSON.parse(readFileSync(manifest, 'utf8'));
  let n = 0;
  for (const s of cat.sounds) {
    const existing = one('SELECT id, peaks, cover_path FROM sounds WHERE slug = :slug', {
      slug: s.id,
    });
    if (existing) {
      backfill(existing, dir, s);
      continue;
    }
    run(
      `INSERT INTO sounds
        (slug, name, description, uploader_id, status, licence, attribution, accent,
         duration_s, sample_rate, channels, bytes, sha256, audio_path,
         nook_channel, nook_state, nook_weight, created_at)
       VALUES
        (:slug, :name, :desc, :uploader, :status, :licence, :attribution, :accent,
         :dur, :sr, :ch, :bytes, :sha, :path, :nc, :ns, :nw, :now)`,
      {
        slug: s.id,
        name: s.name,
        desc: `${cat.packName} field material.`,
        uploader: uploaderId,
        status,
        licence: s.provenance.licence,
        attribution: s.provenance.attribution,
        accent: s.accent,
        dur: s.audio.frames / s.audio.sampleRate,
        sr: s.audio.sampleRate,
        ch: s.audio.channels,
        bytes: s.audio.bytes,
        sha: s.audio.sha256,
        path: null,
        nc: s.nook.channel,
        ns: s.nook.state,
        nw: s.nook.weight,
        now: Date.now(),
      },
    );
    const row = one('SELECT id FROM sounds WHERE slug = :slug', { slug: s.id });
    setTags(row.id, s.tags);

    const src = join(PACKS, dir, s.audio.file);
    if (existsSync(src)) {
      const name = `${s.id}.flac`;
      const dest = join(UPLOADS, name);
      copyFileSync(src, dest);
      let wave = [];
      try {
        wave = peaks(dest);
      } catch {
        wave = [];
      }
      run('UPDATE sounds SET audio_path = :p, peaks = :w, source_codec = :c WHERE id = :id', {
        p: name,
        w: JSON.stringify(wave),
        c: 'flac',
        id: row.id,
      });
    }
    const cov = join(PACKS, dir, s.cover ?? '');
    if (s.cover && existsSync(cov)) {
      const cname = `${s.id}-cover.jpg`;
      copyFileSync(cov, join(UPLOADS, cname));
      run('UPDATE sounds SET cover_path = :p WHERE id = :id', { p: cname, id: row.id });
    }
    n += 1;
  }
  return n;
}

open();
const modId = upsertUser(PEOPLE[0]);
const samId = upsertUser(PEOPLE[1]);

const published = seedPack('field', samId, 'public');
const pending = seedPack('builtin', samId, 'review');

if (!one('SELECT id FROM sounds WHERE slug = :s', { s: 'attic-rain-demo' })) {
  run(
    `INSERT INTO sounds
      (slug, name, description, uploader_id, status, licence, attribution, accent,
       duration_s, sample_rate, channels, nook_channel, nook_state, nook_weight, created_at)
     VALUES
      ('attic-rain-demo', 'Attic Rain', 'Submitted for review so the moderation queue has something in it.',
       :uploader, 'review', 'CC0-1.0', 'Sam Rivers', '#B4E4FC',
       61.0, 48000, 2, 'weather', 'rain', 0.8, :now)`,
    { uploader: samId, now: Date.now() },
  );
  const demo = one('SELECT id FROM sounds WHERE slug = :s', { s: 'attic-rain-demo' });
  setTags(demo.id, ['rain', 'indoor', 'night']);
}

for (const u of PEOPLE) {
  run('UPDATE users SET handle = :h, bio = :b WHERE email = :e AND handle IS NULL', {
    h: u.handle,
    b: u.bio,
    e: u.email,
  });
}

console.log(`users     : ${PEOPLE.map((p) => p.email).join(', ')}`);
console.log(`published : ${published} from the field pack`);
console.log(`in review : ${pending + 1}`);
console.log('');
console.log('sign in with mod@nuru.local / nuru-dev-moderator for the moderation queue');
console.log('these are local development credentials, do not reuse them anywhere real');
void modId;
