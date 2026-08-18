<h1>Nuru</h1>

Nuru plays any combination of ambient sounds at once - rain over a cafe over a
distant train - each seamlessly looped, mixed locally, and saved as a mix you can
come back to.

A native Windows app: `.exe` plus an MSI installer. Nothing streams while it
plays.

### Features

| Feature | Description | Implementation |
| --- | --- | --- |
| Main Interface | Basic app design | **Done** - custom titlebar, mixer column, tile grid |
| Audio seamless looping | Proper seamless loop of an audio | **Done** - measured, not assumed; see the tests below |
| Nook Mode | Opens in a fullscreen, dynamic cozy nook room design that changes based on sounds enabled | **Scene system done**, procedural art pending real assets |
| Presets | Save/load presets of your settings and audio toggles | **Done** |
| Settings | Self-explanatory | **Done** - themes, launch behaviour, audio info |
| Volume | Audio volume for all of audio and individually | **Done** |
| Image support | Thumbnails for each audio slot | **Done** |
| Proper performance | Optimization so it doesnt eat up ram | **Done by design** - streaming decode, ~400 KB buffered per sound |
| Play/pause | pause and play audio | **Done** |
| Audio database | Download custom user-made audios right from the app | Pack format + verification done, download UI pending |
| Timer | Timer to pause all audio after | **Done** |

## Why it sounds right

The looping is the reason this is a remake rather than a reskin.

The usual approach - watch the playback position and seek back near the end -
cannot be seamless. The position event fires every ~250 ms, a seek on a
compressed stream lands on a codec frame rather than a sample, and AAC and MP3
both carry encoder delay that shifts the decoded output away from where the file
says it starts. It passes unnoticed on rain and falls apart on anything tonal.

Nuru instead decodes local FLAC, wraps at an exact sample index inside the
producer thread, and blends the tail into a copy of the head with an equal-power
crossfade when the material wasn't authored to loop. Faders are square-law with
one-pole smoothing, so nothing clicks and nothing zippers.

Audio is always a verified local file. A downloaded pack is written to disk and
checked against its hash before any of it is playable, so a dropped connection
can never become a gap in the sound.

This is tested rather than asserted. A seam is a discontinuity in the waveform,
so it is measurable: `src-tauri/src/audio/tests.rs` synthesises a sine, takes the
largest sample-to-sample step across the loop point, and compares it to the
steepest step the waveform produces on its own. One of the tests deliberately
mis-times a loop and asserts the seam **is** detected - without it the others
could pass by measuring nothing.

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

## Stack

- **Tauri 2** - native window, `.exe`, WiX MSI and NSIS installers
- **Rust** audio engine - `cpal` output, `symphonia` decode, `rubato` resampling,
  `rtrb` lock-free rings. No allocation, no locks and no blocking in the audio
  callback.
- **Svelte 5 + TypeScript + Vite** frontend
- **Segoe UI** for running text, **Comfortaa** (bundled, OFL) for display

## Running it

```bash
npm install
```

The interface runs in a plain browser with the audio engine detached - useful for
design work:

```bash
npm run dev
```

The real app needs the Rust toolchain (`rustup`, MSVC target) and the WebView2
runtime:

```bash
npm run app:dev
```

Build the installers:

```bash
npm run app:build
```

## Versions

Four parts, `MAJOR.MINOR.PATCH.BUILD`, currently `0.0.0.0`.

`version.json` is the only place it is written by hand. `npm run version:sync`
propagates it into `package.json`, `Cargo.toml`, `tauri.conf.json`, the WiX
product version and `src/lib/version.ts`. npm, Cargo and Tauri each validate
their field as three-part semver, so they get the first three.

Everything the user actually sees carries all four:

| Surface | Shows |
| --- | --- |
| App titlebar and Settings | `0.0.0.0` |
| MSI `ProductVersion` | `0.0.0.0` |
| `nuru.exe` Properties | `0.0.0.0` |
| Installer *filenames* | `0.0.0` - Tauri names bundles from the semver field |

The exe needs a second step: `tauri-build` derives its version resource from
Cargo's three-part version and overwrites anything set through
`[package.metadata.winresource]`, so `scripts/patch-version-resource.mjs`
restamps it after linking and before bundling, wired up as `beforeBundleCommand`.

## Sounds

A pack is a folder holding `catalogue.json`, `audio/` and `covers/`. Nuru reads
packs from its resources folder and from `%APPDATA%\app.nuru.player\packs\`.
Format and licensing rules are in
[resources/packs/builtin/README.md](resources/packs/builtin/README.md).

The bundled pack is currently empty. Development uses placeholder audio that is
**not** cleared to ship - Settings shows a warning whenever any is loaded.

## Repository

```
src/               Svelte frontend
src-tauri/         Rust: audio engine, pack library, Tauri commands
resources/packs/   Sound packs that ship in the installer
scripts/           Version sync, icon generation, placeholder pack build
docs/              Website for the downloadable sound database
dev-notes/         Private references and placeholders (gitignored)
```
