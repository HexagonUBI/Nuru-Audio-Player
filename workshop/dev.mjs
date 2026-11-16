import { spawn, execFileSync } from 'node:child_process';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const WORKSHOP = path.dirname(fileURLToPath(import.meta.url));
const DATA = path.join(WORKSHOP, '.dev');
const KEY = path.join(DATA, 'dev-key.pem');
const PORT = Number(process.env.NURU_WS_PORT || 8810);
const MOD = 'a-devmoddevmoddevm';

fs.mkdirSync(DATA, { recursive: true });
if (!fs.existsSync(KEY)) {
  const { privateKey } = crypto.generateKeyPairSync('ed25519');
  fs.writeFileSync(KEY, privateKey.export({ format: 'pem', type: 'pkcs8' }));
}
const privateKey = crypto.createPrivateKey(fs.readFileSync(KEY));
const rawPublic = Buffer.from(crypto.createPublicKey(privateKey).export({ format: 'jwk' }).x, 'base64url');
const config = path.join(DATA, 'config.php');
if (!fs.existsSync(config)) {
  const db = path.join(DATA, 'ws.sqlite').split(path.sep).join('/');
  fs.writeFileSync(config, `<?php\nreturn [\n  'db' => ['dsn' => 'sqlite:${db}'],\n  'ticket_public_key' => '${rawPublic.toString('base64')}',\n  'secret' => '${crypto.randomBytes(16).toString('hex')}',\n  'moderators' => ['${MOD}'],\n];\n`);
}

function ticket(sub, name) {
  const iat = Math.floor(Date.now() / 1000);
  const payload = JSON.stringify({ v: 1, sub, name, app: 'nuru', key: null, dev: 'dev', iat, exp: iat + 604800 });
  return `${Buffer.from(payload).toString('base64url')}.${crypto.sign(null, Buffer.from(payload), privateKey).toString('base64url')}`;
}

if (process.argv[2] === 'ticket') {
  const sub = process.argv[3] || MOD;
  if (!/^a-[a-z2-7]{16}$/.test(sub)) throw new Error('account ids look like a- and 16 of a-z, 2-7');
  console.log(ticket(sub, process.argv[4] || 'Dev'));
  process.exit(0);
}

const php = execFileSync('php', ['-r', 'echo PHP_BINARY;'], { encoding: 'utf8' }).trim();
const args = ['-d', 'opcache.enable=0', '-d', `extension_dir=${path.join(path.dirname(php), 'ext')}`,
  ...['sodium', 'pdo_sqlite', 'mbstring'].flatMap((e) => ['-d', `extension=${e}`]),
  '-S', `127.0.0.1:${PORT}`, '-t', path.join(WORKSHOP, 'public'), path.join(WORKSHOP, 'router.php')];
console.log(`Workshop on http://127.0.0.1:${PORT}`);
console.log(`Moderator ticket (${MOD}):\n${ticket(MOD, 'Dev moderator')}`);
spawn(php, args, { env: { ...process.env, NURU_WS_DATA: DATA }, stdio: 'inherit' });
