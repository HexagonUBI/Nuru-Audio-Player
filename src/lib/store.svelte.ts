

import { api, onProgress, BOOT_EVENT, UPDATE_EVENT, type Progress, type UpdateStatus, type ReleaseInfo } from './bridge';
import { randomAccent, randomLine } from './splash';
import { presenceFor } from './presence';
import type { Layer, Preset, Schedule, SoundEntry, TimerState } from './types';
import { SCHEDULE_SLOTS, SLOT_MINUTES } from './types';

const STORAGE_KEY = 'nuru.state.v1';
const PRESET_KEY = 'nuru.presets.v1';
const SCHEDULE_KEY = 'nuru.schedule.v1';

const SCHEDULE_TICK_MS = 15_000;

export function slotAt(date = new Date()): number {
  return date.getHours() * 2 + (date.getMinutes() >= 30 ? 1 : 0);
}

export function slotLabel(slot: number): string {
  const total = ((slot % SCHEDULE_SLOTS) + SCHEDULE_SLOTS) % SCHEDULE_SLOTS;
  const hour24 = Math.floor(total / 2);
  const minute = total % 2 === 0 ? '00' : '30';
  const suffix = hour24 < 12 ? 'am' : 'pm';
  const hour12 = hour24 % 12 === 0 ? 12 : hour24 % 12;
  return `${hour12}:${minute}${suffix}`;
}

function emptySchedule(): Schedule {
  return Array.from({ length: SCHEDULE_SLOTS }, () => null);
}

function loadSchedule(): Schedule {
  const base = emptySchedule();
  try {
    const raw = localStorage.getItem(SCHEDULE_KEY);
    if (!raw) return base;
    const parsed = JSON.parse(raw) as { slots?: unknown };
    if (!Array.isArray(parsed.slots)) return base;
    for (let i = 0; i < SCHEDULE_SLOTS; i++) {
      const v = parsed.slots[i];
      base[i] = typeof v === 'string' && v ? v : null;
    }
  } catch {
    return emptySchedule();
  }
  return base;
}

const MIN_BOOT_MS = 1100;

export const TILE_SIZES = {
  compact: 148,
  small: 180,
  normal: 216,
  large: 272,
} as const;

export type TileSize = keyof typeof TILE_SIZES;

const DEFAULT_SOUND_VOLUME = 0.55;

const TAG_PRIORITY = [
  'fire',
  'transit',
  'weather',
  'water',
  'urban',
  'nature',
  'life',
  'people',
  'indoor',
  'noise',
];

const TAG_ORDER = [
  'nature',
  'weather',
  'water',
  'fire',
  'life',
  'urban',
  'transit',
  'indoor',
  'people',
  'noise',
];

const OTHER = 'other';

function categoryOf(tags: string[]): string {
  for (const tag of TAG_PRIORITY) if (tags.includes(tag)) return tag;
  return tags[0] ?? OTHER;
}

function categoryRank(tag: string): number {
  const i = TAG_ORDER.indexOf(tag);
  return i === -1 ? TAG_ORDER.length : i;
}

function bySortKey(a: SoundEntry, b: SoundEntry): number {
  const ra = categoryRank(categoryOf(a.tags));
  const rb = categoryRank(categoryOf(b.tags));
  return ra === rb ? a.name.localeCompare(b.name) : ra - rb;
}

interface Persisted {
  masterVolume: number;
  theme: string;
  layers: Array<{ soundId: string; volume: number }>;
  alwaysOnTop: boolean;
  restoreOnLaunch: boolean;
  tileSize: TileSize;
  soundVolumes: Record<string, number>;
  sceneFollowsVolume: boolean;
  scheduleEnabled: boolean;
  collapsedGroups: string[];
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

  }
}

const saved = load<Persisted>(STORAGE_KEY, {
  masterVolume: 0.6,
  theme: 'midnight',
  layers: [],
  alwaysOnTop: false,
  restoreOnLaunch: true,
  tileSize: 'normal',
  soundVolumes: {},
  sceneFollowsVolume: true,
  scheduleEnabled: false,
  collapsedGroups: [],
});

class NuruStore {

  sounds = $state<SoundEntry[]>([]);

  layers = $state<Layer[]>([]);
  masterVolume = $state(saved.masterVolume);
  playing = $state(true);
  theme = $state(saved.theme);
  alwaysOnTop = $state(saved.alwaysOnTop);
  restoreOnLaunch = $state(saved.restoreOnLaunch);
  tileSize = $state<TileSize>(saved.tileSize ?? 'normal');
  soundVolumes = $state<Record<string, number>>(saved.soundVolumes ?? {});
  sceneFollowsVolume = $state(saved.sceneFollowsVolume ?? true);
  collapsedGroups = $state<string[]>(saved.collapsedGroups ?? []);

  outputDevices = $state<string[]>([]);
  outputDevice = $state<string | null>(null);
  activeDevice = $state('');

  boot = $state({
    visible: true,
    mode: 'startup' as 'startup' | 'update',
    phase: '',
    label: '',
    progress: 0,
    line: randomLine(),
    accent: randomAccent(),
    error: null as string | null,
  });

  presets = $state<Preset[]>(load<{ items: Preset[] }>(PRESET_KEY, { items: [] }).items ?? []);

  timer = $state<TimerState>({ kind: 'off' });
  timerRemainingMs = $state(0);

  schedule = $state<Schedule>(loadSchedule());
  scheduleEnabled = $state(saved.scheduleEnabled ?? false);
  scheduleSlot = $state(slotAt());

  filter = $state<string>('');
  search = $state('');

  nookMode = $state(false);
  activePanel = $state<'none' | 'presets' | 'timer' | 'settings' | 'credits'>('none');

  toasts = $state<Array<{ id: number; text: string; tone: 'info' | 'error' }>>([]);
  private nextToast = 0;

  loading = $state(true);
  engineNote = $state<string | null>(null);

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

  presetById = $derived(new Map(this.presets.map((p) => [p.id, p])));

  scheduledPresets = $derived(
    this.presets.filter((p) => this.schedule.includes(p.id)),
  );

  scheduleBlock = $derived.by(() => {
    const slot = this.scheduleSlot;
    const id = this.schedule[slot] ?? null;
    let start = slot;
    while (start > 0 && (this.schedule[start - 1] ?? null) === id) start -= 1;
    let end = slot;
    while (end < SCHEDULE_SLOTS - 1 && (this.schedule[end + 1] ?? null) === id) end += 1;
    return {
      id,
      preset: id ? (this.presetById.get(id) ?? null) : null,
      startsAt: slotLabel(start),
      endsAt: slotLabel(end + 1),
    };
  });

  scheduleNext = $derived.by(() => {
    const here = this.schedule[this.scheduleSlot] ?? null;
    for (let step = 1; step <= SCHEDULE_SLOTS; step++) {
      const slot = (this.scheduleSlot + step) % SCHEDULE_SLOTS;
      const id = this.schedule[slot] ?? null;
      if (id === here) continue;
      return {
        at: slotLabel(slot),
        preset: id ? (this.presetById.get(id) ?? null) : null,
        stops: id === null,
      };
    }
    return null;
  });

  groups = $derived.by(() => {
    const byName = new Map<string, { builtin: boolean; sounds: SoundEntry[] }>();
    for (const s of this.visibleSounds) {
      const name = s.packName || 'Other';
      const found = byName.get(name);
      if (found) found.sounds.push(s);
      else byName.set(name, { builtin: s.builtin, sounds: [s] });
    }
    return [...byName.entries()]
      .sort((a, b) => {
        if (a[1].builtin !== b[1].builtin) return a[1].builtin ? -1 : 1;
        return a[0].localeCompare(b[0]);
      })
      .map(([name, g]) => ({ name, sounds: [...g.sounds].sort(bySortKey) }));
  });

  scene = $derived.by(() => {
    let rain = 0;
    let snow = 0;
    let wind = 0;
    let storm = 0;
    let hearth = 0;
    let life = 0;
    let transit = 0;

    const pick = new Map<string, { state: string; weight: number }>();

    for (const layer of this.layers) {
      const sound = this.byId.get(layer.soundId);
      if (!sound || sound.nook.channel === 'none') continue;
      const strength = this.sceneFollowsVolume
        ? sound.nook.weight * layer.volume
        : sound.nook.weight;
      const { channel, state } = sound.nook;

      if (channel === 'weather') {
        if (state === 'rain') rain += strength;
        else if (state === 'downpour') rain += strength * 1.7;
        else if (state === 'storm') {
          storm += strength;
          rain += strength * 0.8;
        } else if (state === 'snow') snow += strength;
        else if (state === 'wind') wind += strength;
        continue;
      }

      if (channel === 'hearth') hearth = Math.max(hearth, strength);
      else if (channel === 'life') life = Math.max(life, strength);
      else if (channel === 'transit') transit = Math.max(transit, strength);

      const current = pick.get(channel);
      if (!current || strength > current.weight) pick.set(channel, { state, weight: strength });
    }

    const clamp = (n: number) => Math.min(1.6, n);

    return {
      rain: clamp(rain),
      snow: clamp(snow),
      wind: clamp(wind),
      storm: clamp(storm),
      hearth: clamp(hearth),
      life: clamp(life),
      transit: clamp(transit),
      sky: pick.get('sky')?.state ?? 'dusk',
      window: pick.get('window')?.state ?? 'hills',
      hearthKind: pick.get('hearth')?.state ?? null,
    };
  });

  async init() {
    document.documentElement.dataset.theme = this.theme;
    document.documentElement.dataset.tiles = this.tileSize;

    const startedAt = Date.now();

    let markDone!: () => void;
    const finished = new Promise<void>((resolve) => (markDone = resolve));

    const unlisten = await onProgress(BOOT_EVENT, (p: Progress) => {
      this.boot = {
        ...this.boot,
        phase: p.phase,
        label: p.label,
        progress: Math.max(this.boot.progress, p.progress),
        error: p.error,
      };
      if (p.done) markDone();
    });

    await api.boot();

    const guard = setTimeout(markDone, 20000);
    await finished;
    clearTimeout(guard);
    unlisten();

    this.sounds = await api.listSounds();

    const status = await api.status();
    this.engineNote =
      status.sampleRate > 0
        ? `${status.device} - ${(status.sampleRate / 1000).toFixed(1)} kHz - ${status.channels} ch`
        : status.device;

    const devices = await api.listOutputDevices();
    this.outputDevices = devices.devices;
    this.outputDevice = devices.saved;
    this.activeDevice = devices.active;

    await api.setMasterVolume(this.masterVolume);

    if (this.restoreOnLaunch && saved.layers.length) {
      for (const l of saved.layers) {
        if (this.byId.has(l.soundId)) await this.add(l.soundId, l.volume);
      }
      this.playing = false;
      await api.setPlaying(false);
    }

    this.loading = false;

    const held = Date.now() - startedAt;
    if (held < MIN_BOOT_MS) {
      await new Promise((r) => setTimeout(r, MIN_BOOT_MS - held));
    }
    this.boot = { ...this.boot, progress: 1, visible: false, mode: 'startup' };

    this.watchSchedule();

    await this.loadAutoUpdate();
    void this.showChangelogIfNew().catch(() => {});

    void this.checkUpdate()
      .then((status) => {
        if (status.available && this.autoUpdate) void this.installUpdate();
      })
      .catch(() => {});
  }

  private presenceStartedAt: number | null = null;
  private presenceTimer: ReturnType<typeof setTimeout> | null = null;

  private pushPresence() {
    if (this.presenceTimer) clearTimeout(this.presenceTimer);
    this.presenceTimer = setTimeout(() => {
      const playing = this.playing && this.layers.length > 0;
      if (playing && this.presenceStartedAt === null) {
        this.presenceStartedAt = Math.floor(Date.now() / 1000);
      } else if (!playing) {
        this.presenceStartedAt = null;
      }
      const { details, status } = presenceFor(this.scene, this.layers, this.byId, this.nookMode);
      void api.setPresence(details, status, playing, this.presenceStartedAt);
    }, 1200);
  }

  update = $state<UpdateStatus | null>(null);
  updateBusy = $state(false);
  autoUpdate = $state(true);
  changelog = $state<ReleaseInfo | null>(null);
  changelogBusy = $state(false);

  async loadAutoUpdate() {
    this.autoUpdate = await api.getAutoUpdate();
  }

  async setAutoUpdate(enabled: boolean) {
    this.autoUpdate = enabled;
    await api.setAutoUpdate(enabled);
  }

  dismissChangelog() {
    this.changelog = null;
    void api.markChangelogSeen();
  }

  private async showChangelogIfNew() {
    const release = await api.pendingChangelog();
    if (release) this.changelog = release;
  }

  async openChangelog() {
    if (this.changelogBusy) return;
    this.changelogBusy = true;
    try {
      const release = await api.latestChangelog();
      if (release) {
        this.activePanel = 'none';
        this.changelog = release;
      } else {
        this.toast('No release notes to show yet');
      }
    } catch {
      this.toast('Could not load the release notes', 'error');
    } finally {
      this.changelogBusy = false;
    }
  }

  async checkUpdate(quiet = true) {
    const status = await api.checkUpdate();
    this.update = status;
    if (!quiet) {
      if (status.available) this.toast(`Nuru ${status.available.version} is available`);
      else if (status.errorMessage) this.toast(status.errorMessage, 'error');
      else this.toast('Nuru is up to date');
    }
    return status;
  }

  async installUpdate() {
    if (this.updateBusy) return;
    this.updateBusy = true;

    this.boot = {
      ...this.boot,
      visible: true,
      mode: 'update',
      phase: 'checking',
      label: 'Preparing',
      progress: 0,
      line: randomLine(),
      accent: randomAccent(),
      error: null,
    };

    const unlisten = await onProgress(UPDATE_EVENT, (p: Progress) => {
      this.boot = {
        ...this.boot,
        phase: p.phase,
        label: p.label,
        progress: Math.max(this.boot.progress, p.progress),
        error: p.error,
      };
      if (p.error) {
        this.updateBusy = false;
        setTimeout(() => {
          this.boot = { ...this.boot, visible: false };
        }, 3200);
      }
    });

    await api.installUpdate();
    void unlisten;
  }

  async reloadSounds() {
    this.sounds = await api.listSounds();
  }

  setSceneFollowsVolume(on: boolean) {
    this.sceneFollowsVolume = on;
    this.persist();
  }

  setRestoreOnLaunch(on: boolean) {
    this.restoreOnLaunch = on;
    this.persist();
  }

  isCollapsed(group: string): boolean {
    return this.collapsedGroups.includes(group);
  }

  toggleGroup(group: string) {
    this.collapsedGroups = this.isCollapsed(group)
      ? this.collapsedGroups.filter((g) => g !== group)
      : [...this.collapsedGroups, group];
    this.persist();
  }

  setTileSize(size: TileSize) {
    this.tileSize = size;
    document.documentElement.dataset.tiles = size;
    this.persist();
  }

  async setOutputDevice(device: string | null) {
    const layers = this.layers.map((l) => ({ soundId: l.soundId, volume: l.volume }));
    const wasPlaying = this.playing;
    try {
      this.activeDevice = await api.setOutputDevice(device);
      this.outputDevice = device;

      this.layers = [];
      await api.setMasterVolume(this.masterVolume);
      for (const l of layers) await this.add(l.soundId, l.volume);
      if (!wasPlaying) {
        this.playing = false;
        await api.setPlaying(false);
      }
      this.toast(`Output set to ${this.activeDevice}`);
    } catch (e) {
      this.toast(`Could not switch output: ${String(e)}`, 'error');
    }
  }

  private persist() {
    save(STORAGE_KEY, {
      masterVolume: this.masterVolume,
      theme: this.theme,
      alwaysOnTop: this.alwaysOnTop,
      restoreOnLaunch: this.restoreOnLaunch,
      tileSize: this.tileSize,
      soundVolumes: this.soundVolumes,
      sceneFollowsVolume: this.sceneFollowsVolume,
      scheduleEnabled: this.scheduleEnabled,
      collapsedGroups: this.collapsedGroups,
      layers: this.layers.map((l) => ({ soundId: l.soundId, volume: l.volume })),
    } satisfies Persisted);
  }

  toast(text: string, tone: 'info' | 'error' = 'info') {
    const id = ++this.nextToast;
    this.toasts = [...this.toasts, { id, text, tone }];
    setTimeout(() => {
      this.toasts = this.toasts.filter((t) => t.id !== id);
    }, tone === 'error' ? 6000 : 3200);
  }

  async toggle(soundId: string) {
    if (this.activeIds.has(soundId)) await this.remove(soundId);
    else await this.add(soundId);
  }

  async add(soundId: string, volume = this.soundVolumes[soundId] ?? DEFAULT_SOUND_VOLUME) {
    if (this.activeIds.has(soundId)) return;
    const sound = this.byId.get(soundId);
    if (!sound) return;

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
      this.toast(`${sound.name} could not be loaded - ${String(e)}`, 'error');
    }
    this.persist();
    this.pushPresence();
  }

  async remove(soundId: string) {
    this.layers = this.layers.filter((l) => l.soundId !== soundId);
    await api.stopSound(soundId);
    if (!this.layers.length) {
      this.playing = true;
      await api.setPlaying(true);
    }
    this.persist();
    this.pushPresence();
  }

  async setVolume(soundId: string, volume: number) {
    const v = Math.min(1, Math.max(0, volume));
    this.layers = this.layers.map((l) => (l.soundId === soundId ? { ...l, volume: v } : l));
    this.soundVolumes = { ...this.soundVolumes, [soundId]: v };
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
    this.pushPresence();
  }

  async clear() {
    this.layers = [];
    await api.stopAll();
    this.playing = true;
    await api.setPlaying(true);
    this.persist();
    this.pushPresence();
  }

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
    this.presets = [preset, ...this.presets.filter((p) => p.name !== trimmed)];
    save(PRESET_KEY, { items: this.presets });
    this.toast(`Saved "${trimmed}"`);
  }

  deletePreset(id: string) {
    this.presets = this.presets.filter((p) => p.id !== id);
    save(PRESET_KEY, { items: this.presets });
    this.clearSlotsFor(id);
  }

  async loadPreset(id: string, note?: string) {
    const preset = this.presets.find((p) => p.id === id);
    if (!preset) return;
    await this.clear();
    if (preset.masterVolume !== null) await this.setMaster(preset.masterVolume);
    for (const l of preset.layers) await this.add(l.soundId, l.volume);
    this.toast(note ?? `Loaded "${preset.name}"`);
  }

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
        this.toast('Timer finished - playback paused');
      }
    }, 250);
  }

  stopTimer() {
    if (this.timerHandle) clearInterval(this.timerHandle);
    this.timerHandle = null;
    if (this.timer.kind === 'running') this.timer = { kind: 'off' };
    this.timerRemainingMs = 0;
  }

  private scheduleHandle: ReturnType<typeof setInterval> | null = null;
  private scheduleSeen = -1;
  private scheduleApplied: string | null = null;

  watchSchedule() {
    if (this.scheduleHandle) return;
    this.scheduleHandle = setInterval(() => void this.tickSchedule(), SCHEDULE_TICK_MS);
    void this.tickSchedule();
  }

  private async tickSchedule() {
    const slot = slotAt();
    this.scheduleSlot = slot;

    if (!this.scheduleEnabled) {
      this.scheduleSeen = -1;
      this.scheduleApplied = null;
      return;
    }

    if (slot === this.scheduleSeen) return;
    this.scheduleSeen = slot;

    const wanted = this.schedule[slot] ?? null;
    if (wanted === this.scheduleApplied) return;
    this.scheduleApplied = wanted;

    if (!wanted) {
      if (this.playing && this.layers.length) {
        this.playing = false;
        await api.setPlaying(false);
        this.pushPresence();
        this.toast(`Schedule: nothing set for ${slotLabel(slot)}, paused`);
      }
      return;
    }

    const preset = this.presetById.get(wanted);
    if (!preset) {
      this.clearSlotsFor(wanted);
      return;
    }
    await this.loadPreset(wanted, `Schedule: ${preset.name} until ${this.scheduleBlock.endsAt}`);
  }

  setScheduleEnabled(on: boolean) {
    this.scheduleEnabled = on;
    this.scheduleSeen = -1;
    this.scheduleApplied = null;
    this.persist();
    if (on) void this.tickSchedule();
  }

  setSlot(slot: number, presetId: string | null) {
    if (slot < 0 || slot >= SCHEDULE_SLOTS) return;
    const next = [...this.schedule];
    next[slot] = presetId;
    this.schedule = next;
    this.saveSchedule();
    this.scheduleSeen = -1;
    if (this.scheduleEnabled) void this.tickSchedule();
  }

  clearSchedule() {
    this.schedule = emptySchedule();
    this.saveSchedule();
    this.scheduleSeen = -1;
    this.scheduleApplied = null;
  }

  private clearSlotsFor(presetId: string) {
    if (!this.schedule.includes(presetId)) return;
    this.schedule = this.schedule.map((s) => (s === presetId ? null : s));
    this.saveSchedule();
  }

  private saveSchedule() {
    save(SCHEDULE_KEY, { slots: this.schedule });
  }

  scheduledMinutes(presetId: string): number {
    return this.schedule.filter((s) => s === presetId).length * SLOT_MINUTES;
  }

  setTheme(theme: string) {
    this.theme = theme;
    document.documentElement.dataset.theme = theme;
    this.persist();
  }
}

export const nuru = new NuruStore();
