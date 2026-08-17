/**
 * The one place the UI touches the backend.
 *
 * Nuru's UI has to run in two places: inside Tauri, where audio is real, and in
 * a plain browser, where it is being designed. Rather than sprinkling `if
 * (isTauri)` through components, everything goes through this module and the
 * preview implementation is swapped in wholesale.
 *
 * Preview mode is deliberately silent. It would be easy to make `<audio>` play
 * the FLAC files, but that would be a different loop implementation from the one
 * that ships, and a design preview that sounds wrong is worse than one that says
 * nothing.
 */

import type { EngineStatus, SoundEntry, Catalogue } from './types';

export type Runtime = 'tauri' | 'preview';

export const RUNTIME: Runtime =
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window ? 'tauri' : 'preview';

export const IS_PREVIEW = RUNTIME === 'preview';

type Invoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;

let invoke: Invoke = async () => {
  throw new Error('backend not ready');
};

if (RUNTIME === 'tauri') {
  const mod = await import('@tauri-apps/api/core');
  invoke = mod.invoke as Invoke;
}

/* ── Asset URLs ────────────────────────────────────────────────────────────
   Under Tauri, cover art is read from disk through the asset protocol, which
   the CSP and the capability file both scope to the pack directories. In the
   browser it comes from the dev-pack middleware in vite.config.ts. */

let convertFileSrc: (p: string) => string = (p) => p;
if (RUNTIME === 'tauri') {
  const mod = await import('@tauri-apps/api/core');
  convertFileSrc = mod.convertFileSrc;
}

export function coverUrl(entry: SoundEntry): string | null {
  if (RUNTIME === 'preview') return `/devpack/${entry.cover}`;
  return entry.coverPath ? convertFileSrc(entry.coverPath) : null;
}

/* ── Preview backend ───────────────────────────────────────────────────────── */

const previewState = {
  active: new Set<string>(),
  loaded: null as SoundEntry[] | null,
};

async function previewSounds(): Promise<SoundEntry[]> {
  if (previewState.loaded) return previewState.loaded;
  try {
    const res = await fetch('/devpack/catalogue.json');
    if (!res.ok) throw new Error(String(res.status));
    const cat: Catalogue = await res.json();
    previewState.loaded = cat.sounds.map((s) => ({
      ...s,
      pack: cat.pack,
      audioPath: '',
      coverPath: null,
      verified: false,
    }));
  } catch {
    previewState.loaded = [];
  }
  return previewState.loaded;
}

/* ── API ───────────────────────────────────────────────────────────────────── */

export const api = {
  async listSounds(): Promise<SoundEntry[]> {
    if (IS_PREVIEW) return previewSounds();
    return invoke<SoundEntry[]>('list_sounds');
  },

  async playSound(soundId: string, volume: number): Promise<void> {
    if (IS_PREVIEW) {
      previewState.active.add(soundId);
      return;
    }
    return invoke('play_sound', { soundId, volume });
  },

  async stopSound(soundId: string): Promise<void> {
    if (IS_PREVIEW) {
      previewState.active.delete(soundId);
      return;
    }
    return invoke('stop_sound', { soundId });
  },

  async setSoundVolume(soundId: string, volume: number): Promise<void> {
    if (IS_PREVIEW) return;
    return invoke('set_sound_volume', { soundId, volume });
  },

  async setMasterVolume(volume: number): Promise<void> {
    if (IS_PREVIEW) return;
    return invoke('set_master_volume', { volume });
  },

  async setPlaying(playing: boolean): Promise<void> {
    if (IS_PREVIEW) return;
    return invoke('set_playing', { playing });
  },

  async stopAll(): Promise<void> {
    if (IS_PREVIEW) {
      previewState.active.clear();
      return;
    }
    return invoke('stop_all');
  },

  async status(): Promise<EngineStatus> {
    if (IS_PREVIEW) {
      return {
        device: 'Preview (no audio engine)',
        sampleRate: 0,
        channels: 0,
        maxLayers: 32,
        active: [...previewState.active],
        underruns: 0,
      };
    }
    return invoke<EngineStatus>('engine_status');
  },

  async unshippableSounds(): Promise<string[]> {
    if (IS_PREVIEW) {
      const sounds = await previewSounds();
      return sounds.filter((s) => !s.provenance.shippable).map((s) => s.id);
    }
    return invoke<string[]>('unshippable_sounds');
  },
};

/* ── Window controls ───────────────────────────────────────────────────────── */

export const win = {
  async minimize() {
    if (IS_PREVIEW) return;
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await getCurrentWindow().minimize();
  },
  async toggleMaximize() {
    if (IS_PREVIEW) return;
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await getCurrentWindow().toggleMaximize();
  },
  async close() {
    if (IS_PREVIEW) return;
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await getCurrentWindow().close();
  },
  async setFullscreen(on: boolean) {
    if (IS_PREVIEW) {
      if (on) await document.documentElement.requestFullscreen().catch(() => {});
      else if (document.fullscreenElement) await document.exitFullscreen().catch(() => {});
      return;
    }
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await getCurrentWindow().setFullscreen(on);
  },
};
