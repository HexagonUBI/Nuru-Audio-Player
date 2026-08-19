
export type NookChannel =
  | 'sky'
  | 'weather'
  | 'window'
  | 'hearth'
  | 'life'
  | 'transit'
  | 'none';

export interface NookRole {
  channel: NookChannel;
  state: string;
  weight: number;
}

export interface SoundProvenance {
  origin: string;
  licence: string;
  attribution: string | null;
  shippable: boolean;
}

export interface LoopPoints {
  startSample: number;
  endSample: number | null;
  crossfadeMs: number;
  method: 'exact' | 'measured' | 'crossfade' | 'untuned';
}

export interface AudioFileInfo {
  file: string;
  bytes: number;
  sha256: string;
  sampleRate: number;
  channels: number;
  frames: number;
  codec: 'flac' | 'opus' | 'wav';
}

export interface Sound {
  id: string;
  name: string;
  accent: string;
  tags: string[];
  cover: string;
  audio: AudioFileInfo;
  loop: LoopPoints;
  nook: NookRole;
  provenance: SoundProvenance;
}

export interface Catalogue {
  schema: 1;
  pack: string;
  packName: string;
  sounds: Sound[];
}

export interface SoundEntry extends Sound {
  pack: string;
  packName: string;
  builtin: boolean;
  audioPath: string;
  coverPath: string | null;
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

export interface Layer {
  soundId: string;
  volume: number;
  muted: boolean;
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

export const SCHEDULE_SLOTS = 48;
export const SLOT_MINUTES = 24 * 60 / SCHEDULE_SLOTS;

export type Schedule = Array<string | null>;

export const DEV_PLACEHOLDER_LICENCE = 'UNLICENSED-DEV-PLACEHOLDER';
