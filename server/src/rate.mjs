import { TRUST_PROXY } from './config.mjs';

const buckets = new Map();

export const RULES = {
  login: { limit: 8, windowMs: 10 * 60 * 1000 },
  register: { limit: 4, windowMs: 60 * 60 * 1000 },
  upload: { limit: 20, windowMs: 60 * 60 * 1000 },
  write: { limit: 120, windowMs: 10 * 60 * 1000 },
  report: { limit: 12, windowMs: 60 * 60 * 1000 },
};

export function clientIp(req) {
  if (TRUST_PROXY) {
    const fwd = req.headers['x-forwarded-for'];
    if (fwd) return String(fwd).split(',')[0].trim();
    const real = req.headers['x-real-ip'];
    if (real) return String(real).trim();
  }
  return req.socket?.remoteAddress ?? 'unknown';
}

export function take(rule, key) {
  const spec = RULES[rule];
  if (!spec) return { ok: true, retryAfter: 0 };
  const now = Date.now();
  const id = `${rule}:${key}`;
  const hit = buckets.get(id);
  if (!hit || now >= hit.resetAt) {
    buckets.set(id, { count: 1, resetAt: now + spec.windowMs });
    return { ok: true, retryAfter: 0 };
  }
  hit.count += 1;
  if (hit.count > spec.limit) {
    return { ok: false, retryAfter: Math.ceil((hit.resetAt - now) / 1000) };
  }
  return { ok: true, retryAfter: 0 };
}

export function forget(rule, key) {
  buckets.delete(`${rule}:${key}`);
}

export function sweep() {
  const now = Date.now();
  for (const [id, hit] of buckets) {
    if (now >= hit.resetAt) buckets.delete(id);
  }
}
