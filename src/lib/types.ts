/* Shared shapes between the Svelte UI and the Rust audio engine.
   The Rust side mirrors these in src-tauri/src/model.rs — keep them in step. */

/** Which part of the Nook Mode scene a sound drives. */
export type NookChannel =
  | 'sky' // time of day: birds → morning, crickets → night
  | 'weather' // rain, snow, thunder, wind
  | 'window' // what is visible outside: forest, city, beach
  | 'hearth' // fire in the room
  | 'life' // people and animals
  | 'transit' // things passing by
  | 'none'; // noise generators sit outside the scene

export interface NookRole {
  channel: NookChannel;
  /** State name the scene renderer switches on, e.g. 'rain' | 'downpour'. */
  state: string;
  /** How strongly this sound claims its channel when several compete (0–1). */
  weight: number;
}

/** Where a sound came from, and whether it may legally ship. */
export interface SoundProvenance {
  /** Free-text origin, e.g. 'freesound:12345', 'recorded:nuru', 'placeholder:elpy'. */
  origin: string;
  /** SPDX id, 'CC0-1.0', or the sentinel below for dev-only material. */
  licence: string;
  attribution: string | null;
  /** True only when the file is cleared for public builds. The build script
   *  refuses to bundle a sound with this false. */
  shippable: boolean;
}

export interface LoopPoints {
  /** First sample of the loop body, inclusive. */
  startSample: number;
  /** One past the last sample of the loop body. null = end of file. */
  endSample: number | null;
  /** Equal-power crossfade length. 0 means the seam is already sample-exact and
   *  no fade is applied — always prefer 0 and fix the source instead. */
  crossfadeMs: number;
  /** How the points were arrived at, for auditing. */
  method: 'exact' | 'measured' | 'crossfade' | 'untuned';
}

export interface AudioFileInfo {
  /** Path relative to the sound pack root. */
  file: string;
  bytes: number;
  sha256: string;
  sampleRate: number;
  channels: number;
  /** Total frames in the decoded file. */
  frames: number;
  codec: 'flac' | 'opus' | 'wav';
}

export interface Sound {
  id: string;
  name: string;
  /** Hex accent that drives the tile, its glow and its fader. */
  accent: string;
  tags: string[];
  /** Cover image path relative to the pack root. */
  cover: string;
  audio: AudioFileInfo;
  loop: LoopPoints;
  nook: NookRole;
  provenance: SoundProvenance;
}

export interface Catalogue {
  schema: 1;
  /** Pack identity — 'builtin' for the bundled set, otherwise a database id. */
  pack: string;
  packName: string;
  sounds: Sound[];
}

/** A sound resolved against this machine — what `list_sounds` returns. */
export interface SoundEntry extends Sound {
  pack: string;
  /** Absolute path to the audio file locally. Never a URL. */
  audioPath: string;
  coverPath: string | null;
  /** True once the file has been checked against its recorded hash. */
  verified: boolean;
}

export interface EngineStatus {
  device: string;
  sampleRate: number;
  channels: number;
  maxLayers: number;
  active: string[];
  underruns: number;
}

/* ── Runtime state ───────────────────────────────────────────────────────── */

/** One sound currently loaded into the mixer. */
export interface Layer {
  soundId: string;
  /** 0–1, independent of master. */
  volume: number;
  muted: boolean;
  /** Set while the file is being fetched/cached before it can play. */
  loading: boolean;
}

export interface Preset {
  id: string;
  name: string;
  createdAt: string;
  layers: Array<{ soundId: string; volume: number }>;
  masterVolume: number | null;
}

export type TimerState =
  | { kind: 'off' }
  | { kind: 'running'; endsAt: number; totalMs: number }
  | { kind: 'finished' };

/** Marker for material that must never reach a public build. */
export const DEV_PLACEHOLDER_LICENCE = 'UNLICENSED-DEV-PLACEHOLDER';
