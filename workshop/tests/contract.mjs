import { spawn, execFileSync } from 'node:child_process';
import crypto from 'node:crypto';
import fs from 'node:fs';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const WORKSHOP = path.resolve(HERE, '..');
const REPO = path.resolve(WORKSHOP, '..');
const REAL_FLAC = path.join(REPO, 'resources', 'packs', 'field', 'audio', 'station-rain.flac');
const REAL_JPEG = path.join(REPO, 'resources', 'packs', 'field', 'covers', 'station-rain.jpg');

const phpBinary = execFileSync('php', ['-r', 'echo PHP_BINARY;'], { encoding: 'utf8' }).trim();
const PHP_ARGS = ['-d', 'opcache.enable=0', '-d', `extension_dir=${path.join(path.dirname(phpBinary), 'ext')}`,
  ...['sodium', 'pdo_sqlite', 'mbstring'].flatMap((e) => ['-d', `extension=${e}`])];

const CREATOR = 'a-cccccccccccccccc';
const OTHER = 'a-oooooooooooooooo';
const MOD = 'a-mmmmmmmmmmmmmmmm';
const TROLL = 'a-tttttttttttttttt';

const { publicKey, privateKey } = crypto.generateKeyPairSync('ed25519');
const rawPublic = Buffer.from(publicKey.export({ format: 'jwk' }).x, 'base64url');
const b64url = (b) => Buffer.from(b).toString('base64url');

function ticket(sub, opts = {}) {
  const iat = opts.iat ?? Math.floor(Date.now() / 1000);
  const payload = JSON.stringify({ v: 1, sub, name: opts.name ?? `Name ${sub.slice(2, 4)}`, app: opts.app ?? 'nuru', key: null, dev: 'test-device', iat, exp: opts.exp ?? iat + 604800 });
  const sig = crypto.sign(null, Buffer.from(payload), opts.key ?? privateKey);
  return `${b64url(payload)}.${b64url(sig)}`;
}

function flac({ rate = 48000, channels = 2, bits = 16, seconds = 10, extra = 4096 } = {}) {
  const head = Buffer.alloc(42);
  head.write('fLaC', 0, 'latin1');
  head[4] = 0x80;
  head.writeUIntBE(34, 5, 3);
  head.writeUInt16BE(4096, 8);
  head.writeUInt16BE(4096, 10);
  const samples = BigInt(Math.round(rate * seconds));
  const packed = (BigInt(rate) << 44n) | (BigInt(channels - 1) << 41n) | (BigInt(bits - 1) << 36n) | samples;
  head.writeBigUInt64BE(packed, 18);
  return Buffer.concat([head, crypto.randomBytes(extra)]);
}

function jpeg(width, height) {
  const app0 = Buffer.from([0xff, 0xe0, 0x00, 0x10, 0x4a, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00]);
  const sof = Buffer.from([0xff, 0xc0, 0x00, 0x11, 0x08, height >> 8, height & 255, width >> 8, width & 255, 0x03, 0x01, 0x22, 0x00, 0x02, 0x11, 0x01, 0x03, 0x11, 0x01]);
  return Buffer.concat([Buffer.from([0xff, 0xd8]), app0, sof, crypto.randomBytes(200), Buffer.from([0xff, 0xd9])]);
}

const freePort = () => new Promise((resolve) => {
  const s = net.createServer();
  s.listen(0, '127.0.0.1', () => {
    const { port } = s.address();
    s.close(() => resolve(port));
  });
});

const dataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'nuru-ws-'));
const slash = (p) => p.split(path.sep).join('/');
fs.writeFileSync(path.join(dataDir, 'config.php'), `<?php
return [
  'db' => ['dsn' => 'sqlite:${slash(path.join(dataDir, 'ws.sqlite'))}'],
  'ticket_public_key' => '${rawPublic.toString('base64')}',
  'secret' => '${crypto.randomBytes(16).toString('hex')}',
  'moderators' => ['${MOD}'],
  'audio_bytes' => ${8 << 20},
  'cover_bytes' => ${1 << 20},
  'limits' => ['report' => [3, 86400]],
];
`);

const port = await freePort();
const base = `http://127.0.0.1:${port}`;
const php = spawn(phpBinary, [...PHP_ARGS, '-S', `127.0.0.1:${port}`, '-t', path.join(WORKSHOP, 'public'), path.join(WORKSHOP, 'router.php')], {
  env: { ...process.env, NURU_WS_DATA: dataDir },
  stdio: ['ignore', 'ignore', 'pipe'],
});
let phpLog = '';
php.stderr.on('data', (d) => { phpLog += d; });

async function req(method, url, { who, body, raw, headers = {} } = {}) {
  const h = { ...headers };
  if (who) h['X-Hexagon-Ticket'] = typeof who === 'string' && who.includes('.') ? who : ticket(who);
  let payload;
  if (raw !== undefined) payload = raw;
  else if (body !== undefined) {
    payload = JSON.stringify(body);
    h['Content-Type'] = 'application/json';
  }
  const res = await fetch(base + url, { method, headers: h, body: payload });
  const buf = Buffer.from(await res.arrayBuffer());
  let json = null;
  try {
    json = JSON.parse(buf.toString('utf8'));
  } catch {}
  return { status: res.status, headers: res.headers, json, buf };
}

let failed = 0;
let passed = 0;
function check(name, ok, detail = '') {
  if (ok) passed++;
  else {
    failed++;
    console.log(`FAIL ${name} ${typeof detail === 'string' ? detail : JSON.stringify(detail).slice(0, 400)}`);
  }
}
const show = (r) => `${r.status} ${JSON.stringify(r.json ?? r.buf.toString('utf8').slice(0, 200)).slice(0, 400)}`;
const err = (r, status, code) => r.status === status && r.json?.error === code;
const sha = (b) => crypto.createHash('sha256').update(b).digest('hex');

for (let i = 0; i < 50; i++) {
  try {
    const r = await fetch(base + '/api/health');
    if (r.ok) break;
  } catch {}
  await new Promise((r) => setTimeout(r, 100));
}

try {
  const health = await req('GET', '/api/health');
  check('health', health.status === 200 && health.json?.ok === true, show(health));
  const options = await req('GET', '/api/options');
  check('options', options.json?.tags?.length === 10 && options.json?.licences?.includes('CC0-1.0'), show(options));

  check('me signed out', err(await req('GET', '/api/me'), 401, 'signed_out'));
  const other = crypto.generateKeyPairSync('ed25519').privateKey;
  check('me with a foreign key', err(await req('GET', '/api/me', { who: ticket(CREATOR, { key: other }) }), 401, 'invalid_ticket'));
  check('me expired', err(await req('GET', '/api/me', { who: ticket(CREATOR, { exp: Math.floor(Date.now() / 1000) - 5 }) }), 401, 'invalid_ticket'));
  check('me other app', err(await req('GET', '/api/me', { who: ticket(CREATOR, { app: 'outpost' }) }), 401, 'invalid_ticket'));
  const good = ticket(CREATOR);
  const [p, s] = good.split('.');
  const tampered = b64url(Buffer.from(Buffer.from(p, 'base64url').toString().replace(CREATOR, MOD))) + '.' + s;
  check('me tampered', err(await req('GET', '/api/me', { who: tampered }), 401, 'invalid_ticket'));
  check('me bad id', err(await req('GET', '/api/me', { who: ticket('a-UPPERCASE000000') }), 401, 'invalid_ticket'));
  const me = await req('GET', '/api/me', { who: CREATOR });
  check('me', me.status === 200 && me.json.account.id === CREATOR && me.json.moderator === false, show(me));
  const meMod = await req('GET', '/api/me', { who: MOD });
  check('me moderator', meMod.json?.moderator === true, show(meMod));

  check('create signed out', err(await req('POST', '/api/items', { body: { kind: 'sound', meta: { title: 'X y' } } }), 401, 'signed_out'));
  check('create bad kind', (await req('POST', '/api/items', { who: CREATOR, body: { kind: 'mix', meta: { title: 'Rain' } } })).json?.field === 'kind');
  check('create no title', (await req('POST', '/api/items', { who: CREATOR, body: { kind: 'sound', meta: {} } })).json?.field === 'title');
  check('create bad json', err(await req('POST', '/api/items', { who: CREATOR, raw: '{nope', headers: { 'Content-Type': 'application/json' } }), 400, 'bad_json'));
  const created = await req('POST', '/api/items', { who: CREATOR, body: { kind: 'sound', meta: { title: 'Rainy Window' } } });
  check('create', created.status === 201 && created.json.item.slug === 'rainy-window' && created.json.item.status === 'new' && created.json.item.versions?.[0]?.status === 'draft', show(created));
  const soundId = created.json?.item?.id;

  check('draft hidden from anyone', err(await req('GET', '/api/items/rainy-window'), 404, 'not_found'));
  check('draft hidden from others', err(await req('GET', '/api/items/rainy-window', { who: OTHER }), 404, 'not_found'));
  check('draft seen by moderator', (await req('GET', '/api/items/rainy-window', { who: MOD })).status === 200);
  check('draft edit by others', err(await req('PATCH', '/api/items/rainy-window/versions/1', { who: OTHER, body: { meta: { title: 'Mine now' } } }), 404, 'not_found'));

  check('bad tag', (await req('PATCH', '/api/items/rainy-window/versions/1', { who: CREATOR, body: { meta: { tags: ['weather', 'lava'] } } })).json?.field === 'tags');
  check('bad cozy', (await req('PATCH', '/api/items/rainy-window/versions/1', { who: CREATOR, body: { meta: { cozy: { channel: 'weather', state: 'lava', weight: 0.5 } } } })).json?.field === 'cozy');
  check('bad accent', (await req('PATCH', '/api/items/rainy-window/versions/1', { who: CREATOR, body: { meta: { accent: 'blue' } } })).json?.field === 'accent');
  check('bad loop', (await req('PATCH', '/api/items/rainy-window/versions/1', { who: CREATOR, body: { meta: { loop: { start: 10, end: 5, crossfade_ms: 0 } } } })).json?.field === 'loop');
  const meta = { summary: 'Rain on a station roof.\n\nRecorded in Taiwan.', tags: ['weather', 'water'], licence: 'CC-BY-4.0', accent: '#8fb8ff', cozy: { channel: 'weather', state: 'rain', weight: 0.9 }, loop: { start: 0, end: null, crossfade_ms: 1200 }, lufs: -23 };
  const edited = await req('PATCH', '/api/items/rainy-window/versions/1', { who: CREATOR, body: { meta, note: 'First upload' } });
  check('edit draft', edited.status === 200 && edited.json.version.meta.accent === '#8FB8FF' && edited.json.version.meta.title === 'Rainy Window', show(edited));

  check('submit without files', err(await req('POST', '/api/items/rainy-window/versions/1/submit', { who: CREATOR }), 409, 'incomplete'));
  const up = (name, raw, who = CREATOR, slug = 'rainy-window', n = 1) => req('PUT', `/api/items/${slug}/versions/${n}/${name}`, { who, raw });
  check('upload garbage audio', err(await up('audio.flac', crypto.randomBytes(5000)), 400, 'bad_audio'));
  check('upload short audio', err(await up('audio.flac', flac({ seconds: 2 })), 400, 'bad_audio'));
  check('upload 32 kHz', err(await up('audio.flac', flac({ rate: 32000 })), 400, 'bad_audio'));
  check('upload 6 channels', err(await up('audio.flac', flac({ channels: 6 })), 400, 'bad_audio'));
  check('upload too big', err(await up('audio.flac', flac({ extra: 9 << 20 })), 413, 'too_big'));
  check('upload by others', err(await up('audio.flac', flac(), OTHER), 404, 'not_found'));
  const realAudio = fs.readFileSync(REAL_FLAC);
  const audio = await up('audio.flac', realAudio);
  check('upload real FLAC', audio.status === 200 && audio.json.file.rate === 48000 && audio.json.file.samples === 2160000 && audio.json.file.sha256 === sha(realAudio), show(audio));
  check('upload garbage cover', err(await up('cover.jpg', crypto.randomBytes(3000)), 400, 'bad_cover'));
  check('upload wide cover', err(await up('cover.jpg', jpeg(640, 320)), 400, 'bad_cover'));
  check('upload tiny cover', err(await up('cover.jpg', jpeg(64, 64)), 400, 'bad_cover'));
  const realCover = fs.readFileSync(REAL_JPEG);
  const cover = await up('cover.jpg', realCover);
  check('upload real cover', cover.status === 200 && cover.json.file.width === 320 && cover.json.file.height === 320, show(cover));

  const needsCredit = await req('POST', '/api/items/rainy-window/versions/1/submit', { who: CREATOR });
  check('CC-BY needs a credit', err(needsCredit, 409, 'incomplete') && needsCredit.json.field === 'credit', show(needsCredit));
  await req('PATCH', '/api/items/rainy-window/versions/1', { who: CREATOR, body: { meta: { credit: 'Wu Tsan-Cheng, radio aporee' } } });
  const submitted = await req('POST', '/api/items/rainy-window/versions/1/submit', { who: CREATOR });
  check('submit', submitted.status === 200 && submitted.json.version.status === 'review', show(submitted));
  check('edit after submit', err(await req('PATCH', '/api/items/rainy-window/versions/1', { who: CREATOR, body: { meta: { title: 'Changed' } } }), 409, 'not_draft'));
  check('upload after submit', err(await up('cover.jpg', realCover), 409, 'not_draft'));
  check('withdraw', (await req('POST', '/api/items/rainy-window/versions/1/withdraw', { who: CREATOR })).json?.version?.status === 'draft');
  check('submit again', (await req('POST', '/api/items/rainy-window/versions/1/submit', { who: CREATOR })).json?.version?.status === 'review');

  const fileUrl = `/f/${soundId}/1/audio.flac`;
  check('review file hidden from anyone', (await req('GET', fileUrl)).status === 404);
  check('review file hidden from others', (await req('GET', fileUrl, { who: OTHER })).status === 404);
  const ownFile = await req('GET', fileUrl, { who: CREATOR });
  check('review file for the creator', ownFile.status === 200 && sha(ownFile.buf) === sha(realAudio) && ownFile.headers.get('cache-control').includes('no-store'), `${ownFile.status}`);
  check('review file for a moderator', (await req('GET', fileUrl, { who: MOD })).status === 200);

  check('queue for creators', err(await req('GET', '/api/mod/queue', { who: CREATOR }), 403, 'forbidden'));
  const queue = await req('GET', '/api/mod/queue', { who: MOD });
  check('queue', queue.json?.queue?.length === 1 && queue.json.queue[0].version.files['audio.flac'].url === fileUrl, show(queue));
  const versionId = queue.json?.queue?.[0]?.version?.id;
  const rev = queue.json?.queue?.[0]?.version?.rev;
  check('decide needs the revision', (await req('POST', `/api/mod/versions/${versionId}`, { who: MOD, body: { decision: 'accept' } })).json?.field === 'rev');
  check('decline needs a note', (await req('POST', `/api/mod/versions/${versionId}`, { who: MOD, body: { decision: 'decline', rev } })).json?.field === 'note');
  check('decide as creator', err(await req('POST', `/api/mod/versions/${versionId}`, { who: CREATOR, body: { decision: 'accept', rev } }), 403, 'forbidden'));
  check('stale revision', err(await req('POST', `/api/mod/versions/${versionId}`, { who: MOD, body: { decision: 'accept', rev: rev - 1 } }), 409, 'changed'));
  const accepted = await req('POST', `/api/mod/versions/${versionId}`, { who: MOD, body: { decision: 'accept', note: 'Lovely', rev } });
  check('accept', accepted.status === 200 && accepted.json.item.status === 'public' && accepted.json.item.version === 1 && accepted.json.item.published > 0, show(accepted));
  check('accept twice', err(await req('POST', `/api/mod/versions/${versionId}`, { who: MOD, body: { decision: 'accept', rev } }), 409, 'not_review'));

  const list = await req('GET', '/api/items');
  check('browse', list.json?.total === 1 && list.json.items[0].slug === 'rainy-window' && list.json.items[0].cover === `/f/${soundId}/1/cover.jpg` && list.json.items[0].owner.name === 'Name cc', show(list));
  check('browse by tag', (await req('GET', '/api/items?tag=weather')).json?.total === 1 && (await req('GET', '/api/items?tag=fire')).json?.total === 0);
  check('browse by words', (await req('GET', '/api/items?q=RAINY%20window')).json?.total === 1 && (await req('GET', '/api/items?q=zzz')).json?.total === 0);
  check('browse wildcards are literal', (await req('GET', '/api/items?q=%25')).json?.total === 0 && (await req('GET', '/api/items?q=_')).json?.total === 0);
  check('browse by kind', (await req('GET', '/api/items?kind=pack')).json?.total === 0);
  check('browse by owner', (await req('GET', `/api/items?owner=${CREATOR}`)).json?.total === 1 && (await req('GET', `/api/items?owner=${OTHER}`)).json?.total === 0);
  const tags = await req('GET', '/api/tags');
  check('tags', tags.json?.tags?.find((t) => t.tag === 'weather')?.count === 1, show(tags));
  const page = await req('GET', '/api/items/rainy-window');
  check('item page', page.status === 200 && page.json.item.meta.credit === 'Wu Tsan-Cheng, radio aporee' && !page.json.item.versions && !page.json.item.me, show(page));

  const pub = await req('GET', fileUrl);
  check('public file', pub.status === 200 && sha(pub.buf) === sha(realAudio) && pub.headers.get('cache-control') === 'public, no-cache' && pub.headers.get('content-type') === 'audio/flac', `${pub.status}`);
  const part = await req('GET', fileUrl, { headers: { Range: 'bytes=0-99' } });
  check('range start', part.status === 206 && part.buf.length === 100 && part.headers.get('content-range') === `bytes 0-99/${realAudio.length}`, `${part.status} ${part.headers.get('content-range')}`);
  const tail = await req('GET', fileUrl, { headers: { Range: 'bytes=-10' } });
  check('range tail', tail.status === 206 && tail.buf.equals(realAudio.subarray(realAudio.length - 10)), `${tail.status}`);
  check('range past the end', (await req('GET', fileUrl, { headers: { Range: `bytes=${realAudio.length + 5}-` } })).status === 416);
  check('range junk', (await req('GET', fileUrl, { headers: { Range: 'bytes=5-2' } })).status === 416);
  check('etag', (await req('GET', fileUrl, { headers: { 'If-None-Match': `"${sha(realAudio)}"` } })).status === 304);
  const head = await req('HEAD', fileUrl);
  check('head', head.status === 200 && head.buf.length === 0 && Number(head.headers.get('content-length')) === realAudio.length, `${head.status}`);

  check('subscribe signed out', err(await req('PUT', '/api/subs/rainy-window'), 401, 'signed_out'));
  const sub = await req('PUT', '/api/subs/rainy-window', { who: OTHER });
  check('subscribe', sub.json?.subs === 1, show(sub));
  check('subscribe twice', (await req('PUT', '/api/subs/rainy-window', { who: OTHER })).json?.subs === 1);
  const subs = await req('GET', '/api/me/subs', { who: OTHER });
  check('my subscriptions', subs.json?.items?.length === 1 && subs.json.items[0].files['audio.flac'].sha256 === sha(realAudio), show(subs));
  check('unsubscribe', (await req('DELETE', '/api/subs/rainy-window', { who: OTHER })).json?.subscribed === false && (await req('GET', '/api/me/subs', { who: OTHER })).json.items.length === 0);
  await req('PUT', '/api/subs/rainy-window', { who: OTHER });

  check('vote own', err(await req('PUT', '/api/items/rainy-window/vote', { who: CREATOR, body: { value: 1 } }), 409, 'own_item'));
  check('vote', (await req('PUT', '/api/items/rainy-window/vote', { who: OTHER, body: { value: 1 } })).json?.up === 1);
  check('vote bad', (await req('PUT', '/api/items/rainy-window/vote', { who: OTHER, body: { value: 2 } })).json?.field === 'value');
  check('vote again', (await req('PUT', '/api/items/rainy-window/vote', { who: OTHER, body: { value: -1 } })).json?.down === 1);
  const [vote1, vote2] = await Promise.all([1, 2].map(() => req('PUT', '/api/items/rainy-window/vote', { who: OTHER, body: { value: -1 } })));
  check('double vote', vote1.status === 200 && vote2.status === 200, `${show(vote1)} ${show(vote2)}`);
  const mine = await req('GET', '/api/items/rainy-window', { who: OTHER });
  check('my vote and subscription', mine.json?.item?.me?.vote === -1 && mine.json.item.me.subscribed === true, show(mine));

  check('report bad reason', (await req('POST', '/api/items/rainy-window/report', { who: OTHER, body: { reason: 'meh' } })).json?.field === 'reason');
  check('report', (await req('POST', '/api/items/rainy-window/report', { who: OTHER, body: { reason: 'licence', text: 'Is this really CC-BY?' } })).json?.ok === true);
  check('report again updates', (await req('POST', '/api/items/rainy-window/report', { who: OTHER, body: { reason: 'spam' } })).json?.ok === true);
  const reports = await req('GET', '/api/mod/reports', { who: MOD });
  check('reports', reports.json?.reports?.length === 1 && reports.json.reports[0].reason === 'spam', show(reports));
  check('close report', (await req('POST', `/api/mod/reports/${reports.json?.reports?.[0]?.id}`, { who: MOD })).json?.ok === true && (await req('GET', '/api/mod/reports', { who: MOD })).json.reports.length === 0);
  await req('POST', '/api/items/rainy-window/report', { who: OTHER, body: { reason: 'other' } });
  check('report limit', err(await req('POST', '/api/items/rainy-window/report', { who: OTHER, body: { reason: 'other' } }), 429, 'too_many'));

  const v2 = await req('POST', '/api/items/rainy-window/versions', { who: CREATOR, body: { note: 'Louder', meta: { summary: 'Louder rain.' } } });
  check('new version', v2.status === 201 && v2.json.version.number === 2 && v2.json.version.meta.title === 'Rainy Window' && v2.json.version.meta.summary === 'Louder rain.', show(v2));
  check('one open version', err(await req('POST', '/api/items/rainy-window/versions', { who: CREATOR, body: {} }), 409, 'open_version'));
  await up('audio.flac', realAudio, CREATOR, 'rainy-window', 2);
  await up('cover.jpg', realCover, CREATOR, 'rainy-window', 2);
  check('submit v2', (await req('POST', '/api/items/rainy-window/versions/2/submit', { who: CREATOR })).json?.version?.status === 'review');
  check('v1 still public during review', (await req('GET', '/api/items/rainy-window')).json?.item?.version === 1);
  const q2 = await req('GET', '/api/mod/queue', { who: MOD });
  await req('POST', `/api/mod/versions/${q2.json.queue[0].version.id}`, { who: MOD, body: { decision: 'accept', rev: q2.json.queue[0].version.rev } });
  const after = await req('GET', '/api/items/rainy-window', { who: CREATOR });
  check('v2 replaces v1', after.json?.item?.version === 2 && after.json.item.summary === 'Louder rain.' && after.json.item.versions.find((v) => v.number === 1)?.status === 'replaced', show(after));
  check('v1 files gone', (await req('GET', fileUrl, { who: CREATOR })).status === 404);
  check('v2 file public', (await req('GET', `/f/${soundId}/2/audio.flac`)).status === 200);

  const sound2 = await req('POST', '/api/items', { who: CREATOR, body: { kind: 'sound', meta: { title: 'Rainy Window', tags: ['weather'], licence: 'CC0-1.0', accent: '#123456' } } });
  check('same title gets a new slug', sound2.json?.item?.slug === 'rainy-window-2', show(sound2));
  await up('audio.flac', realAudio, CREATOR, 'rainy-window-2');
  await up('cover.jpg', realCover, CREATOR, 'rainy-window-2');
  check('submit second sound', (await req('POST', '/api/items/rainy-window-2/versions/1/submit', { who: CREATOR })).json?.version?.status === 'review');

  const foreign = await req('POST', '/api/items', { who: OTHER, body: { kind: 'pack', meta: { title: 'Borrowed', tags: ['weather'], licence: 'CC0-1.0', sounds: [soundId] } } });
  await up('cover.jpg', realCover, OTHER, foreign.json.item.slug);
  check('pack of someone else\'s sounds', err(await req('POST', `/api/items/${foreign.json.item.slug}/versions/1/submit`, { who: OTHER }), 409, 'incomplete'));
  check('pack rejects audio', err(await up('audio.flac', realAudio, OTHER, foreign.json.item.slug), 400, 'bad_file'));

  const pack = await req('POST', '/api/items', { who: CREATOR, body: { kind: 'pack', meta: { title: 'Rain Set', tags: ['weather'], licence: 'CC0-1.0', sounds: [soundId, sound2.json.item.id] } } });
  await up('cover.jpg', realCover, CREATOR, 'rain-set');
  check('submit pack', (await req('POST', '/api/items/rain-set/versions/1/submit', { who: CREATOR })).json?.version?.status === 'review', show(pack));
  const q3 = (await req('GET', '/api/mod/queue', { who: MOD })).json.queue;
  const packEntry = q3.find((e) => e.item.slug === 'rain-set');
  check('queue shows pack sounds', packEntry?.sounds?.length === 2, JSON.stringify(packEntry?.sounds?.map((s) => s.status)));
  check('pack waits for its sounds', err(await req('POST', `/api/mod/versions/${packEntry.version.id}`, { who: MOD, body: { decision: 'accept', rev: packEntry.version.rev } }), 409, 'sounds_pending'));
  const second = q3.find((e) => e.item.slug === 'rainy-window-2').version;
  await req('POST', `/api/mod/versions/${second.id}`, { who: MOD, body: { decision: 'accept', rev: second.rev } });
  check('accept pack', (await req('POST', `/api/mod/versions/${packEntry.version.id}`, { who: MOD, body: { decision: 'accept', rev: packEntry.version.rev } })).json?.item?.status === 'public');
  const packPage = await req('GET', '/api/items/rain-set');
  check('pack page lists sounds', packPage.json?.item?.sounds?.length === 2 && packPage.json.item.sounds[0].files['audio.flac'].url.startsWith('/f/'), show(packPage));

  check('hide as creator', err(await req('POST', '/api/mod/items/rainy-window', { who: CREATOR, body: { action: 'hide' } }), 403, 'forbidden'));
  check('hide', (await req('POST', '/api/mod/items/rainy-window', { who: MOD, body: { action: 'hide' } })).json?.item?.status === 'hidden');
  check('hidden page', (await req('GET', '/api/items/rainy-window')).status === 404 && (await req('GET', `/f/${soundId}/2/audio.flac`)).status === 404);
  check('hidden not listed', (await req('GET', '/api/items?kind=sound')).json?.total === 1);
  check('hidden not synced', (await req('GET', '/api/me/subs', { who: OTHER })).json?.items?.length === 0);
  check('hidden pack drops it', (await req('GET', '/api/items/rain-set')).json?.item?.sounds?.length === 1);
  check('unhide', (await req('POST', '/api/mod/items/rainy-window', { who: MOD, body: { action: 'unhide' } })).json?.item?.status === 'public');

  check('ban a moderator', err(await req('POST', `/api/mod/accounts/${MOD}`, { who: MOD, body: { action: 'ban' } }), 409, 'moderator'));
  check('ban', (await req('POST', `/api/mod/accounts/${TROLL}`, { who: MOD, body: { action: 'ban' } })).json?.ok === true);
  check('banned can\'t create', err(await req('POST', '/api/items', { who: TROLL, body: { kind: 'sound', meta: { title: 'Spam' } } }), 403, 'banned'));
  check('banned can read', (await req('GET', '/api/me', { who: TROLL })).json?.banned === true);

  const draft = await req('POST', '/api/items', { who: CREATOR, body: { kind: 'sound', meta: { title: 'Scratch' } } });
  await up('cover.jpg', realCover, CREATOR, 'scratch');
  check('delete draft item', (await req('DELETE', '/api/items/scratch', { who: CREATOR })).json?.ok === true && (await req('GET', '/api/items/scratch', { who: CREATOR })).status === 404, show(draft));
  check('delete as others', err(await req('DELETE', '/api/items/rainy-window', { who: OTHER }), 403, 'forbidden'));
  check('delete public item', (await req('DELETE', '/api/items/rainy-window', { who: CREATOR })).json?.ok === true);
  check('removed is gone', (await req('GET', '/api/items/rainy-window', { who: CREATOR })).status === 404 && (await req('GET', `/f/${soundId}/2/audio.flac`)).status === 404);
  check('removed slug stays taken', (await req('POST', '/api/items', { who: CREATOR, body: { kind: 'sound', meta: { title: 'Rainy Window' } } })).json?.item?.slug === 'rainy-window-3');
  check('my items', (await req('GET', '/api/me/items', { who: CREATOR })).json?.items?.every((i) => i.status !== 'removed'));

  check('pack still public with one sound left', (await req('GET', '/api/items/rain-set')).json?.item?.sounds?.length === 1);
  check('remove the last sound', (await req('POST', '/api/mod/items/rainy-window-2', { who: MOD, body: { action: 'remove' } })).json?.item?.status === 'removed');
  check('empty pack is hidden', (await req('GET', '/api/items/rain-set')).status === 404);
  check('empty pack stays hidden', err(await req('POST', '/api/mod/items/rain-set', { who: MOD, body: { action: 'unhide' } }), 409, 'empty_pack'));

  const swap = await req('POST', '/api/items', { who: CREATOR, body: { kind: 'sound', meta: { title: 'Swap Test', tags: ['noise'], licence: 'CC0-1.0', accent: '#abcdef' } } });
  await up('audio.flac', realAudio, CREATOR, 'swap-test');
  await up('cover.jpg', realCover, CREATOR, 'swap-test');
  await req('POST', '/api/items/swap-test/versions/1/submit', { who: CREATOR });
  const seen = (await req('GET', '/api/mod/queue', { who: MOD })).json.queue.find((e) => e.item.slug === 'swap-test')?.version;
  await req('POST', '/api/items/swap-test/versions/1/withdraw', { who: CREATOR });
  await up('cover.jpg', jpeg(512, 512), CREATOR, 'swap-test');
  await req('POST', '/api/items/swap-test/versions/1/submit', { who: CREATOR });
  check('accept after a swap', err(await req('POST', `/api/mod/versions/${seen?.id}`, { who: MOD, body: { decision: 'accept', rev: seen?.rev } }), 409, 'changed'), show(swap));
  check('replaced draft file is gone', (await req('GET', '/api/mod/queue', { who: MOD })).json.queue.find((e) => e.item.slug === 'swap-test')?.version?.files['cover.jpg']?.width === 512);

  check('unsubscribe unknown', (await req('DELETE', '/api/subs/no-such-thing', { who: OTHER })).json?.subscribed === false);
  for (const q of ['kind=mix', 'tag=lava', 'owner=bob', 'sort=weird', `q=${'x'.repeat(81)}`]) {
    check(`bad filter ${q.slice(0, 12)}`, err(await req('GET', `/api/items?${q}`), 400, 'bad_input'));
  }
  check('invisible title', (await req('POST', '/api/items', { who: CREATOR, body: { kind: 'sound', meta: { title: '\u202e\u200b \u00a0' } } })).json?.field === 'title');
  check('bidi stripped', (await req('POST', '/api/items', { who: CREATOR, body: { kind: 'sound', meta: { title: 'Ab\u202ecd' } } })).json?.item?.title === 'Abcd');

  const wrongMethod = await req('GET', '/api/subs/rain-set');
  check('wrong method', err(wrongMethod, 405, 'bad_method') && wrongMethod.headers.get('allow').includes('PUT'), show(wrongMethod));
  check('unknown path', err(await req('GET', '/api/nope'), 404, 'not_found'));
  check('leading zero version', (await req('PATCH', '/api/items/rain-set/versions/01', { who: CREATOR, body: {} })).status === 404);
  check('lib not served', (await req('GET', '/ws/lib.php')).status === 403 && (await req('GET', '/ws/schema.sqlite.sql')).status === 403);
  check('data not served', (await req('GET', '/ws-data/config.php')).status === 403);
  check('case variants not served', (await req('GET', '/WS/lib.php')).status === 403 && (await req('GET', '/Ws-Data/config.php')).status === 403);
  check('trailing dot not served', (await req('GET', '/ws./lib.php')).status === 403 && (await req('GET', '/ws%20/lib.php')).status === 403);
  check('no traversal', (await req('GET', `/f/${soundId}/2/..%2F..%2Fconfig.php`)).status === 404);
} catch (e) {
  failed++;
  console.log('ERROR', e.stack);
} finally {
  php.kill();
  fs.rmSync(dataDir, { recursive: true, force: true });
}

const errors = phpLog.split('\n').filter((l) => /PHP (Warning|Notice|Deprecated|Fatal|Parse)|workshop: /.test(l));
if (errors.length) failed++;
if (errors.length) console.log('PHP log:\n' + errors.slice(0, 20).join('\n'));
console.log(`${passed} passed, ${failed} failed`);
process.exit(failed ? 1 : 0);
