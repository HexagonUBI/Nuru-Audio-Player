const REPO = 'HexagonUBI/Nuru-Audio-Player';

function pick(assets, test) {
  return assets.find((a) => test(a.name.toLowerCase()));
}

function size(bytes) {
  return bytes > 1e9
    ? (bytes / 1e9).toFixed(1) + ' GB'
    : Math.round(bytes / 1e6) + ' MB';
}

async function loadRelease() {
  const meta = document.getElementById('download-meta');
  const primary = document.getElementById('download-primary');
  const files = document.getElementById('files');
  const note = document.getElementById('release-note');

  try {
    const res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`);
    if (!res.ok) throw new Error(String(res.status));
    const release = await res.json();
    const assets = release.assets || [];

    const msi = pick(assets, (n) => n.endsWith('.msi'));
    const nsis = pick(assets, (n) => n.endsWith('setup.exe'));
    const msix = pick(assets, (n) => n.endsWith('.msix'));

    if (meta) meta.textContent = release.tag_name || 'latest';
    if (primary && msi) primary.href = msi.browser_download_url;

    const rows = [
      msi && { kind: 'Installer', name: 'MSI', hint: 'Recommended - ' + size(msi.size), url: msi.browser_download_url },
      nsis && { kind: 'Installer', name: 'Setup', hint: 'NSIS - ' + size(nsis.size), url: nsis.browser_download_url },
      msix && { kind: 'Package', name: 'MSIX', hint: 'Needs a trusted certificate - ' + size(msix.size), url: msix.browser_download_url },
    ].filter(Boolean);

    if (files && rows.length) {
      files.innerHTML = '';
      for (const r of rows) {
        const a = document.createElement('a');
        a.className = 'file';
        a.href = r.url;
        a.innerHTML =
          '<span class="kind"></span><span class="name"></span><span class="hint"></span>';
        a.querySelector('.kind').textContent = r.kind;
        a.querySelector('.name').textContent = r.name;
        a.querySelector('.hint').textContent = r.hint;
        files.appendChild(a);
      }
    }

    if (note && release.published_at) {
      const when = new Date(release.published_at).toLocaleDateString(undefined, {
        year: 'numeric',
        month: 'long',
        day: 'numeric',
      });
      note.textContent = `${release.tag_name} published ${when}`;
    }
  } catch {
    if (meta) meta.textContent = 'from GitHub';
    if (note) note.textContent = 'No published release yet.';
  }
}

loadRelease();

const topWrap = document.getElementById('top-wrap');
if (topWrap) {
  const onScroll = () => topWrap.classList.toggle('scrolled', window.scrollY > 8);
  window.addEventListener('scroll', onScroll, { passive: true });
  onScroll();
}

const shot = document.getElementById('hero-shot');
if (shot && !window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
  const onLean = () => {
    const rect = shot.getBoundingClientRect();
    const travelled = Math.min(1, Math.max(0, 1 - rect.top / window.innerHeight));
    shot.style.setProperty('--lean', String(1 - travelled));
  };
  window.addEventListener('scroll', onLean, { passive: true });
  window.addEventListener('resize', onLean);
  onLean();
}
