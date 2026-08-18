import { defineConfig, type Plugin } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { createReadStream, existsSync, statSync } from 'node:fs';
import { extname, join, normalize } from 'node:path';

const PACK_DIR = join(process.cwd(), 'resources', 'packs', 'elpy-placeholder');
const MIME: Record<string, string> = {
  '.json': 'application/json',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.webp': 'image/webp',
  '.flac': 'audio/flac',
};

function devPack(): Plugin {
  return {
    name: 'nuru-dev-pack',
    apply: 'serve',
    configureServer(server) {
      server.middlewares.use('/devpack', (req, res, next) => {
        const rel = decodeURIComponent((req.url ?? '/').split('?')[0]);
        const file = normalize(join(PACK_DIR, rel));
        if (!file.startsWith(PACK_DIR) || !existsSync(file) || !statSync(file).isFile()) {
          return next();
        }
        res.setHeader('Content-Type', MIME[extname(file)] ?? 'application/octet-stream');
        res.setHeader('Cache-Control', 'no-cache');
        createReadStream(file).pipe(res);
      });
    },
  };
}

export default defineConfig({
  plugins: [svelte(), devPack()],
  resolve: {
    alias: { $lib: join(process.cwd(), 'src', 'lib') },
  },
  clearScreen: false,
  server: {
    port: 5183,
    strictPort: true,
    host: '127.0.0.1',
    watch: {
      ignored: ['**/src-tauri/**', '**/dev-notes/**'],
    },
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    target: 'chrome110',
    minify: 'esbuild',
    sourcemap: false,
    outDir: 'dist',
    emptyOutDir: true,
  },
});
