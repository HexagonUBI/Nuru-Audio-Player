import { rmSync, readdirSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { DATA, open, all } from './src/db.mjs';

if (!existsSync(DATA)) {
  console.log('nothing to reset');
  process.exit(0);
}

if (!process.argv.includes('--yes')) {
  let owned = [];
  try {
    open();
    owned = all(
      `SELECT s.slug, s.name, u.email FROM sounds s
       LEFT JOIN users u ON u.id = s.uploader_id
       WHERE u.email IS NOT NULL AND u.email NOT LIKE '%@nuru.local'`,
    );
  } catch {
    owned = [];
  }
  console.error('This deletes the database AND every uploaded file in server/data.');
  if (owned.length) {
    console.error('');
    console.error(`${owned.length} upload(s) were made by real accounts and have no other copy:`);
    for (const o of owned) console.error(`  ${o.slug}  ${o.name}  (${o.email})`);
  }
  console.error('');
  console.error('Re-run with --yes if that is what you want:');
  console.error('  npm run db:reset -- --yes');
  process.exit(1);
}

let failed = 0;
for (const entry of readdirSync(DATA)) {
  try {
    rmSync(join(DATA, entry), { recursive: true, force: true });
  } catch (e) {
    failed += 1;
    console.error(`could not remove ${entry}: ${e.code ?? e.message}`);
  }
}

if (failed) {
  console.error('');
  console.error('Some files are locked. Stop the server and try again.');
  console.error('Note: files that were NOT locked have already been deleted.');
  process.exit(1);
}
console.log('database cleared');
