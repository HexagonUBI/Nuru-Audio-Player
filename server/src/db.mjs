import { DatabaseSync } from 'node:sqlite';
import { mkdirSync } from 'node:fs';
import { dirname, isAbsolute, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { DATA_DIR } from './config.mjs';

const here = dirname(fileURLToPath(import.meta.url));
export const ROOT = join(here, '..');
export const DATA = DATA_DIR
  ? (isAbsolute(DATA_DIR) ? DATA_DIR : resolve(ROOT, DATA_DIR))
  : join(ROOT, 'data');
export const UPLOADS = join(DATA, 'uploads');

const SCHEMA = `
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS users (
  id           INTEGER PRIMARY KEY,
  email        TEXT NOT NULL UNIQUE,
  name         TEXT NOT NULL,
  handle       TEXT UNIQUE,
  bio          TEXT NOT NULL DEFAULT '',
  pass_hash    TEXT NOT NULL,
  pass_salt    TEXT NOT NULL,
  role         TEXT NOT NULL DEFAULT 'user',
  created_at   INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
  token        TEXT PRIMARY KEY,
  user_id      INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  created_at   INTEGER NOT NULL,
  expires_at   INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS sounds (
  id           INTEGER PRIMARY KEY,
  slug         TEXT NOT NULL UNIQUE,
  name         TEXT NOT NULL,
  description  TEXT NOT NULL DEFAULT '',
  uploader_id  INTEGER REFERENCES users(id) ON DELETE SET NULL,
  status       TEXT NOT NULL DEFAULT 'review',
  licence      TEXT NOT NULL DEFAULT 'CC0-1.0',
  attribution  TEXT,
  accent       TEXT NOT NULL DEFAULT '#8FB8FF',
  duration_s   REAL NOT NULL DEFAULT 0,
  sample_rate  INTEGER NOT NULL DEFAULT 48000,
  channels     INTEGER NOT NULL DEFAULT 2,
  bytes        INTEGER NOT NULL DEFAULT 0,
  sha256       TEXT NOT NULL DEFAULT '',
  audio_path   TEXT,
  cover_path   TEXT,
  peaks        TEXT,
  source_codec TEXT,
  nook_channel TEXT NOT NULL DEFAULT 'none',
  nook_state   TEXT NOT NULL DEFAULT 'none',
  nook_weight  REAL NOT NULL DEFAULT 0.5,
  created_at   INTEGER NOT NULL,
  reviewed_at  INTEGER,
  reviewed_by  INTEGER REFERENCES users(id) ON DELETE SET NULL,
  review_note  TEXT
);

CREATE TABLE IF NOT EXISTS tags (
  id           INTEGER PRIMARY KEY,
  name         TEXT NOT NULL UNIQUE,
  category     TEXT NOT NULL DEFAULT 'other',
  status       TEXT NOT NULL DEFAULT 'official',
  requested_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
  reason       TEXT,
  created_at   INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS reports (
  id           INTEGER PRIMARY KEY,
  kind         TEXT NOT NULL,
  target       TEXT NOT NULL,
  reporter_id  INTEGER REFERENCES users(id) ON DELETE SET NULL,
  reason       TEXT NOT NULL,
  note         TEXT NOT NULL DEFAULT '',
  status       TEXT NOT NULL DEFAULT 'open',
  outcome      TEXT,
  created_at   INTEGER NOT NULL,
  resolved_at  INTEGER,
  resolved_by  INTEGER REFERENCES users(id) ON DELETE SET NULL
);



CREATE TABLE IF NOT EXISTS sound_tags (
  sound_id     INTEGER NOT NULL REFERENCES sounds(id) ON DELETE CASCADE,
  tag_id       INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
  PRIMARY KEY (sound_id, tag_id)
);

CREATE TABLE IF NOT EXISTS mixtapes (
  id           INTEGER PRIMARY KEY,
  slug         TEXT NOT NULL UNIQUE,
  title        TEXT NOT NULL,
  description  TEXT NOT NULL DEFAULT '',
  author_id    INTEGER REFERENCES users(id) ON DELETE CASCADE,
  accent       TEXT NOT NULL DEFAULT '#ffb454',
  cover_path   TEXT,
  status       TEXT NOT NULL DEFAULT 'draft',
  created_at   INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS mixtape_sounds (
  mixtape_id   INTEGER NOT NULL REFERENCES mixtapes(id) ON DELETE CASCADE,
  sound_id     INTEGER NOT NULL REFERENCES sounds(id) ON DELETE CASCADE,
  position     INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (mixtape_id, sound_id)
);

CREATE INDEX IF NOT EXISTS idx_sounds_status ON sounds(status);
CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id);
`;

let db = null;

const MIGRATIONS = [
  ['users', 'handle', 'TEXT'],
  ['users', 'bio', "TEXT NOT NULL DEFAULT ''"],
  ['sounds', 'cover_path', 'TEXT'],
  ['sounds', 'peaks', 'TEXT'],
  ['sounds', 'source_codec', 'TEXT'],
  ['mixtapes', 'cover_path', 'TEXT'],
  ['mixtapes', 'review_note', 'TEXT'],
  ['tags', 'category', "TEXT NOT NULL DEFAULT 'other'"],
  ['tags', 'status', "TEXT NOT NULL DEFAULT 'official'"],
  ['tags', 'requested_by', 'INTEGER'],
  ['tags', 'reason', 'TEXT'],
  ['tags', 'created_at', 'INTEGER NOT NULL DEFAULT 0'],
];

function migrate(handle) {
  for (const [table, column, type] of MIGRATIONS) {
    const cols = handle.prepare(`PRAGMA table_info(${table})`).all();
    if (!cols.length) continue;
    if (cols.some((c) => c.name === column)) continue;
    handle.exec(`ALTER TABLE ${table} ADD COLUMN ${column} ${type}`);
  }
}

const LATE_INDEXES = `
CREATE INDEX IF NOT EXISTS idx_reports_status ON reports(status);
CREATE INDEX IF NOT EXISTS idx_tags_status ON tags(status);
`;

export function open() {
  if (db) return db;
  mkdirSync(UPLOADS, { recursive: true });
  db = new DatabaseSync(join(DATA, 'nuru.db'));
  db.exec(SCHEMA);
  migrate(db);
  db.exec(LATE_INDEXES);
  return db;
}

export function all(sql, params = {}) {
  return open().prepare(sql).all(params);
}

export function one(sql, params = {}) {
  return open().prepare(sql).get(params) ?? null;
}

export function run(sql, params = {}) {
  return open().prepare(sql).run(params);
}

export function tagsFor(soundId) {
  return all(
    `SELECT t.name FROM tags t
     JOIN sound_tags st ON st.tag_id = t.id
     WHERE st.sound_id = :id ORDER BY t.name`,
    { id: soundId },
  ).map((r) => r.name);
}

export function setTags(soundId, names) {
  run('DELETE FROM sound_tags WHERE sound_id = :id', { id: soundId });
  let skipped = 0;
  for (const raw of names.slice(0, 8)) {
    const name = String(raw).trim().toLowerCase();
    if (!name) continue;
    const tag = one("SELECT id FROM tags WHERE name = :name AND status = 'official'", { name });
    if (!tag) {
      skipped += 1;
      continue;
    }
    run('INSERT OR IGNORE INTO sound_tags (sound_id, tag_id) VALUES (:s, :t)', {
      s: soundId,
      t: tag.id,
    });
  }
  return skipped;
}

export function openReportCount(kind, target) {
  const r = one(
    "SELECT COUNT(*) AS n FROM reports WHERE kind = :k AND target = :t AND status = 'open'",
    { k: kind, t: target },
  );
  return r ? r.n : 0;
}

export function mixtapeFor(soundId) {
  const r = one(
    `SELECT m.slug, m.title FROM mixtape_sounds ms
     JOIN mixtapes m ON m.id = ms.mixtape_id
     WHERE ms.sound_id = :id AND m.status = 'public'
     LIMIT 1`,
    { id: soundId },
  );
  return r ? { slug: r.slug, title: r.title } : null;
}

export function mixtapeSounds(mixtapeId) {
  return all(
    `SELECT s.*, u.name AS uploader, u.handle AS uploader_handle
     FROM mixtape_sounds ms
     JOIN sounds s ON s.id = ms.sound_id
     LEFT JOIN users u ON u.id = s.uploader_id
     WHERE ms.mixtape_id = :id
     ORDER BY ms.position, s.name`,
    { id: mixtapeId },
  );
}

export function shapeMixtape(row, includeSounds = true) {
  if (!row) return null;
  const sounds = includeSounds ? mixtapeSounds(row.id).map(shape) : [];
  return {
    slug: row.slug,
    title: row.title,
    description: row.description,
    accent: row.accent,
    status: row.status,
    author: row.author ?? null,
    authorHandle: row.author_handle ?? null,
    hasCover: Boolean(row.cover_path),
    createdAt: row.created_at,
    reviewNote: row.review_note ?? null,
    openReports: openReportCount('mixtape', row.slug),
    count: includeSounds
      ? sounds.length
      : all('SELECT COUNT(*) AS n FROM mixtape_sounds WHERE mixtape_id = :id', { id: row.id })[0].n,
    sounds,
    installUrl: `nuru://mixtape/${row.slug}`,
  };
}

export function shape(row) {
  if (!row) return null;
  return {
    slug: row.slug,
    name: row.name,
    description: row.description,
    status: row.status,
    licence: row.licence,
    attribution: row.attribution,
    accent: row.accent,
    uploader: row.uploader ?? null,
    durationSeconds: row.duration_s,
    sampleRate: row.sample_rate,
    channels: row.channels,
    bytes: row.bytes,
    sha256: row.sha256,
    hasAudio: Boolean(row.audio_path),
    mixtape: row.mixtape_slug
      ? { slug: row.mixtape_slug, title: row.mixtape_title }
      : mixtapeFor(row.id),
    hasCover: Boolean(row.cover_path),
    sourceCodec: row.source_codec,
    peaks: row.peaks ? JSON.parse(row.peaks) : null,
    uploaderHandle: row.uploader_handle ?? null,
    nook: { channel: row.nook_channel, state: row.nook_state, weight: row.nook_weight },
    createdAt: row.created_at,
    openReports: openReportCount('sound', row.slug),
    reviewedAt: row.reviewed_at,
    reviewNote: row.review_note,
    tags: tagsFor(row.id),
    installUrl: `nuru://install/${row.slug}`,
  };
}
