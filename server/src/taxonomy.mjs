export const LIMITS = {
  soundName: 48,
  soundDescription: 600,
  mixtapeTitle: 48,
  mixtapeDescription: 600,
  attribution: 160,
  bio: 400,
  displayName: 32,
  reportNote: 500,
  tagRequestReason: 200,
  tagName: 24,
  tagsPerSound: 8,
  soundsPerMixtape: 24,
};

export const OFFICIAL_TAGS = {
  weather: ['rain', 'heavy rain', 'drizzle', 'storm', 'thunder', 'wind', 'snow', 'hail', 'fog'],
  water: ['ocean', 'waves', 'river', 'stream', 'waterfall', 'lake', 'underwater', 'harbour', 'dripping'],
  nature: ['forest', 'jungle', 'desert', 'meadow', 'mountains', 'cave', 'swamp', 'beach', 'garden'],
  life: ['birds', 'seagulls', 'crickets', 'frogs', 'insects', 'owls', 'wolves', 'livestock', 'dogs', 'cats'],
  urban: ['city', 'traffic', 'street', 'construction', 'market', 'crowd', 'playground', 'sirens'],
  indoor: ['cafe', 'library', 'office', 'home', 'kitchen', 'laundromat', 'hallway', 'attic', 'basement'],
  transit: ['train', 'subway', 'bus', 'car', 'plane', 'boat', 'station', 'airport'],
  fire: ['campfire', 'fireplace', 'candle', 'stove'],
  noise: ['white noise', 'pink noise', 'brown noise', 'fan', 'air conditioning', 'hum', 'static'],
  time: ['dawn', 'morning', 'day', 'dusk', 'night'],
  mood: ['calm', 'cosy', 'melancholy', 'tense', 'focus', 'sleep', 'busy'],
  human: ['chatter', 'footsteps', 'typing', 'writing', 'cooking', 'pages'],
};

export const REPORT_REASONS = [
  { id: 'not-ambience', label: 'Not a soundscape', note: 'A clip, a one shot, music, or speech.' },
  { id: 'licence', label: 'Licence looks wrong', note: 'Probably not theirs to give away.' },
  { id: 'quality', label: 'Broken audio', note: 'Clipping, buffeting, cut off, or silent.' },
  { id: 'mislabelled', label: 'Wrong name or tags', note: 'Does not match what you actually hear.' },
  { id: 'duplicate', label: 'Duplicate', note: 'Already on the workshop under another name.' },
  { id: 'other', label: 'Something else', note: 'Tell the moderators what is wrong.' },
];

export function clip(value, max) {
  return String(value ?? '').slice(0, max);
}

export function tooLong(value, max) {
  return String(value ?? '').length > max;
}
