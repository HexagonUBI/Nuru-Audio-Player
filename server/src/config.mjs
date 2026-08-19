const env = process.env;

function flag(name, fallback) {
  const v = env[name];
  if (v === undefined || v === '') return fallback;
  return v === '1' || v.toLowerCase() === 'true' || v.toLowerCase() === 'yes';
}

function text(name, fallback) {
  const v = env[name];
  return v === undefined || v === '' ? fallback : v;
}

export const PORT = Number(text('NURU_DB_PORT', text('PORT', '5175')));
export const HOST = text('NURU_DB_HOST', '127.0.0.1');

export const PUBLIC_URL = text('NURU_PUBLIC_URL', '').trim().replace(/\/+$/, '');
export const IS_PUBLIC = PUBLIC_URL.length > 0;
export const IS_HTTPS = PUBLIC_URL.startsWith('https://');

export const TRUST_PROXY = flag('NURU_TRUST_PROXY', IS_PUBLIC);
export const OPEN_SIGNUP = flag('NURU_OPEN_SIGNUP', true);

export const FFMPEG = text('NURU_FFMPEG', 'ffmpeg');
export const FFPROBE = text('NURU_FFPROBE', 'ffprobe');

export const MODERATOR_EMAIL = text('NURU_MODERATOR_EMAIL', '').trim().toLowerCase();

export const DATA_DIR = text('NURU_DATA_DIR', '').trim();

export const ORIGINS = (() => {
  const extra = text('NURU_ALLOWED_ORIGINS', '')
    .split(',')
    .map((s) => s.trim().replace(/\/+$/, ''))
    .filter(Boolean);
  const list = new Set(extra);
  if (PUBLIC_URL) list.add(PUBLIC_URL);
  if (!IS_PUBLIC) {
    list.add(`http://${HOST}:${PORT}`);
    list.add(`http://localhost:${PORT}`);
  }
  return list;
})();

export function originAllowed(origin) {
  if (!origin) return true;
  return ORIGINS.has(origin.replace(/\/+$/, ''));
}

export function describe() {
  const lines = [];
  lines.push(`listening on http://${HOST}:${PORT}`);
  if (IS_PUBLIC) {
    lines.push(`public url ${PUBLIC_URL}`);
    lines.push(`secure cookies ${IS_HTTPS ? 'on' : 'OFF, NURU_PUBLIC_URL is not https'}`);
    lines.push(`trusting proxy headers ${TRUST_PROXY ? 'yes' : 'no'}`);
  } else {
    lines.push('development mode, set NURU_PUBLIC_URL to serve this publicly');
  }
  if (!OPEN_SIGNUP) lines.push('registration is closed');
  if (MODERATOR_EMAIL) lines.push(`first moderator ${MODERATOR_EMAIL}`);
  else if (IS_PUBLIC) lines.push('no NURU_MODERATOR_EMAIL, nobody will be made a moderator on signup');
  return lines;
}
