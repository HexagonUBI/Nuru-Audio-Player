import { Resvg } from '@resvg/resvg-js';
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const SOURCE = join(ROOT, 'resources', 'logo.svg');
const ICONS = join(ROOT, 'src-tauri', 'icons');

const PAD = 0.0;
const ICO_SIZES = [16, 24, 32, 48, 64, 128, 256];

const PNGS = [
  ['32x32.png', 32],
  ['64x64.png', 64],
  ['128x128.png', 128],
  ['128x128@2x.png', 256],
  ['icon.png', 512],
  ['source.png', 1024],
];

function square(svg) {
  const head = svg.match(/<svg[^>]*>/)[0];
  const viewBox = head.match(/viewBox="([^"]+)"/);
  const box = viewBox ? viewBox[1] : `0 0 ${head.match(/width="(\d+)"/)[1]} ${head.match(/height="(\d+)"/)[1]}`;
  const body = svg.slice(svg.indexOf('>', svg.indexOf('<svg')) + 1, svg.lastIndexOf('</svg>'));
  const inset = (PAD * 100).toFixed(2);
  const span = (100 - PAD * 200).toFixed(2);
  return [
    '<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 1024 1024">',
    `<svg x="${inset}%" y="${inset}%" width="${span}%" height="${span}%"`,
    ` viewBox="${box}" preserveAspectRatio="xMidYMid meet">`,
    body,
    '</svg></svg>',
  ].join('');
}

function render(svg, size) {
  return new Resvg(svg, { fitTo: { mode: 'width', value: size }, background: 'rgba(0,0,0,0)' })
    .render()
    .asPng();
}

function ico(entries) {
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(entries.length, 4);

  const dir = Buffer.alloc(16 * entries.length);
  let offset = header.length + dir.length;

  entries.forEach(({ size, png }, i) => {
    const at = i * 16;
    dir.writeUInt8(size >= 256 ? 0 : size, at);
    dir.writeUInt8(size >= 256 ? 0 : size, at + 1);
    dir.writeUInt8(0, at + 2);
    dir.writeUInt8(0, at + 3);
    dir.writeUInt16LE(1, at + 4);
    dir.writeUInt16LE(32, at + 6);
    dir.writeUInt32LE(png.length, at + 8);
    dir.writeUInt32LE(offset, at + 12);
    offset += png.length;
  });

  return Buffer.concat([header, dir, ...entries.map((e) => e.png)]);
}

const source = readFileSync(SOURCE, 'utf8');
const canvas = square(source);

const favicon = join(ROOT, 'docs', 'assets', 'favicon.svg');
writeFileSync(favicon, canvas);
console.log(`favicon.svg      square, padded  ${canvas.length} bytes`);

for (const [name, size] of PNGS) {
  const png = render(canvas, size);
  writeFileSync(join(ICONS, name), png);
  console.log(`${name.padEnd(16)} ${size}x${size}  ${png.length} bytes`);
}

const entries = ICO_SIZES.map((size) => ({ size, png: render(canvas, size) }));
const bundle = ico(entries);
writeFileSync(join(ICONS, 'icon.ico'), bundle);
console.log(`icon.ico         ${ICO_SIZES.join(', ')}  ${bundle.length} bytes`);
