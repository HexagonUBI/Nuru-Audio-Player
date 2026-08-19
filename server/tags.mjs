import { open, all, one, run } from './src/db.mjs';
import { OFFICIAL_TAGS } from './src/taxonomy.mjs';

open();

let added = 0;
let promoted = 0;
for (const [category, names] of Object.entries(OFFICIAL_TAGS)) {
  for (const name of names) {
    const found = one('SELECT id, status, category FROM tags WHERE name = :name', { name });
    if (!found) {
      run(
        `INSERT INTO tags (name, category, status, created_at)
         VALUES (:name, :category, 'official', :now)`,
        { name, category, now: Date.now() },
      );
      added += 1;
    } else if (found.status !== 'official' || found.category !== category) {
      run("UPDATE tags SET status = 'official', category = :c WHERE id = :id", {
        c: category,
        id: found.id,
      });
      promoted += 1;
    }
  }
}

const official = new Set(Object.values(OFFICIAL_TAGS).flat());
const strays = all('SELECT id, name FROM tags').filter((t) => !official.has(t.name));

const ALIASES = {
  nature: 'forest',
  water: 'river',
  weather: 'rain',
  urban: 'city',
  life: 'birds',
  indoor: 'home',
  'dark ambient': 'melancholy',
  gaming: 'busy',
  test: null,
  edited: null,
  night: 'night',
};

let remapped = 0;
let orphaned = 0;
for (const stray of strays) {
  const target = ALIASES[stray.name];
  if (target && official.has(target)) {
    const to = one('SELECT id FROM tags WHERE name = :n', { n: target });
    for (const link of all('SELECT sound_id FROM sound_tags WHERE tag_id = :id', { id: stray.id })) {
      run('INSERT OR IGNORE INTO sound_tags (sound_id, tag_id) VALUES (:s, :t)', {
        s: link.sound_id,
        t: to.id,
      });
    }
    remapped += 1;
  } else {
    orphaned += 1;
  }
  run('DELETE FROM sound_tags WHERE tag_id = :id', { id: stray.id });
  run('DELETE FROM tags WHERE id = :id', { id: stray.id });
}

const total = all("SELECT COUNT(*) AS n FROM tags WHERE status = 'official'")[0].n;
console.log(`official tags : ${total} (${added} added, ${promoted} promoted)`);
console.log(`free text tags: ${strays.length} removed (${remapped} remapped, ${orphaned} dropped)`);
for (const [cat, names] of Object.entries(OFFICIAL_TAGS)) {
  console.log(`  ${cat.padEnd(8)} ${names.length}`);
}
