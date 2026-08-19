import { readFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { all, one, run, setTags, shape, shapeMixtape, UPLOADS } from './db.mjs';
import { unlinkSync } from 'node:fs';
import { ACCEPTED, LICENCES, coverFile, peaks, sha256, toFlac } from './media.mjs';
import { LIMITS, OFFICIAL_TAGS, REPORT_REASONS, clip, tooLong } from './taxonomy.mjs';
import { IS_PUBLIC, MODERATOR_EMAIL, OPEN_SIGNUP } from './config.mjs';
import {
  clearCookie,
  cookieToken,
  createSession,
  dropSession,
  hashPassword,
  sessionCookie,
  userForToken,
  verifyPassword,
} from './auth.mjs';

const STATUSES = ['review', 'public', 'private', 'declined'];

const EMAIL = /^[^\s@]+@[^\s@]+\.[^\s@]{2,}$/;
const NO_FFMPEG = 'this workshop has no ffmpeg installed, so it cannot accept uploads right now';
const DECOY_SALT = '0f1e2d3c4b5a69788796a5b4c3d2e1f0';

const WEAK = new Set([
  'password', 'password1', 'password12', 'password123', 'passw0rd12',
  '1234567890', '12345678901', '123456789012', 'qwertyuiop', 'letmein123',
  'iloveyou12', 'welcome123', 'admin12345', 'nuruworkshop', 'changeme123',
]);

function normaliseEmail(value) {
  return String(value ?? '').trim().toLowerCase();
}

function roleFor(address) {
  if (MODERATOR_EMAIL) return address === MODERATOR_EMAIL ? 'moderator' : 'user';
  if (IS_PUBLIC) return 'user';
  return one('SELECT id FROM users LIMIT 1') ? 'user' : 'moderator';
}

const RULES = [
  {
    title: 'Upload things people can actually use',
    body: 'A soundscape should be at least 20 seconds of steady ambience that can loop. Clips, one shots, stings and half second noises are not soundscapes and will be declined.',
  },
  {
    title: 'It has to be yours to give',
    body: 'Upload your own recordings, or material that is genuinely CC0 or public domain. If a licence asks for credit, put the recordist in the attribution field. No commercial music, and nothing ripped out of another app.',
  },
  {
    title: 'Ambience only',
    body: 'No speech you can make out, no songs, no radio. Thumbnails with logos or watermarks get declined too.',
  },
  {
    title: 'Name and tag it honestly',
    body: 'The name should say what it is. Tags should describe the sound rather than chase attention. Three to six honest tags beat twenty guesses.',
  },
  {
    title: 'Clean at the edges',
    body: 'Watch for hard clipping, wind buffeting on the mic, notifications and sudden bangs. A quiet recording is fine. A broken one is not.',
  },
  {
    title: 'Mixtapes are for sets that belong together',
    body: 'Group your own soundscapes into a mixtape when they are meant to be heard as one place. Two or more, sharing a mood, rather than everything you have ever uploaded.',
  },
  {
    title: 'Fix it rather than reupload it',
    body: 'Editing details costs nothing. Replacing the audio sends it back for review, which is fine. The same sound three times under three names is not.',
  },
];

function slugify(name) {
  return String(name)
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 60);
}

function uniqueSlug(base) {
  let slug = base || 'sound';
  let n = 1;
  while (one('SELECT id FROM sounds WHERE slug = :slug', { slug })) {
    n += 1;
    slug = `${base}-${n}`;
  }
  return slug;
}

function withUploader(sql) {
  return sql.replace('FROM sounds', 'FROM sounds LEFT JOIN users ON users.id = sounds.uploader_id');
}

const SELECT = `SELECT sounds.*, users.name AS uploader, users.handle AS uploader_handle
  FROM sounds LEFT JOIN users ON users.id = sounds.uploader_id`;

function mixRow(where) {
  const key = where.slug !== undefined ? 'm.slug = :slug' : 'm.id = :id';
  return one(
    `SELECT m.*, u.name AS author, u.handle AS author_handle
     FROM mixtapes m LEFT JOIN users u ON u.id = m.author_id WHERE ${key}`,
    where,
  );
}

export const routes = {
  'POST /api/register': (ctx) => {
    if (!OPEN_SIGNUP) return ctx.fail(403, 'registration is closed on this workshop');
    const { email, name, password } = ctx.body ?? {};
    if (!email || !name || !password) return ctx.fail(400, 'email, name and password are required');
    const address = normaliseEmail(email);
    if (!EMAIL.test(address)) return ctx.fail(400, 'that does not look like an email address');
    if (String(password).length < 10) {
      return ctx.fail(400, 'password must be at least 10 characters');
    }
    if (WEAK.has(String(password).toLowerCase())) {
      return ctx.fail(400, 'pick a password that is not on every guess list');
    }
    const display = clip(String(name).trim(), LIMITS.displayName);
    if (!display) return ctx.fail(400, 'a display name is required');
    if (one('SELECT id FROM users WHERE email = :email', { email: address })) {
      return ctx.fail(409, 'that email is already registered');
    }
    const { hash, salt } = hashPassword(String(password));
    let handle = slugify(display) || 'listener';
    let n = 1;
    while (one('SELECT id FROM users WHERE handle = :h', { h: handle })) {
      n += 1;
      handle = `${slugify(display)}-${n}`;
    }
    run(
      `INSERT INTO users (email, name, handle, pass_hash, pass_salt, role, created_at)
       VALUES (:email, :name, :handle, :hash, :salt, :role, :now)`,
      {
        email: address,
        name: display,
        handle,
        hash,
        salt,
        role: roleFor(address),
        now: Date.now(),
      },
    );
    const user = one('SELECT id, email, name, handle, role FROM users WHERE email = :email', {
      email: address,
    });
    const token = createSession(user.id);
    ctx.setCookie(sessionCookie(token));
    return { user };
  },

  'POST /api/login': (ctx) => {
    const { email, password } = ctx.body ?? {};
    const row = one('SELECT * FROM users WHERE email = :email', { email: normaliseEmail(email) });
    const given = String(password ?? '');
    if (!row) {
      hashPassword(given, DECOY_SALT);
      return ctx.fail(401, 'wrong email or password');
    }
    if (!verifyPassword(given, row.pass_hash, row.pass_salt)) {
      return ctx.fail(401, 'wrong email or password');
    }
    const token = createSession(row.id);
    ctx.setCookie(sessionCookie(token));
    return {
      user: { id: row.id, email: row.email, name: row.name, handle: row.handle, role: row.role },
    };
  },

  'POST /api/logout': (ctx) => {
    dropSession(cookieToken(ctx.req));
    ctx.setCookie(clearCookie);
    return { ok: true };
  },

  'GET /api/me': (ctx) => ({ user: ctx.user }),

  'GET /api/licences': () => ({ licences: LICENCES, accepted: ACCEPTED }),

  'GET /api/users/:handle': (ctx) => {
    const u = one(
      'SELECT id, name, handle, bio, role, created_at FROM users WHERE handle = :h',
      { h: ctx.params.handle },
    );
    if (!u) return ctx.fail(404, 'no such profile');
    const mine = ctx.user?.id === u.id || ctx.user?.role === 'moderator';
    const rows = all(
      `${SELECT} WHERE sounds.uploader_id = :id ${mine ? '' : "AND sounds.status = 'public'"}
       ORDER BY sounds.created_at DESC`,
      { id: u.id },
    );
    return {
      profile: {
        name: u.name,
        handle: u.handle,
        bio: u.bio,
        role: u.role,
        joined: u.created_at,
        published: all(
          "SELECT COUNT(*) AS n FROM sounds WHERE uploader_id = :id AND status = 'public'",
          { id: u.id },
        )[0].n,
      },
      sounds: rows.map(shape),
    };
  },

  'PATCH /api/me': (ctx) => {
    if (!ctx.user) return ctx.fail(401, 'sign in first');
    if (tooLong(ctx.body?.bio, LIMITS.bio)) {
      return ctx.fail(400, `the bio has to be ${LIMITS.bio} characters or fewer`);
    }
    const bio = clip(ctx.body?.bio, LIMITS.bio);
    run('UPDATE users SET bio = :bio WHERE id = :id', { bio, id: ctx.user.id });
    return { ok: true, bio };
  },

  'GET /api/tags': () => {
    const rows = all(`SELECT t.name, t.category, t.status, COUNT(st.sound_id) AS uses
       FROM tags t LEFT JOIN sound_tags st ON st.tag_id = t.id
       GROUP BY t.id ORDER BY t.category, t.name`);
    const official = rows.filter((r) => r.status === 'official');
    const grouped = {};
    for (const r of official) (grouped[r.category] ??= []).push({ name: r.name, uses: r.uses });
    return { tags: official, grouped, limits: LIMITS };
  },

  'GET /api/limits': (ctx) => ({
    limits: LIMITS,
    reportReasons: REPORT_REASONS,
    uploads: Boolean(ctx.ffmpeg),
    openSignup: OPEN_SIGNUP,
    live: IS_PUBLIC,
  }),

  'POST /api/tags/request': (ctx) => {
    if (!ctx.user) return ctx.fail(401, 'sign in first');
    const name = String(ctx.body?.name ?? '').trim().toLowerCase();
    if (!name) return ctx.fail(400, 'a tag name is required');
    if (tooLong(name, LIMITS.tagName)) {
      return ctx.fail(400, `keep tag names under ${LIMITS.tagName} characters`);
    }
    if (!/^[a-z0-9][a-z0-9 -]*$/.test(name)) {
      return ctx.fail(400, 'tags can only use lowercase letters, numbers, spaces and hyphens');
    }
    const existing = one('SELECT status FROM tags WHERE name = :name', { name });
    if (existing) {
      return ctx.fail(409, `that tag already exists and is ${existing.status}`);
    }
    run(
      `INSERT INTO tags (name, category, status, requested_by, reason, created_at)
       VALUES (:name, 'other', 'requested', :by, :reason, :now)`,
      {
        name,
        by: ctx.user.id,
        reason: clip(ctx.body?.reason, LIMITS.tagRequestReason),
        now: Date.now(),
      },
    );
    return { ok: true, name, status: 'requested' };
  },

  'GET /api/mod/tags': (ctx) => {
    if (ctx.user?.role !== 'moderator') return ctx.fail(403, 'moderators only');
    return {
      requested: all(
        `SELECT t.name, t.reason, t.created_at, u.name AS by, u.handle AS handle
         FROM tags t LEFT JOIN users u ON u.id = t.requested_by
         WHERE t.status = 'requested' ORDER BY t.created_at`,
      ),
    };
  },

  'POST /api/mod/tags/:name/decision': (ctx) => {
    if (ctx.user?.role !== 'moderator') return ctx.fail(403, 'moderators only');
    const name = String(ctx.params.name).toLowerCase();
    const row = one('SELECT * FROM tags WHERE name = :name', { name });
    if (!row) return ctx.fail(404, 'no such tag');
    const d = ctx.body?.decision;
    if (d === 'approve') {
      const category = String(ctx.body?.category ?? 'other');
      run("UPDATE tags SET status = 'official', category = :c WHERE id = :id", {
        c: Object.keys(OFFICIAL_TAGS).includes(category) ? category : 'other',
        id: row.id,
      });
    } else if (d === 'reject') {
      run('DELETE FROM sound_tags WHERE tag_id = :id', { id: row.id });
      run('DELETE FROM tags WHERE id = :id', { id: row.id });
    } else {
      return ctx.fail(400, 'decision must be approve or reject');
    }
    return { ok: true, name, decision: d };
  },

  'POST /api/reports': (ctx) => {
    if (!ctx.user) return ctx.fail(401, 'sign in to report');
    const kind = String(ctx.body?.kind ?? '');
    if (!['sound', 'mixtape'].includes(kind)) return ctx.fail(400, 'unknown report kind');
    const reason = String(ctx.body?.reason ?? '');
    if (!REPORT_REASONS.some((r) => r.id === reason)) return ctx.fail(400, 'unknown reason');
    const target = String(ctx.body?.target ?? '');
    const table = kind === 'sound' ? 'sounds' : 'mixtapes';
    if (!one(`SELECT id FROM ${table} WHERE slug = :t`, { t: target })) {
      return ctx.fail(404, 'no such target');
    }
    const already = one(
      `SELECT id FROM reports WHERE kind = :k AND target = :t AND reporter_id = :r
       AND status = 'open'`,
      { k: kind, t: target, r: ctx.user.id },
    );
    if (already) return ctx.fail(409, 'you already reported that, a moderator will look');
    run(
      `INSERT INTO reports (kind, target, reporter_id, reason, note, status, created_at)
       VALUES (:k, :t, :r, :reason, :note, 'open', :now)`,
      {
        k: kind,
        t: target,
        r: ctx.user.id,
        reason,
        note: clip(ctx.body?.note, LIMITS.reportNote),
        now: Date.now(),
      },
    );
    return { ok: true };
  },

  'GET /api/mod/reports': (ctx) => {
    if (ctx.user?.role !== 'moderator') return ctx.fail(403, 'moderators only');
    return {
      reports: all(
        `SELECT r.*, u.name AS reporter, u.handle AS reporter_handle
         FROM reports r LEFT JOIN users u ON u.id = r.reporter_id
         WHERE r.status = 'open' ORDER BY r.created_at`,
      ).map((r) => ({
        id: r.id,
        kind: r.kind,
        target: r.target,
        reason: r.reason,
        note: r.note,
        reporter: r.reporter,
        reporterHandle: r.reporter_handle,
        createdAt: r.created_at,
      })),
    };
  },

  'POST /api/mod/reports/:id/resolve': (ctx) => {
    if (ctx.user?.role !== 'moderator') return ctx.fail(403, 'moderators only');
    const row = one('SELECT * FROM reports WHERE id = :id', { id: Number(ctx.params.id) });
    if (!row) return ctx.fail(404, 'no such report');
    const outcome = String(ctx.body?.outcome ?? '');
    if (!['upheld', 'dismissed'].includes(outcome)) {
      return ctx.fail(400, 'outcome must be upheld or dismissed');
    }
    run(
      `UPDATE reports SET status = 'closed', outcome = :o, resolved_at = :now, resolved_by = :by
       WHERE id = :id`,
      { o: outcome, now: Date.now(), by: ctx.user.id, id: row.id },
    );
    if (outcome === 'upheld' && row.kind === 'sound') {
      run("UPDATE sounds SET status = 'review', review_note = :n WHERE slug = :t", {
        n: 'Pulled back for review after a report.',
        t: row.target,
      });
    }
    if (outcome === 'upheld' && row.kind === 'mixtape') {
      run("UPDATE mixtapes SET status = 'review', review_note = :n WHERE slug = :t", {
        n: 'Pulled back for review after a report.',
        t: row.target,
      });
    }
    return { ok: true, outcome };
  },

  'GET /api/mod/mixtapes': (ctx) => {
    if (ctx.user?.role !== 'moderator') return ctx.fail(403, 'moderators only');
    return {
      mixtapes: all(
        `SELECT m.*, u.name AS author, u.handle AS author_handle
         FROM mixtapes m LEFT JOIN users u ON u.id = m.author_id
         WHERE m.status = 'review' ORDER BY m.created_at`,
      ).map((r) => shapeMixtape(r)),
    };
  },

  'POST /api/mod/mixtapes/:slug/decision': (ctx) => {
    if (ctx.user?.role !== 'moderator') return ctx.fail(403, 'moderators only');
    const row = one('SELECT * FROM mixtapes WHERE slug = :slug', { slug: ctx.params.slug });
    if (!row) return ctx.fail(404, 'no such mixtape');
    const d = ctx.body?.decision;
    if (d !== 'accept' && d !== 'decline') return ctx.fail(400, 'decision must be accept or decline');
    run('UPDATE mixtapes SET status = :s, review_note = :n WHERE id = :id', {
      s: d === 'accept' ? 'public' : 'declined',
      n: clip(ctx.body?.note, LIMITS.reportNote) || null,
      id: row.id,
    });
    return { ok: true, decision: d };
  },

  'GET /api/sounds': (ctx) => {
    const q = (ctx.query.get('q') ?? '').trim().toLowerCase();
    const tag = (ctx.query.get('tag') ?? '').trim().toLowerCase();
    const status = ctx.query.get('status') ?? 'public';
    if (!STATUSES.includes(status)) return ctx.fail(400, 'unknown status');
    if (status !== 'public' && ctx.user?.role !== 'moderator') {
      return ctx.fail(403, 'only moderators can browse that status');
    }
    let rows = all(`${SELECT} WHERE sounds.status = :status ORDER BY sounds.created_at DESC`, {
      status,
    });
    if (q) {
      rows = rows.filter(
        (r) =>
          r.name.toLowerCase().includes(q) ||
          (r.description ?? '').toLowerCase().includes(q),
      );
    }
    let out = rows.map(shape);
    if (tag) out = out.filter((s) => s.tags.includes(tag));
    return { sounds: out, total: out.length };
  },

  'GET /api/sounds/:slug': (ctx) => {
    const row = one(`${SELECT} WHERE sounds.slug = :slug`, { slug: ctx.params.slug });
    if (!row) return ctx.fail(404, 'no such sound');
    if (row.status !== 'public' && ctx.user?.role !== 'moderator' && ctx.user?.id !== row.uploader_id) {
      return ctx.fail(403, 'that sound is not public');
    }
    return shape(row);
  },

  'POST /api/sounds': (ctx) => {
    if (!ctx.user) return ctx.fail(401, 'sign in to upload');
    const b = ctx.body ?? {};
    if (!b.name) return ctx.fail(400, 'name is required');
    if (tooLong(b.name, LIMITS.soundName)) {
      return ctx.fail(400, `the name has to be ${LIMITS.soundName} characters or fewer`);
    }
    if (tooLong(b.description, LIMITS.soundDescription)) {
      return ctx.fail(400, `the description has to be ${LIMITS.soundDescription} characters or fewer`);
    }
    if (tooLong(b.attribution, LIMITS.attribution)) {
      return ctx.fail(400, `the attribution has to be ${LIMITS.attribution} characters or fewer`);
    }
    if (Array.isArray(b.tags) && b.tags.length > LIMITS.tagsPerSound) {
      return ctx.fail(400, `pick at most ${LIMITS.tagsPerSound} tags`);
    }
    const slug = uniqueSlug(slugify(b.name));
    run(
      `INSERT INTO sounds
        (slug, name, description, uploader_id, status, licence, attribution, accent,
         nook_channel, nook_state, nook_weight, created_at)
       VALUES
        (:slug, :name, :desc, :uploader, 'review', :licence, :attribution, :accent,
         :ch, :st, :w, :now)`,
      {
        slug,
        name: String(b.name),
        desc: String(b.description ?? ''),
        uploader: ctx.user.id,
        licence: String(b.licence ?? 'CC0-1.0'),
        attribution: b.attribution ? String(b.attribution) : null,
        accent: String(b.accent ?? '#8FB8FF'),
        ch: String(b.nookChannel ?? 'none'),
        st: String(b.nookState ?? 'none'),
        w: Number(b.nookWeight ?? 0.5),
        now: Date.now(),
      },
    );
    const row = one(`${SELECT} WHERE sounds.slug = :slug`, { slug });
    const skipped = setTags(row.id, Array.isArray(b.tags) ? b.tags : []);
    return { ...shape(one(`${SELECT} WHERE sounds.slug = :slug`, { slug })), skippedTags: skipped };
  },

  'PATCH /api/sounds/:slug': (ctx) => {
    if (!ctx.user) return ctx.fail(401, 'sign in first');
    const row = one('SELECT * FROM sounds WHERE slug = :slug', { slug: ctx.params.slug });
    if (!row) return ctx.fail(404, 'no such sound');
    if (row.uploader_id !== ctx.user.id && ctx.user.role !== 'moderator') {
      return ctx.fail(403, 'that upload is not yours');
    }
    const b = ctx.body ?? {};
    if (tooLong(b.name, LIMITS.soundName)) {
      return ctx.fail(400, `the name has to be ${LIMITS.soundName} characters or fewer`);
    }
    if (tooLong(b.description, LIMITS.soundDescription)) {
      return ctx.fail(400, `the description has to be ${LIMITS.soundDescription} characters or fewer`);
    }
    if (Array.isArray(b.tags) && b.tags.length > LIMITS.tagsPerSound) {
      return ctx.fail(400, `pick at most ${LIMITS.tagsPerSound} tags`);
    }
    const set = {
      name: b.name !== undefined ? String(b.name) : row.name,
      desc: b.description !== undefined ? String(b.description) : row.description,
      licence: b.licence !== undefined ? String(b.licence) : row.licence,
      attribution:
        b.attribution !== undefined ? (b.attribution ? String(b.attribution) : null) : row.attribution,
      accent: b.accent !== undefined ? String(b.accent) : row.accent,
      nc: b.nookChannel !== undefined ? String(b.nookChannel) : row.nook_channel,
      ns: b.nookState !== undefined ? String(b.nookState) : row.nook_state,
      nw: b.nookWeight !== undefined ? Number(b.nookWeight) : row.nook_weight,
      id: row.id,
    };
    run(
      `UPDATE sounds SET name = :name, description = :desc, licence = :licence,
         attribution = :attribution, accent = :accent,
         nook_channel = :nc, nook_state = :ns, nook_weight = :nw
       WHERE id = :id`,
      set,
    );
    if (Array.isArray(b.tags)) setTags(row.id, b.tags);
    return shape(one(`${SELECT} WHERE sounds.id = :id`, { id: row.id }));
  },

  'PUT /api/sounds/:slug/audio': (ctx) => {
    if (!ctx.user) return ctx.fail(401, 'sign in to upload');
    const row = one('SELECT * FROM sounds WHERE slug = :slug', { slug: ctx.params.slug });
    if (!row) return ctx.fail(404, 'no such sound');
    if (row.uploader_id !== ctx.user.id && ctx.user.role !== 'moderator') {
      return ctx.fail(403, 'that upload is not yours');
    }
    if (!ctx.ffmpeg) return ctx.fail(503, NO_FFMPEG);
    if (!ctx.raw?.length) return ctx.fail(400, 'empty body');
    if (ctx.raw.length > 120 * 1024 * 1024) return ctx.fail(413, 'file is over 120 MB');

    const ext = String(ctx.query.get('ext') ?? '.flac').toLowerCase();
    if (!ACCEPTED.includes(ext)) {
      return ctx.fail(415, `${ext} is not supported. Accepted: ${ACCEPTED.join(' ')}`);
    }

    let info;
    try {
      info = toFlac(ctx.raw, row.slug, ext);
    } catch (e) {
      return ctx.fail(422, `could not decode that audio: ${String(e.message ?? e).slice(0, 160)}`);
    }

    const blob = readFileSync(info.path);
    let wave = [];
    try {
      wave = peaks(info.path);
    } catch {
      wave = [];
    }

    const wasPublic = row.status === 'public';
    run(
      `UPDATE sounds SET audio_path = :path, bytes = :bytes, sha256 = :sha,
         duration_s = :dur, sample_rate = :sr, channels = :ch,
         peaks = :peaks, source_codec = :codec, status = :status,
         reviewed_at = NULL, reviewed_by = NULL,
         review_note = :note
       WHERE id = :id`,
      {
        path: `${row.slug}.flac`,
        bytes: blob.length,
        sha: sha256(blob),
        dur: info.duration,
        sr: info.sampleRate,
        ch: info.channels,
        peaks: JSON.stringify(wave),
        codec: info.sourceCodec,
        status: 'review',
        note: wasPublic ? 'Audio was replaced, so this went back for review.' : null,
        id: row.id,
      },
    );
    return {
      ...shape(one(`${SELECT} WHERE sounds.id = :id`, { id: row.id })),
      returnedToReview: wasPublic,
    };
  },

  'PUT /api/sounds/:slug/cover': (ctx) => {
    if (!ctx.user) return ctx.fail(401, 'sign in to upload');
    const row = one('SELECT * FROM sounds WHERE slug = :slug', { slug: ctx.params.slug });
    if (!row) return ctx.fail(404, 'no such sound');
    if (row.uploader_id !== ctx.user.id && ctx.user.role !== 'moderator') {
      return ctx.fail(403, 'that upload is not yours');
    }
    if (!ctx.ffmpeg) return ctx.fail(503, NO_FFMPEG);
    if (!ctx.raw?.length) return ctx.fail(400, 'empty body');
    if (ctx.raw.length > 12 * 1024 * 1024) return ctx.fail(413, 'image is over 12 MB');
    let name;
    try {
      name = coverFile(ctx.raw, row.slug);
    } catch (e) {
      return ctx.fail(422, `could not read that image: ${String(e.message ?? e).slice(0, 160)}`);
    }
    run('UPDATE sounds SET cover_path = :p WHERE id = :id', { p: name, id: row.id });
    return shape(one(`${SELECT} WHERE sounds.id = :id`, { id: row.id }));
  },

  'GET /api/me/sounds': (ctx) => {
    if (!ctx.user) return ctx.fail(401, 'sign in first');
    const rows = all(`${SELECT} WHERE sounds.uploader_id = :id ORDER BY sounds.created_at DESC`, {
      id: ctx.user.id,
    });
    return { sounds: rows.map(shape) };
  },

  'GET /api/mod/queue': (ctx) => {
    if (ctx.user?.role !== 'moderator') return ctx.fail(403, 'moderators only');
    const rows = all(`${SELECT} WHERE sounds.status = 'review' ORDER BY sounds.created_at`);
    return { sounds: rows.map(shape) };
  },

  'POST /api/mod/:slug/decision': (ctx) => {
    if (ctx.user?.role !== 'moderator') return ctx.fail(403, 'moderators only');
    const decision = ctx.body?.decision;
    if (decision !== 'accept' && decision !== 'decline') {
      return ctx.fail(400, 'decision must be accept or decline');
    }
    const row = one('SELECT id FROM sounds WHERE slug = :slug', { slug: ctx.params.slug });
    if (!row) return ctx.fail(404, 'no such sound');
    run(
      `UPDATE sounds SET status = :status, reviewed_at = :now, reviewed_by = :by, review_note = :note
       WHERE id = :id`,
      {
        status: decision === 'accept' ? 'public' : 'declined',
        now: Date.now(),
        by: ctx.user.id,
        note: ctx.body?.note ? String(ctx.body.note) : null,
        id: row.id,
      },
    );
    return shape(one(`${SELECT} WHERE sounds.id = :id`, { id: row.id }));
  },

  'DELETE /api/sounds/:slug': (ctx) => {
    if (!ctx.user) return ctx.fail(401, 'sign in first');
    const row = one('SELECT * FROM sounds WHERE slug = :slug', { slug: ctx.params.slug });
    if (!row) return ctx.fail(404, 'no such sound');
    const mine = row.uploader_id === ctx.user.id;
    if (!mine && ctx.user.role !== 'moderator') {
      return ctx.fail(403, 'that upload is not yours');
    }
    for (const f of [row.audio_path, row.cover_path]) {
      if (!f) continue;
      try {
        unlinkSync(join(UPLOADS, f));
      } catch {
        void 0;
      }
    }
    run('DELETE FROM sound_tags WHERE sound_id = :id', { id: row.id });
    run('DELETE FROM mixtape_sounds WHERE sound_id = :id', { id: row.id });
    run('DELETE FROM sounds WHERE id = :id', { id: row.id });
    return { ok: true, deleted: row.slug, by: mine ? 'author' : 'moderator' };
  },

  'GET /api/rules': () => ({ rules: RULES }),

  'GET /api/mixtapes': (ctx) => {
    const handle = ctx.query.get('by');
    const canSeeDrafts =
      handle && (ctx.user?.handle === handle || ctx.user?.role === 'moderator');
    let sql = `SELECT m.*, u.name AS author, u.handle AS author_handle
       FROM mixtapes m LEFT JOIN users u ON u.id = m.author_id `;
    const params = {};
    if (handle) {
      params.handle = handle;
      sql += canSeeDrafts
        ? 'WHERE u.handle = :handle '
        : "WHERE u.handle = :handle AND m.status = 'public' ";
    } else {
      sql += "WHERE m.status = 'public' ";
    }
    sql += 'ORDER BY m.created_at DESC';
    return { mixtapes: all(sql, params).map((r) => shapeMixtape(r, false)) };
  },

  'GET /api/mixtapes/:slug': (ctx) => {
    const row = one(
      `SELECT m.*, u.name AS author, u.handle AS author_handle
       FROM mixtapes m LEFT JOIN users u ON u.id = m.author_id
       WHERE m.slug = :slug`,
      { slug: ctx.params.slug },
    );
    if (!row) return ctx.fail(404, 'no such mixtape');
    if (row.status !== 'public' && ctx.user?.id !== row.author_id && ctx.user?.role !== 'moderator') {
      return ctx.fail(403, 'that mixtape is not published');
    }
    return shapeMixtape(row);
  },

  'POST /api/mixtapes': (ctx) => {
    if (!ctx.user) return ctx.fail(401, 'sign in first');
    const title = String(ctx.body?.title ?? '').trim();
    if (!title) return ctx.fail(400, 'a title is required');
    if (tooLong(title, LIMITS.mixtapeTitle)) {
      return ctx.fail(400, `the title has to be ${LIMITS.mixtapeTitle} characters or fewer`);
    }
    if (tooLong(ctx.body?.description, LIMITS.mixtapeDescription)) {
      return ctx.fail(400, `the description has to be ${LIMITS.mixtapeDescription} characters or fewer`);
    }
    const base = slugify(title) || 'mixtape';
    let slug = base;
    let n = 1;
    while (one('SELECT id FROM mixtapes WHERE slug = :slug', { slug })) {
      n += 1;
      slug = base + '-' + n;
    }
    run(
      `INSERT INTO mixtapes (slug, title, description, author_id, accent, status, created_at)
       VALUES (:slug, :title, :desc, :author, :accent, 'draft', :now)`,
      {
        slug,
        title,
        desc: String(ctx.body?.description ?? ''),
        author: ctx.user.id,
        accent: String(ctx.body?.accent ?? '#ffb454'),
        now: Date.now(),
      },
    );
    return shapeMixtape(mixRow({ slug }));
  },

  'PATCH /api/mixtapes/:slug': (ctx) => {
    if (!ctx.user) return ctx.fail(401, 'sign in first');
    const row = one('SELECT * FROM mixtapes WHERE slug = :slug', { slug: ctx.params.slug });
    if (!row) return ctx.fail(404, 'no such mixtape');
    if (row.author_id !== ctx.user.id && ctx.user.role !== 'moderator') {
      return ctx.fail(403, 'that mixtape is not yours');
    }
    const b = ctx.body ?? {};
    if (b.status && !['draft', 'review'].includes(b.status)) {
      return ctx.fail(400, 'you can only move a mixtape between draft and review');
    }
    if (b.status === 'review') {
      const n = all('SELECT COUNT(*) AS n FROM mixtape_sounds WHERE mixtape_id = :id', {
        id: row.id,
      })[0].n;
      if (n < 2) return ctx.fail(400, 'a mixtape needs at least two soundscapes before review');
      const unpublished = all(
        `SELECT COUNT(*) AS n FROM mixtape_sounds ms JOIN sounds s ON s.id = ms.sound_id
         WHERE ms.mixtape_id = :id AND s.status != 'public'`,
        { id: row.id },
      )[0].n;
      if (unpublished) {
        return ctx.fail(400, 'every soundscape in the mixtape has to be published first');
      }
    }
    run(
      `UPDATE mixtapes SET title = :title, description = :desc, accent = :accent, status = :status
       WHERE id = :id`,
      {
        title: b.title !== undefined ? String(b.title) : row.title,
        desc: b.description !== undefined ? String(b.description) : row.description,
        accent: b.accent !== undefined ? String(b.accent) : row.accent,
        status: b.status ?? row.status,
        id: row.id,
      },
    );
    return shapeMixtape(mixRow({ id: row.id }));
  },

  'DELETE /api/mixtapes/:slug': (ctx) => {
    if (!ctx.user) return ctx.fail(401, 'sign in first');
    const row = one('SELECT * FROM mixtapes WHERE slug = :slug', { slug: ctx.params.slug });
    if (!row) return ctx.fail(404, 'no such mixtape');
    if (row.author_id !== ctx.user.id && ctx.user.role !== 'moderator') {
      return ctx.fail(403, 'that mixtape is not yours');
    }
    run('DELETE FROM mixtape_sounds WHERE mixtape_id = :id', { id: row.id });
    run('DELETE FROM mixtapes WHERE id = :id', { id: row.id });
    return { ok: true, deleted: row.slug };
  },

  'POST /api/mixtapes/:slug/sounds': (ctx) => {
    if (!ctx.user) return ctx.fail(401, 'sign in first');
    const m = one('SELECT * FROM mixtapes WHERE slug = :slug', { slug: ctx.params.slug });
    if (!m) return ctx.fail(404, 'no such mixtape');
    if (m.author_id !== ctx.user.id && ctx.user.role !== 'moderator') {
      return ctx.fail(403, 'that mixtape is not yours');
    }
    const sound = one('SELECT * FROM sounds WHERE slug = :slug', {
      slug: String(ctx.body?.slug ?? ''),
    });
    if (!sound) return ctx.fail(404, 'no such sound');
    if (sound.uploader_id !== m.author_id) {
      return ctx.fail(400, 'a mixtape can only hold soundscapes by the same author');
    }
    if (ctx.body?.remove) {
      run('DELETE FROM mixtape_sounds WHERE mixtape_id = :m AND sound_id = :s', {
        m: m.id,
        s: sound.id,
      });
    } else {
      const pos = all('SELECT COUNT(*) AS n FROM mixtape_sounds WHERE mixtape_id = :m', {
        m: m.id,
      })[0].n;
      if (pos >= LIMITS.soundsPerMixtape) {
        return ctx.fail(400, `a mixtape holds at most ${LIMITS.soundsPerMixtape} soundscapes`);
      }
      run(
        'INSERT OR IGNORE INTO mixtape_sounds (mixtape_id, sound_id, position) VALUES (:m, :s, :p)',
        { m: m.id, s: sound.id, p: pos },
      );
    }
    return shapeMixtape(mixRow({ id: m.id }));
  },

  'GET /api/stats': () => ({
    users: one('SELECT COUNT(*) AS n FROM users').n,
    tags: one('SELECT COUNT(*) AS n FROM tags').n,
    byStatus: Object.fromEntries(
      all('SELECT status, COUNT(*) AS n FROM sounds GROUP BY status').map((r) => [r.status, r.n]),
    ),
  }),
};

export function audioFile(slug) {
  const row = one('SELECT audio_path, status FROM sounds WHERE slug = :slug', { slug });
  if (!row?.audio_path) return null;
  const path = join(UPLOADS, row.audio_path);
  if (!existsSync(path)) return null;
  return { path, status: row.status, read: () => readFileSync(path) };
}

export function coverImage(slug) {
  const row = one('SELECT cover_path FROM sounds WHERE slug = :slug', { slug });
  if (!row?.cover_path) return null;
  const path = join(UPLOADS, row.cover_path);
  if (!existsSync(path)) return null;
  return { path, read: () => readFileSync(path) };
}

export { userForToken, cookieToken, withUploader };
