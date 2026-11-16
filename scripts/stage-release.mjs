import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const site = process.env.NURU_SITE || join(root, '..', '.website-repos', 'simplefox-studio', 'site');
const { version } = JSON.parse(readFileSync(join(root, 'version.json'), 'utf8'));

const buildFile = join(root, 'release', version, 'build.json');
const notesFile = join(root, 'release-notes', `${version}.json`);
for (const f of [buildFile, notesFile]) {
  if (!existsSync(f)) {
    console.error(`missing ${f}`);
    process.exit(1);
  }
}
const build = JSON.parse(readFileSync(buildFile, 'utf8'));
const notes = JSON.parse(readFileSync(notesFile, 'utf8'));
if (!Array.isArray(notes.entries) || notes.entries.some((e) => !e.title || !e.text)) {
  console.error(`${notesFile}: expected {"entries":[{"title","text"}]}`);
  process.exit(1);
}

const dir = join(site, 'apps', 'nuru');
mkdirSync(join(dir, 'files'), { recursive: true });
copyFileSync(join(root, 'release', version, build.download), join(dir, 'files', build.download));

const feedFile = join(dir, 'changelog.json');
const feed = existsSync(feedFile)
  ? JSON.parse(readFileSync(feedFile, 'utf8'))
  : { app: 'nuru', showcase: [], releases: [], builds: {} };
feed.releases = [
  { version, date: build.date, entries: notes.entries },
  ...feed.releases.filter((r) => r.version !== version),
];
feed.builds = { ...feed.builds, windows: build };
writeFileSync(feedFile, JSON.stringify(feed, null, 2) + '\n');

console.log(`staged ${build.download} and ${feedFile}. Nothing is deployed.`);
