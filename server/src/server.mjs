import { createServer } from 'node:http';
import { createReadStream, existsSync, statSync } from 'node:fs';
import { extname, join, normalize } from 'node:path';
import { open, ROOT, run } from './db.mjs';
import { routes, audioFile, coverImage } from './api.mjs';
import { cookieToken, userForToken } from './auth.mjs';
import { haveFfmpeg } from './media.mjs';
import { clientIp, sweep, take } from './rate.mjs';
import {
  HOST,
  IS_HTTPS,
  IS_PUBLIC,
  PORT,
  PUBLIC_URL,
  describe,
  originAllowed,
} from './config.mjs';

const PUBLIC = join(ROOT, 'public');
const WRITE_METHODS = ['POST', 'PUT', 'PATCH', 'DELETE'];

const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.svg': 'image/svg+xml',
  '.flac': 'audio/flac',
  '.ttf': 'font/ttf',
  '.woff': 'font/woff',
  '.woff2': 'font/woff2',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.ico': 'image/x-icon',
  '.txt': 'text/plain; charset=utf-8',
};

const IMMUTABLE = ['.ttf', '.woff', '.woff2'];

const CSP = [
  "default-src 'self'",
  "img-src 'self' data: blob:",
  "media-src 'self' blob:",
  "style-src 'self' 'unsafe-inline'",
  "script-src 'self'",
  "font-src 'self'",
  "connect-src 'self'",
  "form-action 'self'",
  "frame-ancestors 'none'",
  "base-uri 'self'",
  "object-src 'none'",
].join('; ');

function securityHeaders() {
  const h = {
    'Content-Security-Policy': CSP,
    'X-Content-Type-Options': 'nosniff',
    'Referrer-Policy': 'same-origin',
    'X-Frame-Options': 'DENY',
    'Cross-Origin-Resource-Policy': 'same-site',
    'Permissions-Policy': 'camera=(), microphone=(), geolocation=()',
  };
  if (IS_HTTPS) h['Strict-Transport-Security'] = 'max-age=31536000; includeSubDomains';
  return h;
}

function rateRule(method, path) {
  if (path === '/api/login') return 'login';
  if (path === '/api/register') return 'register';
  if (path === '/api/reports' || path === '/api/tags/request') return 'report';
  if (path.endsWith('/audio') || path.endsWith('/cover')) return 'upload';
  if (WRITE_METHODS.includes(method)) return 'write';
  return null;
}

function match(method, path) {
  for (const key of Object.keys(routes)) {
    const [m, pattern] = key.split(' ');
    if (m !== method) continue;
    const a = pattern.split('/');
    const b = path.split('/');
    if (a.length !== b.length) continue;
    const params = {};
    let hit = true;
    for (let i = 0; i < a.length; i++) {
      if (a[i].startsWith(':')) params[a[i].slice(1)] = decodeURIComponent(b[i]);
      else if (a[i] !== b[i]) { hit = false; break; }
    }
    if (hit) return { handler: routes[key], params };
  }
  return null;
}

function readBody(req, limit = 96 * 1024 * 1024) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    let size = 0;
    req.on('data', (c) => {
      size += c.length;
      if (size > limit) {
        reject(new Error('body too large'));
        req.destroy();
        return;
      }
      chunks.push(c);
    });
    req.on('end', () => resolve(Buffer.concat(chunks)));
    req.on('error', reject);
  });
}

function serveStatic(res, path, head) {
  const rel = path === '/' ? '/index.html' : path;
  const file = normalize(join(PUBLIC, decodeURIComponent(rel)));
  if (!file.startsWith(PUBLIC) || !existsSync(file) || !statSync(file).isFile()) return false;
  const ext = extname(file);
  res.writeHead(200, {
    ...securityHeaders(),
    'Content-Type': MIME[ext] ?? 'application/octet-stream',
    'Content-Length': statSync(file).size,
    'Cache-Control': IMMUTABLE.includes(ext) ? 'public, max-age=604800' : 'no-cache',
  });
  if (head) {
    res.end();
    return true;
  }
  createReadStream(file).pipe(res);
  return true;
}

open();
run('DELETE FROM sessions WHERE expires_at < :now', { now: Date.now() });

const ffmpeg = haveFfmpeg();

const server = createServer(async (req, res) => {
  const method = req.method ?? 'GET';
  const url = new URL(req.url ?? '/', PUBLIC_URL || `http://${HOST}:${PORT}`);
  const path = url.pathname;

  const send = (code, payload, headers = {}) => {
    const body = JSON.stringify(payload, null, 2);
    res.writeHead(code, {
      ...securityHeaders(),
      'Content-Type': MIME['.json'],
      'Content-Length': Buffer.byteLength(body),
      ...headers,
    });
    res.end(method === 'HEAD' ? undefined : body);
  };

  if (path === '/healthz') {
    return send(200, { ok: true, ffmpeg, uptime: Math.round(process.uptime()) });
  }

  if (path.startsWith('/audio/')) {
    const found = audioFile(decodeURIComponent(path.slice(7).replace(/\.flac$/, '')));
    if (!found) return send(404, { error: 'no audio for that sound' });
    const total = statSync(found.path).size;
    const range = req.headers.range;
    const base = { ...securityHeaders(), 'Content-Type': MIME['.flac'], 'Accept-Ranges': 'bytes' };

    if (range) {
      const m = /bytes=(\d*)-(\d*)/.exec(range);
      let start = m && m[1] ? Number(m[1]) : 0;
      let end = m && m[2] ? Number(m[2]) : total - 1;
      if (Number.isNaN(start) || Number.isNaN(end) || start > end || start >= total) {
        res.writeHead(416, { ...base, 'Content-Range': `bytes */${total}` });
        res.end();
        return;
      }
      end = Math.min(end, total - 1);
      res.writeHead(206, {
        ...base,
        'Content-Length': end - start + 1,
        'Content-Range': `bytes ${start}-${end}/${total}`,
        'Cache-Control': 'public, max-age=3600',
      });
      if (method === 'HEAD') return res.end();
      createReadStream(found.path, { start, end }).pipe(res);
      return;
    }

    res.writeHead(200, { ...base, 'Content-Length': total, 'Cache-Control': 'public, max-age=3600' });
    if (method === 'HEAD') return res.end();
    createReadStream(found.path).pipe(res);
    return;
  }

  if (path.startsWith('/cover/')) {
    const found = coverImage(decodeURIComponent(path.slice(7).replace(/\.jpg$/, '')));
    if (!found) return send(404, { error: 'no cover for that sound' });
    const bytes = found.read();
    res.writeHead(200, {
      ...securityHeaders(),
      'Content-Type': 'image/jpeg',
      'Content-Length': bytes.length,
      'Cache-Control': 'public, max-age=3600',
    });
    res.end(method === 'HEAD' ? undefined : bytes);
    return;
  }

  if (!path.startsWith('/api/')) {
    if ((method === 'GET' || method === 'HEAD') && serveStatic(res, path, method === 'HEAD')) return;
    res.writeHead(404, { ...securityHeaders(), 'Content-Type': 'text/plain; charset=utf-8' });
    res.end('not found');
    return;
  }

  if (WRITE_METHODS.includes(method)) {
    const origin = req.headers.origin ?? null;
    if (origin && !originAllowed(origin)) {
      return send(403, { error: 'that origin is not accepted by this workshop' });
    }
    if (IS_PUBLIC && !origin && req.headers['sec-fetch-site'] === 'cross-site') {
      return send(403, { error: 'cross site writes are not accepted' });
    }
  }

  const rule = rateRule(method, path);
  if (rule) {
    const allowed = take(rule, clientIp(req));
    if (!allowed.ok) {
      return send(429, { error: 'too many requests, wait a little' }, {
        'Retry-After': String(allowed.retryAfter),
      });
    }
  }

  const hit = match(method, path);
  if (!hit) return send(404, { error: 'no such endpoint' });

  let raw = Buffer.alloc(0);
  if (WRITE_METHODS.includes(method)) {
    try {
      raw = await readBody(req);
    } catch {
      return send(413, { error: 'body too large' });
    }
  }

  const type = req.headers['content-type'] ?? '';
  let body = null;
  if (raw.length && type.includes('application/json')) {
    try {
      body = JSON.parse(raw.toString('utf8'));
    } catch {
      return send(400, { error: 'body is not valid json' });
    }
  }

  let cookie = null;
  const ctx = {
    req,
    raw,
    body,
    ffmpeg,
    ip: clientIp(req),
    query: url.searchParams,
    params: hit.params,
    user: userForToken(cookieToken(req)),
    setCookie: (v) => {
      cookie = v;
    },
    fail: (code, error) => ({ __status: code, error }),
  };

  try {
    const out = await hit.handler(ctx);
    const headers = cookie ? { 'Set-Cookie': cookie } : {};
    if (out && typeof out === 'object' && out.__status) {
      const { __status, ...rest } = out;
      return send(__status, rest, headers);
    }
    return send(200, out ?? { ok: true }, headers);
  } catch (e) {
    console.error('[api]', path, e);
    return send(500, { error: String(e?.message ?? e) });
  }
});

const housekeeping = setInterval(() => {
  sweep();
  run('DELETE FROM sessions WHERE expires_at < :now', { now: Date.now() });
}, 60 * 60 * 1000);
housekeeping.unref();

server.listen(PORT, HOST, () => {
  console.log('nuru workshop');
  for (const line of describe()) console.log(`  ${line}`);
  console.log(`  ffmpeg ${ffmpeg ? 'found' : 'MISSING, uploads will be refused'}`);
});

for (const signal of ['SIGINT', 'SIGTERM']) {
  process.on(signal, () => {
    console.log(`${signal}, closing`);
    server.close(() => process.exit(0));
    setTimeout(() => process.exit(0), 5000).unref();
  });
}
