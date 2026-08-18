# docs

The Nuru website, served by GitHub Pages from this folder.

`index.html` is static. `release.js` asks the GitHub releases API for the latest
release at page load and rewrites the download links to point at its assets, so
publishing a release is enough to update the site. If there is no release yet, or
the API rate limits, the static links fall back to the releases page.

Set the repository Pages source to the `docs/` folder on the default branch.

Later this is also where the sound database front end goes.
