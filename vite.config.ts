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

/**
 * Serves the development sound pack at /devpack/*.
 *
 * Only matters when Nuru is opened in a plain browser rather than through
 * Tauri: it lets the preview build show the real catalogue, names and cover art
 * instead of invented ones. The pack lives outside the Vite root, so it needs
 * its own middleware rather than publicDir.
 *
 * Keep this path in step with the pack directory — if it drifts, the Tauri app
 * keeps working and only the browser preview silently shows an empty grid.
 */
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

// Tauri drives the dev server; these settings keep HMR working inside WebView2
// and stop Vite from hiding Rust compiler errors behind its own output.
export default defineConfig({
  plugins: [svelte(), devPack()],
  resolve: {
    // Mirrors the `paths` entry in tsconfig.json — tsconfig only teaches the
    // type checker, the bundler needs telling separately.
    alias: { $lib: join(process.cwd(), 'src', 'lib') },
  },
  clearScreen: false,
  server: {
    port: 5183,
    strictPort: true,
    host: '127.0.0.1',
    watch: {
      // src-tauri is Rust's business, not Vite's.
      ignored: ['**/src-tauri/**', '**/dev-notes/**'],
    },
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    // WebView2 on Windows 10 is evergreen Chromium; no need to down-level.
    target: 'chrome110',
    minify: 'esbuild',
    sourcemap: false,
    outDir: 'dist',
    emptyOutDir: true,
  },
});
