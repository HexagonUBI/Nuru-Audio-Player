
import { readFileSync, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { rcedit } from 'rcedit';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const { version: full } = JSON.parse(readFileSync(join(root, 'version.json'), 'utf8'));

const candidates = [
  join(root, 'src-tauri', 'target', 'release', 'nuru.exe'),
  join(root, 'src-tauri', 'target', 'debug', 'nuru.exe'),
];

const exe = candidates.find(existsSync);
if (!exe) {
  console.error('patch-version-resource: no nuru.exe found; nothing to stamp');
  process.exit(1);
}

await rcedit(exe, {
  'file-version': full,
  'product-version': full,
  'version-string': {
    FileVersion: full,
    ProductVersion: full,
    ProductName: 'Nuru',
    CompanyName: 'SimpleFox',
    FileDescription: 'Nuru - ambient sound mixer',
    LegalCopyright: 'Copyright (c) 2026 SimpleFox',
    InternalName: 'nuru',
    OriginalFilename: 'nuru.exe',
  },
});

console.log(`stamped ${full} onto ${exe}`);
