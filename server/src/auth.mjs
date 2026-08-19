import { randomBytes, scryptSync, timingSafeEqual } from 'node:crypto';
import { one, run } from './db.mjs';
import { IS_HTTPS, IS_PUBLIC } from './config.mjs';

const SESSION_DAYS = 14;
const SAME_SITE = IS_PUBLIC ? 'Lax' : 'Strict';
const SECURE = IS_HTTPS ? '; Secure' : '';

export function hashPassword(password, salt = randomBytes(16).toString('hex')) {
  const hash = scryptSync(password, salt, 64).toString('hex');
  return { hash, salt };
}

export function verifyPassword(password, hash, salt) {
  const attempt = scryptSync(password, salt, 64);
  const known = Buffer.from(hash, 'hex');
  if (attempt.length !== known.length) return false;
  return timingSafeEqual(attempt, known);
}

export function createSession(userId) {
  const token = randomBytes(32).toString('hex');
  const now = Date.now();
  run(
    `INSERT INTO sessions (token, user_id, created_at, expires_at)
     VALUES (:token, :user, :now, :exp)`,
    { token, user: userId, now, exp: now + SESSION_DAYS * 86400000 },
  );
  return token;
}

export function dropSession(token) {
  if (token) run('DELETE FROM sessions WHERE token = :token', { token });
}

export function userForToken(token) {
  if (!token) return null;
  const row = one(
    `SELECT u.id, u.email, u.name, u.handle, u.bio, u.role, s.expires_at
     FROM sessions s JOIN users u ON u.id = s.user_id
     WHERE s.token = :token`,
    { token },
  );
  if (!row) return null;
  if (row.expires_at < Date.now()) {
    dropSession(token);
    return null;
  }
  return { id: row.id, email: row.email, name: row.name, handle: row.handle, bio: row.bio, role: row.role };
}

export function cookieToken(req) {
  const raw = req.headers.cookie;
  if (!raw) return null;
  for (const part of raw.split(';')) {
    const [k, ...rest] = part.trim().split('=');
    if (k === 'nuru_session') return decodeURIComponent(rest.join('='));
  }
  return null;
}

export function sessionCookie(token) {
  const age = SESSION_DAYS * 86400;
  return `nuru_session=${encodeURIComponent(token)}; HttpOnly; SameSite=${SAME_SITE}; Path=/; Max-Age=${age}${SECURE}`;
}

export const clearCookie = `nuru_session=; HttpOnly; SameSite=${SAME_SITE}; Path=/; Max-Age=0${SECURE}`;
