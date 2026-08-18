# docs

The Nuru website, served by GitHub Pages from this folder.

Set the repository Pages source to the `docs/` folder on the default branch.

## Files

`index.html` is static, and every section is laid out for a landscape screen.
The hero locks to the viewport height once the screen is at least 16:10 and at
least 1180px wide, so on a normal monitor the headline and the screenshot sit on
one screen with nothing cut off. Narrower or squarer screens fall back to a
stacked column.

`style.css` carries the three palettes from the app as `[data-theme]` blocks on
`<html>`. The switch in the header sets that attribute and stores the choice in
localStorage, so the site recolours the same way Settings does.

`i18n.js` holds the English, Russian and Ukrainian strings and paints anything
carrying `data-i18n`. English is the default; the switch in the header stores
the choice and it is remembered after that. Browser language is deliberately not
sniffed. It loads before `demo.js` and exposes `window.NuruI18n`.

The Ukrainian button reads UA because that is what people recognise, but the
code and the `lang` attribute stay `uk`. `uk` is the ISO 639-1 language code;
`ua` is the country code and is not a valid language tag.

`demo.js` is everything interactive:

- nothing, for the hero. `shots/mixer.png` is placed at its own pixel size and
  left alone: no transform, no scaling, so on a 1x display it renders one image
  pixel per screen pixel and the app's own text stays legible. That is why the
  hero stacks rather than sitting beside the copy, and why the shot escapes the
  1440px content column through `.shot-wrap`. It shrinks below about 1580px of
  viewport and is 85 percent of native at 1366. Recapture it with
  `scripts/capture-shots.ps1` when the interface changes.
- the loop panel, the one place that uses the Web Audio API, because the whole
  point of it is an audible A/B. It plays a tone whose buffer either is or is not
  an exact number of cycles long. The unequal one clicks every time it wraps. The
  readout reports the step across the join as a multiple of a normal sample to
  sample step, the same measure `audio/tests.rs` uses.
- the Cozy Mode window, which mirrors `NookMode.svelte` layer for layer: sky
  gradient, three rounded view bands, city lights, lightning, a rain and snow
  canvas, haze, the window frame and mullions, the sill, the hearth glow and the
  vignette. The palettes are the same `SKY` and `VIEW` tables the app uses. Five
  sliders and two segmented pickers stand in for what the mix decides at runtime.
- the theme switch, the sticky header and the scroll reveals.

`release.js` asks the GitHub releases API for the latest release at page load
and rewrites the download links and the version line to point at its NSIS
installer, so publishing a release is enough to update the site. If there is no
release yet, or the API rate limits, the links fall back to the releases page.

## Notes

The site ships no soundscape artwork of its own. The only cover art in the repo
is under `resources/packs/elpy-placeholder/covers/`, which is gitignored and,
per that pack's README, not ours to publish. The hero screenshot is the one place
that artwork appears, and it was already committed and already served as the
`og:image` before this rewrite.

The reveal animation is gated behind a `js` class on `<html>` and has a 2.5
second timeout, so a failure to load `demo.js` leaves the page readable rather
than blank.

The frame thickness in Cozy Mode is set from JavaScript as `--frame` in pixels.
`box-shadow` spread does not accept percentages, so the obvious pure-CSS version
of that inset silently does nothing.

The Cozy Mode canvas always allocates a 2x buffer, not `devicePixelRatio`. Rain
streaks are sub-pixel hairlines and alias badly at 1x.

Later this is also where the sound database front end goes.
