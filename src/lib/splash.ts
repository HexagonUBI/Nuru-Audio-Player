

export const SPLASH_COLOURS = [
  '#ffb454',
  '#5ae3ff',
  '#74dc74',
  '#f7896b',
  '#d5b5f6',
  '#62e6a7',
  '#f4bf71',
  '#779cff',
  '#f16b79',
  '#44facc',
  '#fcf44c',
  '#abb5d5',
] as const;

const LINES = [
  'Warming up the rain',
  'Tuning the crickets',
  'Stacking loops end to end',
  'Counting samples, twice',
  'Politely asking the wind to loop',
  'Folding the seams away',
  'Lighting the fire',
  'Measuring the quiet',
  'Rounding every corner',
  'Sixteen bits at a time',
  'Teaching thunder to wait its turn',
  'Sanding down the edges',
  'Convincing the cat to move',
  'Filling the kettle',
  'Finding a comfortable chair',
  'Letting the room settle',
  'Nothing streams, everything waits',
  'Dimming the lights',
  'Checking under the floorboards',
  'Arranging the weather',
  'Aligning the loop points',
  'Making the ocean agree with itself',
  'Brewing something warm',
  'Closing the curtains',
] as const;

function pick<T>(items: readonly T[]): T {
  return items[Math.floor(Math.random() * items.length)];
}

export function randomAccent(): string {
  return pick(SPLASH_COLOURS);
}

export function randomLine(): string {
  return pick(LINES);
}
