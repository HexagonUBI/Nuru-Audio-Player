

import type { EngineStatus, SoundEntry, Catalogue } from './types';


export interface Progress {
  phase: string;
  label: string;
  progress: number;
  done: boolean;
  error: string | null;
}

export const BOOT_EVENT = 'nuru://boot';

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



let convertFileSrc: (p: string) => string = (p) => p;
if (RUNTIME === 'tauri') {
  const mod = await import('@tauri-apps/api/core');
  convertFileSrc = mod.convertFileSrc;
}

export function coverUrl(entry: SoundEntry): string | null {
  if (RUNTIME === 'preview') return `/devpack/${entry.cover}`;
  return entry.coverPath ? convertFileSrc(entry.coverPath) : null;
}



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
      packName: cat.packName,
      builtin: true,
      audioPath: '',
      coverPath: null,
      verified: false,
    }));
  } catch {
    previewState.loaded = [];
  }
  return previewState.loaded;
}




export async function onProgress(
  event: string,
  cb: (p: Progress) => void,
): Promise<() => void> {
  if (IS_PREVIEW) {
    const steps: Progress[] = [
      { phase: 'engine', label: 'Preview, no audio engine', progress: 0.1, done: false, error: null },
      { phase: 'packs', label: 'Reading sound packs', progress: 0.25, done: false, error: null },
      { phase: 'verify', label: 'Checking sounds', progress: 0.7, done: false, error: null },
      { phase: 'ready', label: '', progress: 1, done: true, error: null },
    ];
    let i = 0;
    const timer = setInterval(() => {
      if (i >= steps.length) return clearInterval(timer);
      cb(steps[i++]);
    }, 260);
    return () => clearInterval(timer);
  }
  const { listen } = await import('@tauri-apps/api/event');
  const un = await listen<Progress>(event, (e) => cb(e.payload));
  return un;
}

export const api = {
  async boot(): Promise<void> {
    if (IS_PREVIEW) return;
    return invoke('boot');
  },

  async listOutputDevices(): Promise<{
    devices: string[];
    saved: string | null;
    active: string;
  }> {
    if (IS_PREVIEW) {
      return { devices: ['Preview output'], saved: null, active: 'Preview output' };
    }
    const [devices, saved, active] =
      await invoke<[string[], string | null, string]>('list_output_devices');
    return { devices, saved, active };
  },

  async setOutputDevice(device: string | null): Promise<string> {
    if (IS_PREVIEW) return 'Preview output';
    return invoke<string>('set_output_device', { device });
  },

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

  async setPresence(
    details: string,
    status: string,
    active: boolean,
    startedAt: number | null,
  ): Promise<void> {
    if (IS_PREVIEW) return;
    return invoke('set_presence', { details, status, active, startedAt });
  },

  async now(): Promise<number> {
    if (IS_PREVIEW) return Math.floor(Date.now() / 1000);
    return invoke<number>('discord_now');
  },

  async unshippableSounds(): Promise<string[]> {
    if (IS_PREVIEW) {
      const sounds = await previewSounds();
      return sounds.filter((s) => !s.provenance.shippable).map((s) => s.id);
    }
    return invoke<string[]>('unshippable_sounds');
  },
};



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
    const w = getCurrentWindow();

    if (on) {
      wasMaximized = await w.isMaximized();
      if (wasMaximized) await w.unmaximize();
      await w.setFullscreen(true);
    } else {
      await w.setFullscreen(false);
      if (wasMaximized) await w.maximize();
      wasMaximized = false;
    }
  },
};

let wasMaximized = false;
