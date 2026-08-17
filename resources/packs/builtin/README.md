# Nuru Essentials

The sound pack that ships inside the installer. Empty until Nuru has audio it
owns — see the licensing note below.

A pack is a directory with this shape:

```
catalogue.json
audio/<id>.flac
covers/<id>.png
```

`catalogue.json` is validated against `src/lib/types.ts` (and its Rust mirror in
`src-tauri/src/model.rs`). Every entry needs:

| Field | Why it matters |
| --- | --- |
| `audio.sha256` | Checked before the file is allowed to play. A pack downloaded from the sound database is not playable until it verifies. |
| `audio.frames` | The engine's loop end when `loop.endSample` is null. |
| `loop.startSample` / `loop.endSample` | Sample-exact loop bounds. Not seconds — a float second is not a sample. |
| `loop.crossfadeMs` | `0` when the material was authored to loop end-to-start. Anything else blends the tail into a copy of the head over this window. |
| `loop.method` | `exact` for authored loops, `crossfade` for field recordings, `untuned` for anything not yet checked. |
| `provenance.shippable` | Must be `true`. The engine loads unshippable sounds so development works, but flags them in Settings and the release build refuses to bundle them. |
| `nook` | Which part of the Nook Mode scene this sound drives. `channel: "none"` opts out. |

## Audio format

FLAC, at the source's native rate, 16-bit unless the master is genuinely
higher-resolution.

Not AAC or MP3. Both carry encoder delay and padding, and both seek to a codec
frame rather than a sample, which makes a sample-exact loop impossible — the
seam is either a click or a hiccup. FLAC decodes to exactly the samples that
went in.

Opus is acceptable for large downloadable packs where size matters: it is
gapless-native and carries an explicit pre-skip. Do not use it for anything
tonal.

## Licensing

Nothing goes in here unless Nuru has the right to distribute it. CC0, a licence
that permits redistribution in a commercial application, or original recordings.
Record the source in `provenance.origin` and any required credit in
`provenance.attribution`.

The development placeholders under `dev-notes/placeholder-pack/` are extracted
from another application and are **not** shippable. They exist so the engine has
real material to loop.
