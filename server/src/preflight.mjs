import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);

function parts(version) {
  return version.split('.').map((n) => Number(n));
}

export function check() {
  const version = process.versions.node;
  const [major, minor] = parts(version);

  let sqlite = true;
  try {
    require('node:sqlite');
  } catch {
    sqlite = false;
  }

  if (sqlite) return { ok: true, version, notes: [] };

  const lines = [`node ${version} cannot load node:sqlite, which this service is built on.`];

  if (major < 22 || (major === 22 && minor < 5)) {
    lines.push('node:sqlite arrived in 22.5. Pick node 24 or newer in your host panel.');
  } else if (major === 22 || (major === 23 && minor < 4)) {
    lines.push('this node has node:sqlite behind a flag. Start it with --experimental-sqlite,');
    lines.push('or pick node 24 or newer, which needs no flag.');
  } else {
    lines.push('node:sqlite should exist here. The build may have been compiled without it.');
  }

  return { ok: false, version, notes: lines };
}
