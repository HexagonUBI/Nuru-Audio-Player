/**
 * Application state.
 *
 * The engine is the source of truth for what is *audible*; this is the source of
 * truth for what the user asked for. They are kept in step by every mutation
 * here also calling the backend — never the other way round, because a UI that
 * waits for an audio thread to answer before it moves feels broken.
 */

import { api } from './bridge';
import type { Layer, Preset, SoundEntry, TimerState } from './types';

const STORAGE_KEY = 'nuru.state.v1';
const PRESET_KEY = 'nuru.presets.v1';

interface Persisted {
  masterVolume: number;
  theme: string;
  layers: Array<{ soundId: string; volume: number }>;
  alwaysOnTop: boolean;
  restoreOnLaunch: boolean;
}

function load<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    return raw ? { ...fallback, ...JSON.parse(raw) } : fallback;
  } catch {
    return fallback;
  }
}

function save(key: string, value: unknown) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    /* a full or blocked store must not take the app down */
  }
}

const saved = load<Persisted>(STORAGE_KEY, {
  masterVolume: 0.6,
  theme: 'midnight',
  layers: [],
  alwaysOnTop: false,
  restoreOnLaunch: true,
});

class NuruStore {
  /** Everything in every loaded pack. */
  sounds = $state<SoundEntry[]>([]);
  /** What is loaded into the mixer, in the order it was added. */
  layers = $state<Layer[]>([]);
  masterVolume = $state(saved.masterVolume);
  playing = $state(true);
  theme = $state(saved.theme);
  alwaysOnTop = $state(saved.alwaysOnTop);
  restoreOnLaunch = $state(saved.restoreOnLaunch);

  presets = $state<Preset[]>(load<{ items: Preset[] }>(PRESET_KEY, { items: [] }).items ?? []);

  timer = $state<TimerState>({ kind: 'off' });
  timerRemainingMs = $state(0);

  /** Tag filter on the grid; empty means everything. */
  filter = $state<string>('');
  search = $state('');

  nookMode = $state(false);
  activePanel = $state<'none' | 'presets' | 'timer' | 'settings'>('none');

  toasts = $state<Array<{ id: number; text: string; tone: 'info' | 'error' }>>([]);
  private nextToast = 0;

  loading = $state(true);
  engineNote = $state<string | null>(null);

  /* ── Derived ─────────────────────────────────────────────────────────── */

  byId = $derived(new Map(this.sounds.map((s) => [s.id, s])));

  tags = $derived.by(() => {
    const counts = new Map<string, number>();
    for (const s of this.sounds) for (const t of s.tags) counts.set(t, (counts.get(t) ?? 0) + 1);
    return [...counts.entries()].sort((a, b) => b[1] - a[1]).map(([t]) => t);
  });

  visibleSounds = $derived.by(() => {
    const q = this.search.trim().toLowerCase();
    return this.sounds.filter((s) => {
      if (this.filter && !s.tags.includes(this.filter)) return false;
      if (q && !s.name.toLowerCase().includes(q) && !s.tags.some((t) => t.includes(q))) return false;
      return true;
    });
  });

  activeIds = $derived(new Set(this.layers.map((l) => l.soundId)));

  /** What Nook Mode renders: the winning state for each scene channel. */
  scene = $derived.by(() => {
    const winners = new Map<string, { state: string; weight: number }>();
    for (const layer of this.layers) {
      const sound = this.byId.get(layer.soundId);
      if (!sound || sound.nook.channel === 'none') continue;
      const strength = sound.nook.weight * layer.volume;
      const current = winners.get(sound.nook.channel);
      if (!current || strength > current.weight) {
        winners.set(sound.nook.channel, { state: sound.nook.state, weight: strength });
      }
    }
    return Object.fromEntries([...winners].map(([k, v]) => [k, v.state]));
  });

  /* ── Lifecycle ───────────────────────────────────────────────────────── */

  async init() {
    this.sounds = await api.listSounds();
    document.documentElement.dataset.theme = this.theme;

    const status = await api.status();
    this.engineNote =
      status.sampleRate > 0
        ? `${status.device} · ${(status.sampleRate / 1000).toFixed(1)} kHz · ${status.channels} ch`
        : status.device;

    await api.setMasterVolume(this.masterVolume);

    if (this.restoreOnLaunch && saved.layers.length) {
      for (const l of saved.layers) {
        if (this.byId.has(l.soundId)) await this.add(l.soundId, l.volume);
      }
      // Restoring a mix should not start it playing over whatever the user was
      // already doing when they opened the app.
      this.playing = false;
      await api.setPlaying(false);
    }

    this.loading = false;
  }

  private persist() {
    save(STORAGE_KEY, {
      masterVolume: this.masterVolume,
      theme: this.theme,
      alwaysOnTop: this.alwaysOnTop,
      restoreOnLaunch: this.restoreOnLaunch,
      layers: this.layers.map((l) => ({ soundId: l.soundId, volume: l.volume })),
    } satisfies Persisted);
  }

  /* ── Toasts ──────────────────────────────────────────────────────────── */

  toast(text: string, tone: 'info' | 'error' = 'info') {
    const id = ++this.nextToast;
    this.toasts = [...this.toasts, { id, text, tone }];
    setTimeout(() => {
      this.toasts = this.toasts.filter((t) => t.id !== id);
    }, tone === 'error' ? 6000 : 3200);
  }

  /* ── Mixer ───────────────────────────────────────────────────────────── */

  async toggle(soundId: string) {
    if (this.activeIds.has(soundId)) await this.remove(soundId);
    else await this.add(soundId);
  }

  async add(soundId: string, volume = 0.55) {
    if (this.activeIds.has(soundId)) return;
    const sound = this.byId.get(soundId);
    if (!sound) return;

    // Show the layer immediately and let it settle once the file is verified.
    // The alternative — waiting for the hash check on a 30 MB file — makes the
    // tile feel unresponsive for no benefit.
    this.layers = [...this.layers, { soundId, volume, muted: false, loading: true }];
    if (!this.playing) {
      this.playing = true;
      await api.setPlaying(true);
    }

    try {
      await api.playSound(soundId, volume);
      this.layers = this.layers.map((l) => (l.soundId === soundId ? { ...l, loading: false } : l));
    } catch (e) {
      this.layers = this.layers.filter((l) => l.soundId !== soundId);
      this.toast(`${sound.name} could not be loaded — ${String(e)}`, 'error');
    }
    this.persist();
  }

  async remove(soundId: string) {
    this.layers = this.layers.filter((l) => l.soundId !== soundId);
    await api.stopSound(soundId);
    if (!this.layers.length) {
      this.playing = true;
      await api.setPlaying(true);
    }
    this.persist();
  }

  async setVolume(soundId: string, volume: number) {
    const v = Math.min(1, Math.max(0, volume));
    this.layers = this.layers.map((l) => (l.soundId === soundId ? { ...l, volume: v } : l));
    await api.setSoundVolume(soundId, v);
    this.persist();
  }

  async setMaster(volume: number) {
    this.masterVolume = Math.min(1, Math.max(0, volume));
    await api.setMasterVolume(this.masterVolume);
    this.persist();
  }

  async togglePlaying() {
    if (!this.layers.length) return;
    this.playing = !this.playing;
    await api.setPlaying(this.playing);
  }

  async clear() {
    this.layers = [];
    await api.stopAll();
    this.playing = true;
    await api.setPlaying(true);
    this.persist();
  }

  /* ── Presets ─────────────────────────────────────────────────────────── */

  savePreset(name: string) {
    const trimmed = name.trim();
    if (!trimmed || !this.layers.length) return;
    const preset: Preset = {
      id: crypto.randomUUID(),
      name: trimmed,
      createdAt: new Date().toISOString(),
      layers: this.layers.map((l) => ({ soundId: l.soundId, volume: l.volume })),
      masterVolume: this.masterVolume,
    };
    // Replacing by name rather than appending a duplicate is what people expect
    // from a "save" that reuses a name.
    this.presets = [preset, ...this.presets.filter((p) => p.name !== trimmed)];
    save(PRESET_KEY, { items: this.presets });
    this.toast(`Saved "${trimmed}"`);
  }

  deletePreset(id: string) {
    this.presets = this.presets.filter((p) => p.id !== id);
    save(PRESET_KEY, { items: this.presets });
  }

  async loadPreset(id: string) {
    const preset = this.presets.find((p) => p.id === id);
    if (!preset) return;
    await this.clear();
    if (preset.masterVolume !== null) await this.setMaster(preset.masterVolume);
    for (const l of preset.layers) await this.add(l.soundId, l.volume);
    this.toast(`Loaded "${preset.name}"`);
  }

  /* ── Timer ───────────────────────────────────────────────────────────── */

  private timerHandle: ReturnType<typeof setInterval> | null = null;

  startTimer(minutes: number) {
    this.stopTimer();
    const totalMs = minutes * 60_000;
    const endsAt = Date.now() + totalMs;
    this.timer = { kind: 'running', endsAt, totalMs };
    this.timerRemainingMs = totalMs;

    this.timerHandle = setInterval(() => {
      if (this.timer.kind !== 'running') return;
      const left = this.timer.endsAt - Date.now();
      this.timerRemainingMs = Math.max(0, left);
      if (left <= 0) {
        this.stopTimer();
        this.timer = { kind: 'finished' };
        this.playing = false;
        void api.setPlaying(false);
        this.toast('Timer finished — playback paused');
      }
    }, 250);
  }

  stopTimer() {
    if (this.timerHandle) clearInterval(this.timerHandle);
    this.timerHandle = null;
    if (this.timer.kind === 'running') this.timer = { kind: 'off' };
    this.timerRemainingMs = 0;
  }

  /* ── Settings ────────────────────────────────────────────────────────── */

  setTheme(theme: string) {
    this.theme = theme;
    document.documentElement.dataset.theme = theme;
    this.persist();
  }
}

export const nuru = new NuruStore();
