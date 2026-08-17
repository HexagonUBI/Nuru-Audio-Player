# Elpy placeholder pack — temporary

Everything else in this folder is generated from Elpy 1.1.4.0 (© Vane Jung) and
is **development material only**. It is bundled into local builds so there is
something to listen to while Nuru's own recordings are sourced. It is not ours to
publish.

Only this README is committed; the audio, cover art and catalogue are gitignored.

## Regenerating

```bash
npm run pack:placeholder
```

Reads `dev-notes/elpy-reference/assets/`, transcodes to FLAC and writes
`catalogue.json`, `audio/` and `covers/` here.

## Removing it

This pack is deliberately self-contained so it can be deleted in one move:

1. Delete everything in this folder except this README.
2. Drop the `../resources/packs/elpy-placeholder` entry from `resources` in
   `src-tauri/tauri.conf.json`.
3. Delete this folder and the `pack:placeholder` script.

Nothing else references it. Sounds are discovered by scanning pack directories,
so removing one changes no code.

## Why it is safe to forget about

Every entry is written with `provenance.shippable: false`, and:

- the app logs a warning at startup naming the count,
- Settings shows a "Development build" panel listing them,
- `Library::unshippable()` returns them for a release check to fail on.
