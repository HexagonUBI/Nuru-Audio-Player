import { cpSync, existsSync, mkdirSync, mkdtempSync, rmSync, statSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { tmpdir } from 'node:os';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const source = join(root, 'server');
const outDir = join(root, 'release');
const out = join(outDir, 'nuru-workshop.zip');

const INCLUDE = ['src', 'public', 'index.js', 'package.json', 'seed.mjs', 'tags.mjs', 'promote.mjs', 'reset.mjs'];
const NEVER = ['data', 'deploy', 'node_modules'];

if (!existsSync(source)) {
  console.error('no server directory');
  process.exit(1);
}

const stage = mkdtempSync(join(tmpdir(), 'nuru-workshop-'));

try {
  for (const name of INCLUDE) {
    const from = join(source, name);
    if (!existsSync(from)) {
      console.error(`missing ${name}`);
      process.exit(1);
    }
    cpSync(from, join(stage, name), { recursive: true });
  }

  for (const name of NEVER) {
    if (existsSync(join(stage, name))) {
      console.error(`refusing to pack ${name}`);
      process.exit(1);
    }
  }

  mkdirSync(outDir, { recursive: true });
  rmSync(out, { force: true });

  execFileSync(
    'powershell',
    [
      '-NoProfile',
      '-Command',
      `Compress-Archive -Path '${stage}\\*' -DestinationPath '${out}' -Force`,
    ],
    { stdio: 'inherit' },
  );

  const size = statSync(out).size;
  console.log(`wrote ${out}`);
  console.log(`${(size / 1024 / 1024).toFixed(2)} MB, no data directory inside`);
  console.log('upload this, then set the entry file to index.js and node to 24');
} finally {
  rmSync(stage, { recursive: true, force: true });
}
