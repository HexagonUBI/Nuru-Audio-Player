import { createHash } from 'node:crypto';
import { cpSync, existsSync, mkdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const { version } = JSON.parse(readFileSync(join(root, 'version.json'), 'utf8'));
if (!/^\d+\.\d+\.\d+\.\d+$/.test(version)) {
  console.error(`version.json: expected four numeric parts, got "${version}"`);
  process.exit(1);
}

const run = (cmd, args) => execFileSync(cmd, args, { cwd: root, stdio: 'inherit' });

run('cargo', ['build', '--release', '-p', 'nuru', '-p', 'nuru-restart']);

const meta = JSON.parse(
  execFileSync('cargo', ['metadata', '--format-version', '1', '--no-deps'], { cwd: root, encoding: 'utf8' }),
);
const bin = join(meta.target_directory, 'release');
const stage = join(meta.target_directory, 'installer-stage');
rmSync(stage, { recursive: true, force: true });
mkdirSync(stage, { recursive: true });

cpSync(join(bin, 'nuru.exe'), join(stage, 'nuru.exe'));
cpSync(join(bin, 'nuru-restart.exe'), join(stage, 'nuru-restart.exe'));
cpSync(join(root, 'crates', 'nuru', 'ui', 'assets', 'icon.ico'), join(stage, 'icon.ico'));
const packs = process.env.NURU_PACKS_SRC || join(root, 'resources', 'packs');
cpSync(packs, join(stage, 'packs'), {
  recursive: true,
  filter: (src) => !src.endsWith('README.md') && !src.endsWith('DO-NOT-SHIP.md'),
});

const nsisHome = process.env.NSIS_HOME || join(process.env.LOCALAPPDATA ?? '', 'tauri', 'NSIS');
const makensis = join(nsisHome, 'makensis.exe');
if (!existsSync(makensis)) {
  console.error(`makensis not found at ${makensis}. Set NSIS_HOME.`);
  process.exit(1);
}

const outDir = join(root, 'release', version);
mkdirSync(outDir, { recursive: true });
const name = `Nuru_${version}_x64-setup.exe`;
const out = join(outDir, name);
run(makensis, ['-V2', `-DVERSION=${version}`, `-DSTAGE=${stage}`, `-DOUTFILE=${out}`, join(root, 'installer', 'nuru.nsi')]);

const bytes = readFileSync(out);
const build = {
  version,
  date: new Date().toISOString().slice(0, 10),
  download: name,
  size: statSync(out).size,
  sha256: createHash('sha256').update(bytes).digest('hex'),
};
writeFileSync(join(outDir, 'build.json'), JSON.stringify(build, null, 2) + '\n');
console.log(JSON.stringify(build, null, 2));
