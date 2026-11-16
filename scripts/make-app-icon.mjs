import { Resvg } from '@resvg/resvg-js';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const SOURCE = join(ROOT, 'resources', 'logo.svg');
const ASSETS = join(ROOT, 'crates', 'nuru', 'ui', 'assets');
const SITE = process.env.NURU_SITE || join(ROOT, '..', '.website-repos', 'simplefox-studio', 'site');

const BACKGROUND = '#171717';
const COVER = 0.71;
const ICO_SIZES = [16, 20, 24, 32, 40, 48, 64, 128, 256];

function tile(svg) {
  const head = svg.match(/<svg[^>]*>/)[0];
  const box = head.match(/viewBox="([^"]+)"/)[1];
  const [, , w, h] = box.split(/\s+/).map(Number);
  const body = svg.slice(svg.indexOf('>', svg.indexOf('<svg')) + 1, svg.lastIndexOf('</svg>'));
  const s = (COVER * 1024) / w;
  const tx = (1024 - w * s) / 2;
  const ty = (1024 - h * s) / 2;
  return [
    '<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 1024 1024">',
    `<rect width="1024" height="1024" fill="${BACKGROUND}"/>`,
    `<g transform="translate(${tx} ${ty}) scale(${s})">${body}</g>`,
    '</svg>',
  ].join('');
}

function render(svg, size) {
  return new Resvg(svg, { fitTo: { mode: 'width', value: size } }).render().asPng();
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
    dir.writeUInt16LE(1, at + 4);
    dir.writeUInt16LE(32, at + 6);
    dir.writeUInt32LE(png.length, at + 8);
    dir.writeUInt32LE(offset, at + 12);
    offset += png.length;
  });
  return Buffer.concat([header, dir, ...entries.map((e) => e.png)]);
}

const art = tile(readFileSync(SOURCE, 'utf8'));
const bundle = ico(ICO_SIZES.map((size) => ({ size, png: render(art, size) })));
writeFileSync(join(ASSETS, 'icon.ico'), bundle);
console.log(`icon.ico  ${ICO_SIZES.join(', ')}  ${bundle.length} bytes`);

writeFileSync(join(ASSETS, 'icon.png'), render(art, 256));
console.log('icon.png  256');

const catalog = join(SITE, 'apps', 'nuru');
if (existsSync(catalog)) {
  writeFileSync(join(catalog, 'icon.png'), render(art, 256));
  console.log(`catalog   ${join(catalog, 'icon.png')}`);
}
