# server

The Nuru Workshop. A Node service with its own SQLite database, sessions and
uploads.

This is deliberately **not** in `docs/`. That folder is GitHub Pages, which serves
static files only, so it cannot hold a database, sessions or uploads.

It runs on your own machine for development, and on a host that can run Node for
the public instance at `https://paradise.simplefox.studio`. See
`deploy/README.md` for what a host has to provide and why Hostinger shared
hosting is not one.

## Running it

```
npm run db:seed     write server/data/nuru.db and fill it
npm run db:dev      serve http://127.0.0.1:5175
npm run db:reset    delete the database and seed again
```

Nothing to install. Node 26 ships `node:sqlite`, so the whole service has zero
dependencies and no native modules. `NURU_DB_PORT` overrides the port, and every
other setting is listed in `deploy/README.md`.

Uploads shell out to `ffmpeg` and `ffprobe`. Without them the server still starts
and browse, playback and install all work, but the upload endpoints answer 503
and the front end says so.

`server/data/` holds the database and uploaded audio and is gitignored.

Stop the server before `db:reset`. Windows keeps the SQLite file locked while the
process is alive and the delete fails with EBUSY.

## What is in it

Front end at `/`, with browse, upload, my uploads, moderation and account tabs.
The API:

```
POST /api/register            first account created becomes a moderator
POST /api/login               sets an httpOnly session cookie
POST /api/logout
GET  /api/me
GET  /api/tags                tag names with use counts
GET  /api/sounds              ?q= &tag= &status=   public unless moderator
GET  /api/sounds/:slug
POST /api/sounds              metadata, lands in review
PUT  /api/sounds/:slug/audio  raw body, 80 MB cap
GET  /api/me/sounds           own uploads with their status
GET  /api/mod/queue           moderators only
POST /api/mod/:slug/decision  accept or decline, with a note
GET  /api/stats
GET  /audio/:slug.flac
```

Upload is two steps on purpose. The metadata post returns a slug, then the audio
goes up as a raw body to that slug. That avoids parsing multipart by hand and
keeps the service dependency free.

Statuses are `review`, `public`, `private` and `declined`. Everything starts in
`review`, and only a moderator decision moves it to `public`, so nothing reaches
the browse list unreviewed.

`installUrl` on every sound is the `nuru://install/<slug>` deep link the app will
answer to. Nothing registers that scheme yet.

## Seed data

The seed writes two accounts and copies the Field Recordings pack in as published
material, plus one submission left in review so the moderation queue is not empty.

Accounts are `mod@nuru.local` and `sam@nuru.local`. Their passwords are printed by
the seed script. They are local development logins, generated for an empty local
database, and must not be reused anywhere real.

**Do not run the seed on the public instance.** Start it from an empty
`server/data`, run `npm run db:tags` for the tag vocabulary only, and register
the address in `NURU_MODERATOR_EMAIL` as the first account.

## Not done

Waveform cropping on upload. Email verification and password reset.

## State of the auth

Passwords are scrypt hashed with a per user salt and have to be at least ten
characters. Sessions are random 32 byte tokens in an httpOnly cookie, `Secure`
and `SameSite=Lax` once `NURU_PUBLIC_URL` is an https address, `SameSite=Strict`
otherwise. Expired sessions are deleted at startup and hourly.

Writes are rejected when the `Origin` header names a host the server does not
accept, which together with `SameSite` is what stands in for a CSRF token.
Login, registration, uploads and reports are rate limited per IP, in memory.

Registration lowercases the address, checks its shape, and refuses a short list
of obvious passwords. The account named by `NURU_MODERATOR_EMAIL` becomes a
moderator whenever it registers. **In public mode nobody else ever does**, which
is the one thing that would have been badly wrong to leave as it was: the old
code made the first account to register a moderator, so on a public server that
would have been whoever got there first.

Still missing: email verification, password reset, and any rate limiting that
survives a restart or a distributed attempt.
