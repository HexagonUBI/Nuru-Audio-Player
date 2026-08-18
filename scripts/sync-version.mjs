
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const read = (p) => readFileSync(join(root, p), 'utf8');
const write = (p, s) => writeFileSync(join(root, p), s);

const { version: full, channel } = JSON.parse(read('version.json'));

const parts = full.split('.');
if (parts.length !== 4 || parts.some((p) => !/^\d+$/.test(p))) {
  console.error(`version.json: expected four numeric parts, got "${full}"`);
  process.exit(1);
}
const [major, minor, patch, build] = parts;
const semver = `${major}.${minor}.${patch}`;

const pkg = JSON.parse(read('package.json'));
pkg.version = semver;
write('package.json', JSON.stringify(pkg, null, 2) + '\n');

const cargo = read('src-tauri/Cargo.toml');
let section = '';
write(
  'src-tauri/Cargo.toml',
  cargo
    .split('\n')
    .map((line) => {
      const header = line.trim().match(/^\[([^\]]+)\]$/);
      if (header) {
        section = header[1];
        return line;
      }
      if (section === 'package' && /^version\s*=/.test(line.trim())) {
        return `version = "${semver}"`;
      }
      if (section === 'package.metadata.winresource') {
        const m = line.trim().match(/^(FileVersion|ProductVersion)\s*=/);
        if (m) return `${m[1]} = "${full}"`;
      }
      return line;
    })
    .join('\n'),
);

const conf = JSON.parse(read('src-tauri/tauri.conf.json'));
conf.version = semver;
write('src-tauri/tauri.conf.json', JSON.stringify(conf, null, 2) + '\n');

write(
  'src-tauri/src/version.rs',
  `pub const FULL: &str = "${full}";
pub const CHANNEL: &str = "${channel}";
`,
);

write(
  'src/lib/version.ts',
  `export const VERSION = '${full}';
export const VERSION_SEMVER = '${semver}';
export const BUILD = ${Number(build)};
export const CHANNEL = '${channel}';
`,
);

console.log(`version ${full}  ->  semver ${semver}, build ${build}, channel ${channel}`);
