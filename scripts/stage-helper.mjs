import { copyFileSync, existsSync, mkdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const built = join(root, 'src-tauri', 'target', 'release', 'nuru-restart.exe');
const target = join(root, 'resources', 'nuru-restart.exe');

if (!existsSync(built)) {
  console.error(`stage-helper: ${built} is missing, build it first`);
  process.exit(1);
}

mkdirSync(dirname(target), { recursive: true });
copyFileSync(built, target);
console.log('staged the restart window into resources');
