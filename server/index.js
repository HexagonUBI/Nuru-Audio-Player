import { check } from './src/preflight.mjs';

const state = check();

if (!state.ok) {
  console.error('nuru workshop cannot start');
  for (const line of state.notes) console.error(`  ${line}`);
  process.exit(1);
}

await import('./src/server.mjs');
