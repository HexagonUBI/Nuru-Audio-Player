import type { SoundEntry, Layer } from './types';

interface Scene {
  rain: number;
  snow: number;
  wind: number;
  storm: number;
  hearth: number;
  life: number;
  transit: number;
  sky: string;
  window: string;
}

type Mood =
  | 'storm'
  | 'downpour'
  | 'rain'
  | 'snow'
  | 'hearth'
  | 'cafe'
  | 'city'
  | 'forest'
  | 'beach'
  | 'night'
  | 'morning'
  | 'noise'
  | 'quiet'
  | 'mixed';

const LINES: Record<Mood, string[]> = {
  storm: [
    'Counting seconds between the flashes',
    'Letting the storm do the talking',
    'Somewhere dry, thankfully',
    'The sky is in a mood',
  ],
  downpour: [
    'It is really coming down',
    'Not going outside today',
    'Watching the gutters give up',
    'Rain, with commitment',
  ],
  rain: [
    'Rain on the window, tea in hand',
    'Listening to the roof',
    'Grey outside, warm inside',
    'The good kind of weather',
  ],
  snow: [
    'Everything is quieter with snow',
    'Watching it settle',
    'Snow is falling, nobody is coming',
    'The world got a blanket',
  ],
  hearth: [
    'Closest seat to the fire',
    'Feeding the fire',
    'Warm side facing in',
    'Nothing beats a fire',
  ],
  cafe: [
    'Eavesdropping politely',
    'In the corner with a notebook',
    'Third refill, no regrets',
    'Somebody else can make the coffee',
  ],
  city: [
    'Up above the traffic',
    'City lights, window shut',
    'Eight million people, none of them here',
    'The city hums itself to sleep',
  ],
  forest: [
    'Somewhere with more trees than people',
    'Out where the signal ends',
    'The forest is doing fine without us',
    'Deep in the green',
  ],
  beach: [
    'Toes in the imaginary sand',
    'Tide coming in',
    'The sea repeating itself, kindly',
    'Salt in the air, allegedly',
  ],
  night: [
    'Long past bedtime',
    'On the night shift',
    'Crickets have the floor',
    'The quiet hours',
  ],
  morning: [
    'Birds are up first, as usual',
    'Easing into the day',
    'Early, on purpose',
    'The morning shift',
  ],
  noise: [
    'Tuning the world out',
    'Static and nothing else',
    'Building a wall of nothing',
    'Focus mode, engaged',
  ],
  quiet: ['Picking something to listen to', 'Deciding on a mood', 'Browsing the weather'],
  mixed: [
    'Building a little weather',
    'Somewhere cosy',
    'Assembling an atmosphere',
    'Making a place to be',
  ],
};

function moodOf(scene: Scene, sounds: SoundEntry[]): Mood {
  if (!sounds.length) return 'quiet';

  if (scene.storm > 0.25) return 'storm';
  if (scene.rain > 0.9) return 'downpour';
  if (scene.snow > 0.3 && scene.snow >= scene.rain) return 'snow';
  if (scene.rain > 0.2) return 'rain';
  if (scene.hearth > 0.3) return 'hearth';

  if (scene.window === 'city') return 'city';
  if (scene.window === 'forest') return 'forest';
  if (scene.window === 'beach') return 'beach';
  if (scene.life > 0.3) return 'cafe';

  const allNoise = sounds.every((s) => s.tags.includes('noise'));
  if (allNoise) return 'noise';

  if (scene.sky === 'night') return 'night';
  if (scene.sky === 'morning') return 'morning';
  return 'mixed';
}

const chosen = new Map<Mood, string>();

export function presenceFor(
  scene: Scene,
  layers: Layer[],
  byId: Map<string, SoundEntry>,
  cozy: boolean,
): { details: string; status: string } {
  const sounds = layers
    .map((l) => byId.get(l.soundId))
    .filter((s): s is SoundEntry => Boolean(s));

  const mood = moodOf(scene, sounds);
  if (!chosen.has(mood)) {
    const pool = LINES[mood];
    chosen.set(mood, pool[Math.floor(Math.random() * pool.length)]);
  }

  const n = sounds.length;
  const count = n === 0 ? 'Nothing playing' : n === 1 ? '1 soundscape' : `${n} soundscapes`;

  return {
    details: cozy ? `${count}, curled up` : count,
    status: chosen.get(mood) as string,
  };
}
