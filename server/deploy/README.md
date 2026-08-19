# Hosting the Workshop

The Workshop is a long lived Node process. It owns a SQLite file and an uploads
directory that both have to survive restarts, and it runs `ffmpeg` and `ffprobe`
as child processes when someone uploads.

It needs **node 24 or newer**, because the whole service is built on
`node:sqlite` and there are no other dependencies. Node 18 and 20 do not have
that module at all. Node 22 and 23.0 to 23.3 have it behind
`--experimental-sqlite`. `server/index.js` checks this at startup and says which
of those cases you are in rather than failing with a module resolution error.

## Hostinger, the shared plan

Uploading the files into `public_html/paradise/` does not work and cannot be
made to work by rearranging them. LiteSpeed serves files. `/api/sounds`,
`/audio/...` and `/cover/...` are not files, they are answers a running program
computes. There is nothing there to run it.

What that produces, measured:

```
curl -sI https://paradise.simplefox.studio/   -> 403, no index file at the doc root
curl -s  .../api/stats                        -> the parent domain's static 404 page
```

**Never put `server/data` on a static host.** It was briefly world readable at
`https://paradise.simplefox.studio/data/nuru.db`, which exposes every account's
email, password hash and salt, and the live session tokens. `npm run db:pack`
builds an upload artifact that physically cannot contain it.

## Hostinger, Node.js apps, the one to use

Hostinger runs managed Node apps on **Business** and on the **Cloud** plans, from
hPanel. The app lives in `/home/<user>/domains/<domain>/nodejs`, outside
`public_html`, and hPanel writes the `.htaccess` that routes the subdomain to it.
Outside the web root is exactly where `server/data` belongs.

If your plan has it, the subdomain works with no VPS and no DNS change.

1. hPanel, Websites, `paradise.simplefox.studio`, look for **Node.js** in the
   sidebar. If it is not there, the plan does not include it: either upgrade to
   Business, or use the VPS path below.
2. Build the upload artifact:

   ```
   npm run db:pack
   ```

   That writes `release/nuru-workshop.zip`, about 140 kB, containing `src`,
   `public`, `index.js`, `package.json` and the admin scripts. It refuses to run
   if `data` or `deploy` ended up staged.

3. Create the app. Node version **24**. Entry file **`index.js`**. Upload the zip.
4. Set the environment variables in the Node.js panel:

   ```
   NURU_PUBLIC_URL     https://paradise.simplefox.studio
   NURU_MODERATOR_EMAIL your@address
   NURU_TRUST_PROXY    1
   NURU_DATA_DIR       /home/<user>/domains/paradise.simplefox.studio/workshop-data
   ```

   `NURU_DATA_DIR` is optional. The default is `data` next to the app, which is
   already outside `public_html` here. Setting it to a directory outside the
   deployed tree means a redeploy cannot wipe the database.

   Do not set `NURU_DB_PORT`. The platform injects `PORT` and the server reads it.

5. Start it, then check:

   ```
   curl -s https://paradise.simplefox.studio/healthz
   ```

   Expect `{"ok":true,"ffmpeg":...}`. If `ffmpeg` is `false`, browse, playback
   and install all still work and uploads answer 503 with an "uploads offline"
   chip in the UI. To turn uploads on, upload a static ffmpeg build into the app
   directory and point `NURU_FFMPEG` and `NURU_FFPROBE` at the two binaries.
   Whether the platform lets a child process run is the thing to test; if it
   does not, the Workshop is read only and you publish by uploading from a local
   instance and copying `data` up.

6. Seed the tag vocabulary only, then register your moderator address as the
   first account:

   ```
   npm run db:tags
   ```

   `npm run db:seed` is a development fixture that writes throwaway logins. Never
   run it against the public instance.

## A VPS

If the plan has no Node.js entry, a VPS is sold separately. The WordPress site
stays on the shared plan and only the `paradise` A record moves.

```bash
apt update && apt install -y nodejs npm ffmpeg nginx certbot python3-certbot-nginx
adduser --system --group --home /srv/nuru nuru
```

Check `node -v` is 24 or newer; Debian and Ubuntu repositories often ship
something older, in which case use nodesource. Then:

```bash
cp server/deploy/nginx.conf /etc/nginx/sites-available/paradise
ln -s /etc/nginx/sites-available/paradise /etc/nginx/sites-enabled/paradise
certbot --nginx -d paradise.simplefox.studio
cp server/deploy/nuru-workshop.service /etc/systemd/system/
```

Edit `NURU_MODERATOR_EMAIL` in the unit file, then:

```bash
systemctl daemon-reload
systemctl enable --now nuru-workshop
curl -s https://paradise.simplefox.studio/healthz
```

This is the only option where ffmpeg is certainly available, so uploads
certainly work.

## A container host

Fly.io, Railway and Render all work. Requirements are a persistent volume
mounted at whatever `NURU_DATA_DIR` points to, ffmpeg in the image, node 24, and
`PORT` honoured, which the server does. Point a CNAME at the platform hostname.

## Environment

| Variable | Default | What it does |
| --- | --- | --- |
| `NURU_PUBLIC_URL` | empty | The canonical origin. Setting it is what switches the server out of development mode: secure cookies, HSTS, the origin check, and no first-signup moderator. |
| `NURU_DATA_DIR` | `data` next to the app | Where the database and uploads live. Absolute, or relative to the server directory. Put it outside the deployed tree so redeploys cannot wipe it. |
| `NURU_DB_HOST` | `127.0.0.1` | Bind address. Leave it on loopback behind a reverse proxy. |
| `NURU_DB_PORT` | `PORT`, then `5175` | Listen port. Managed platforms inject `PORT`; leave this unset there. |
| `NURU_TRUST_PROXY` | on when public | Read `X-Forwarded-For` for rate limiting. Only turn this on when something you control sets that header, or rate limits can be bypassed. |
| `NURU_MODERATOR_EMAIL` | empty | This address gets the moderator role when it registers, whenever that happens. Nobody else does. |
| `NURU_OPEN_SIGNUP` | `1` | Set to `0` to close registration. |
| `NURU_ALLOWED_ORIGINS` | empty | Extra origins accepted on writes, comma separated. |
| `NURU_FFMPEG` / `NURU_FFPROBE` | `ffmpeg` / `ffprobe` | Absolute paths, for hosts where the binaries are not on `PATH`. |

Without `NURU_PUBLIC_URL` the server keeps its development behaviour, so
`npm run db:dev` is unchanged.

## Fixing a role afterwards

```
npm run db:promote -- you@example.com moderator
```

## What is still missing

No email verification and no password reset, so a forgotten password means
editing the database. No virus scanning on uploads. Rate limiting is in memory,
so it resets when the process restarts and does not help against a distributed
attempt; it is there to stop one host guessing passwords.
