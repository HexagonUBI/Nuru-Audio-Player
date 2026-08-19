const view = document.getElementById('view');
const state = {
  user: null,
  licences: [],
  accepted: [],
  tags: [],
  grouped: {},
  limits: {},
  reportReasons: [],
  uploads: true,
  live: false,
};

const esc = (s) =>
  String(s ?? '').replace(/[&<>"']/g, (c) =>
    ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c],
  );

const BASE = (() => {
  const src = document.currentScript?.src;
  if (src) return new URL('.', src).pathname.replace(/\/$/, '');
  return location.pathname.replace(/\/[^/]*$/, '').replace(/\/$/, '');
})();

const at = (path) => `${BASE}${path}`;

function paintModeChip() {
  const chip = document.getElementById('mode-chip');
  if (!chip) return;
  if (!state.live) {
    chip.textContent = 'local dev';
    chip.hidden = false;
    return;
  }
  if (!state.uploads) {
    chip.textContent = 'uploads offline';
    chip.hidden = false;
    return;
  }
  chip.hidden = true;
}

async function api(path, opts = {}) {
  const res = await fetch(at(path), { credentials: 'same-origin', ...opts });
  const text = await res.text();
  let data = null;
  try {
    data = text ? JSON.parse(text) : null;
  } catch {
    data = { error: text.slice(0, 200) };
  }
  if (!res.ok) throw new Error(data?.error ?? `http ${res.status}`);
  return data;
}

const json = (path, method, body) =>
  api(path, { method, headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) });

function fmtTime(s) {
  if (!s || !isFinite(s)) return '0:00';
  const m = Math.floor(s / 60);
  return `${m}:${String(Math.floor(s % 60)).padStart(2, '0')}`;
}

function fmtBytes(b) {
  if (!b) return '-';
  return b > 1e6 ? `${(b / 1e6).toFixed(1)} MB` : `${Math.round(b / 1e3)} kB`;
}

function initials(name) {
  return String(name ?? '?')
    .split(/\s+/)
    .slice(0, 2)
    .map((w) => w[0])
    .join('')
    .toUpperCase();
}

function tileLayers(accent, coverUrl, active) {
  const art = coverUrl ? `url('${coverUrl}')` : 'none';
  if (active) {
    const tint = `linear-gradient(${accent}e0, ${accent}e0)`;
    return { image: `${tint}, ${art}`, blend: 'screen, normal' };
  }
  const scrim = 'linear-gradient(to top, rgba(4, 5, 7, 0.7) 0%, rgba(4, 5, 7, 0) 52%)';
  const dim = 'linear-gradient(rgba(7, 8, 10, 0.66), rgba(7, 8, 10, 0.66))';
  return { image: `${scrim}, ${dim}, ${art}`, blend: 'normal, normal, normal' };
}

function tileHtml(s, opts = {}) {
  const active = Boolean(opts.on);
  const cover = s.hasCover ? at(`/cover/${encodeURIComponent(s.slug)}.jpg`) : null;
  const l = tileLayers(s.accent, cover, active);
  const badge = opts.badge === false ? '' : `<span class="badge ${s.status}">${s.status}</span>`;
  return `
    <button class="tile ${active ? 'active' : ''}"
      style="--accent:${esc(s.accent)};background-image:${l.image};background-blend-mode:${l.blend}"
      aria-pressed="${active}"
      aria-label="${esc(s.name)}${active ? ', playing' : ''}"
      data-slug="${esc(s.slug)}">
      <span class="label">${esc(s.name)}</span>
      ${badge}
    </button>`;
}

function canEdit(s) {
  return Boolean(
    state.user && (state.user.role === 'moderator' || state.user.handle === s.uploaderHandle),
  );
}

function cardHtml(s) {
  const who = s.uploaderHandle
    ? `<a href="#/u/${esc(s.uploaderHandle)}">${esc(s.uploader)}</a>`
    : esc(s.uploader ?? 'unknown');
  return `<div>
    ${tileHtml(s)}
    <div class="card-meta">${who}<span>&middot;</span><span>${fmtTime(s.durationSeconds)}</span></div>
  </div>`;
}

function drawWave(canvas, peaks, progress = 0, accent = '#ffb454') {
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  const w = canvas.clientWidth;
  const h = canvas.clientHeight;
  canvas.width = Math.floor(w * dpr);
  canvas.height = Math.floor(h * dpr);
  const c = canvas.getContext('2d');
  c.scale(dpr, dpr);
  c.clearRect(0, 0, w, h);
  if (!peaks || !peaks.length) {
    c.fillStyle = 'rgba(243,244,246,0.18)';
    c.fillRect(0, h / 2 - 1, w, 2);
    return;
  }
  const n = peaks.length;
  const bar = Math.max(1, w / n - 1);
  for (let i = 0; i < n; i++) {
    const x = (i / n) * w;
    const amp = Math.max(0.02, peaks[i]) * (h * 0.44);
    c.fillStyle = x / w <= progress ? accent : 'rgba(243,244,246,0.22)';
    c.fillRect(x, h / 2 - amp, bar, amp * 2);
  }
}

function mountPlayer(root, sound) {
  const canvas = root.querySelector('canvas');
  const audio = root.querySelector('audio');
  const btn = root.querySelector('.play-btn');
  const time = root.querySelector('.time');
  const wave = root.querySelector('.wave');
  const hover = root.querySelector('.wave-hover');
  const cursor = root.querySelector('.wave-cursor');
  let scrubbing = false;
  let wasPlaying = false;

  const dur = () => (isFinite(audio.duration) && audio.duration) || sound.durationSeconds || 0;
  const paint = () => drawWave(canvas, sound.peaks, dur() ? audio.currentTime / dur() : 0, sound.accent);
  const setTime = () => {
    time.textContent = `${fmtTime(audio.currentTime)} / ${fmtTime(dur())}`;
  };

  paint();
  setTime();
  window.addEventListener('resize', paint);
  audio.addEventListener('loadedmetadata', () => {
    paint();
    setTime();
  });
  audio.addEventListener('timeupdate', () => {
    if (scrubbing) return;
    paint();
    setTime();
  });
  audio.addEventListener('ended', () => {
    btn.textContent = 'Play';
    paint();
  });
  btn.addEventListener('click', () => {
    if (audio.paused) {
      audio.play();
      btn.textContent = 'Pause';
    } else {
      audio.pause();
      btn.textContent = 'Play';
    }
  });

  const ratio = (clientX) => {
    const r = wave.getBoundingClientRect();
    return Math.min(1, Math.max(0, (clientX - r.left) / r.width));
  };

  wave.addEventListener('pointermove', (e) => {
    const p = ratio(e.clientX);
    hover.style.left = `${p * 100}%`;
    hover.textContent = fmtTime(p * dur());
    hover.hidden = false;
    cursor.style.left = `${p * 100}%`;
    cursor.hidden = false;
    if (scrubbing) {
      audio.currentTime = p * dur();
      paint();
      setTime();
    }
  });
  wave.addEventListener('pointerleave', () => {
    hover.hidden = true;
    cursor.hidden = true;
  });
  wave.addEventListener('pointerdown', (e) => {
    scrubbing = true;
    wasPlaying = !audio.paused;
    audio.pause();
    try {
      wave.setPointerCapture(e.pointerId);
    } catch {
      void 0;
    }
    audio.currentTime = ratio(e.clientX) * dur();
    paint();
    setTime();
  });
  const endScrub = (e) => {
    if (!scrubbing) return;
    scrubbing = false;
    try {
      wave.releasePointerCapture(e.pointerId);
    } catch {
      void 0;
    }
    if (wasPlaying) {
      audio.play();
      btn.textContent = 'Pause';
    }
  };
  wave.addEventListener('pointerup', endScrub);
  wave.addEventListener('pointercancel', endScrub);

  wave.addEventListener('keydown', (e) => {
    const step = e.shiftKey ? 10 : 2;
    if (e.key === 'ArrowRight') audio.currentTime = Math.min(dur(), audio.currentTime + step);
    else if (e.key === 'ArrowLeft') audio.currentTime = Math.max(0, audio.currentTime - step);
    else if (e.key === ' ') {
      e.preventDefault();
      btn.click();
      return;
    } else return;
    e.preventDefault();
    paint();
    setTime();
  });
}

function playerHtml(s) {
  if (!s.hasAudio) return '<p class="none">No audio uploaded yet.</p>';
  return `
    <div class="wave" tabindex="0" role="slider" aria-label="Seek"
         aria-valuemin="0" aria-valuemax="${Math.round(s.durationSeconds)}">
      <canvas></canvas>
      <span class="wave-cursor" hidden></span>
      <span class="wave-hover" hidden>0:00</span>
    </div>
    <div class="player">
      <button class="play-btn">Play</button>
      <span class="time">0:00 / ${fmtTime(s.durationSeconds)}</span>
      <span class="hint-sm">drag to scrub, arrows to step, shift for 10s</span>
      <audio preload="metadata" src="${at(`/audio/${esc(s.slug)}.flac`)}"></audio>
    </div>`;
}

function nookPreview(canvas, channel, stateName, weight) {
  const c = canvas.getContext('2d');
  const w = (canvas.width = canvas.clientWidth * 2);
  const h = (canvas.height = canvas.clientHeight * 2);
  const SKY = { night: ['#080a14', '#131a2e'], dusk: ['#0d1016', '#1e232c'], morning: ['#1a2436', '#6b6553'] };
  const VIEW = { forest: '#16281c', city: '#141c26', beach: '#13303b', hills: '#151a20' };
  const sky = channel === 'sky' ? (SKY[stateName] ?? SKY.dusk) : SKY.night;
  const g = c.createLinearGradient(0, 0, 0, h);
  g.addColorStop(0, sky[0]);
  g.addColorStop(1, sky[1]);
  c.fillStyle = g;
  c.fillRect(0, 0, w, h);

  const land = channel === 'window' ? (VIEW[stateName] ?? VIEW.hills) : VIEW.hills;
  c.fillStyle = land;
  c.beginPath();
  c.moveTo(0, h);
  c.lineTo(0, h * 0.66);
  for (let x = 0; x <= w; x += 24) c.lineTo(x, h * 0.66 + Math.sin(x * 0.01) * h * 0.05);
  c.lineTo(w, h);
  c.fill();

  if (channel === 'weather' && (stateName === 'rain' || stateName === 'downpour' || stateName === 'storm')) {
    c.strokeStyle = `rgba(190,225,255,${0.12 + weight * 0.3})`;
    c.lineWidth = 2;
    for (let i = 0; i < Math.round(60 + weight * 220); i++) {
      const x = Math.random() * w;
      const y = Math.random() * h * 0.8;
      const len = 14 + Math.random() * 22;
      c.beginPath();
      c.moveTo(x, y);
      c.lineTo(x - len * 0.3, y + len);
      c.stroke();
    }
  }
  if (channel === 'weather' && stateName === 'snow') {
    c.fillStyle = `rgba(233,240,255,${0.4 + weight * 0.4})`;
    for (let i = 0; i < Math.round(40 + weight * 150); i++) {
      c.beginPath();
      c.arc(Math.random() * w, Math.random() * h * 0.85, 2 + Math.random() * 3, 0, Math.PI * 2);
      c.fill();
    }
  }
  if (channel === 'hearth') {
    const gl = c.createRadialGradient(w * 0.24, h * 1.02, 0, w * 0.24, h * 1.02, h);
    gl.addColorStop(0, `rgba(255,150,70,${0.55 * Math.min(1, weight)})`);
    gl.addColorStop(1, 'rgba(255,110,40,0)');
    c.fillStyle = gl;
    c.fillRect(0, 0, w, h);
  }

  c.fillStyle = 'rgba(9,10,13,0.94)';
  const f = Math.round(w * 0.055);
  c.fillRect(0, 0, w, f);
  c.fillRect(0, h - f, w, f);
  c.fillRect(0, 0, f, h);
  c.fillRect(w - f, 0, f, h);
  c.fillRect(w / 2 - f * 0.18, 0, f * 0.36, h);
  c.fillRect(0, h * 0.46, w, f * 0.36);
}

function field(id, label, value, max, multiline = false) {
  const control = multiline
    ? `<textarea id="${id}" maxlength="${max}" rows="3">${esc(value ?? '')}</textarea>`
    : `<input id="${id}" maxlength="${max}" value="${esc(value ?? '')}" />`;
  return `<div class="field">
    <span class="field-top">${esc(label)}<span class="counter" data-for="${id}">0/${max}</span></span>
    ${control}
  </div>`;
}

function wireCounters(root) {
  root.querySelectorAll('.counter').forEach((c) => {
    const el = root.querySelector(`#${c.dataset.for}`);
    if (!el) return;
    const max = Number(el.getAttribute('maxlength'));
    const paint = () => {
      c.textContent = `${el.value.length}/${max}`;
      c.classList.toggle('over', el.value.length >= max);
    };
    el.addEventListener('input', paint);
    paint();
  });
}

function tagPicker(root, initial = []) {
  const picked = new Set(initial);
  const max = state.limits.tagsPerSound ?? 8;
  const box = root.querySelector('.tagpicker');
  if (!box) return { get: () => [...picked] };

  const render = () => {
    const cats = Object.entries(state.grouped);
    box.innerHTML = `
      <div class="field-top">
        Tags <span class="counter ${picked.size >= max ? 'over' : ''}">${picked.size}/${max}</span>
      </div>
      <div class="tagpicked">${
        [...picked].map((t) => `<span class="pill">${esc(t)} <button type="button" data-drop="${esc(t)}">&times;</button></span>`).join('') ||
        '<span class="none" style="padding:0">Nothing picked yet</span>'
      }</div>
      ${cats
        .map(
          ([cat, list]) => `<div class="tagcat">
            <span class="tagcat-name">${esc(cat)}</span>
            <div class="tagcat-list">${list
              .map(
                (t) => `<button type="button" class="tagchip ${picked.has(t.name) ? 'on' : ''}"
                  data-tag="${esc(t.name)}"
                  ${!picked.has(t.name) && picked.size >= max ? 'disabled' : ''}>${esc(t.name)}</button>`,
              )
              .join('')}</div>
          </div>`,
        )
        .join('')}
      <details class="tagreq">
        <summary class="tagcat-name" style="cursor:pointer">Missing a tag? Request one</summary>
        <div class="row" style="margin-top:8px">
          <input id="tagreq-name" maxlength="${state.limits.tagName ?? 24}" placeholder="tag name" />
          <input id="tagreq-why" maxlength="${state.limits.tagRequestReason ?? 200}" placeholder="why it is needed" />
          <button type="button" class="btn sm" id="tagreq-send">Request</button>
        </div>
        <p class="none" style="padding:6px 0 0">
          A moderator adds it to the official list if it fits. Tags stay a fixed vocabulary so
          browsing does not fill up with near duplicates.
        </p>
        <div id="tagreq-msg"></div>
      </details>`;
  };

  box.addEventListener('click', async (e) => {
    const add = e.target.closest('[data-tag]');
    const drop = e.target.closest('[data-drop]');
    if (add) {
      const t = add.dataset.tag;
      if (picked.has(t)) picked.delete(t);
      else if (picked.size < max) picked.add(t);
      render();
      return;
    }
    if (drop) {
      picked.delete(drop.dataset.drop);
      render();
      return;
    }
    if (e.target.id === 'tagreq-send') {
      const name = box.querySelector('#tagreq-name').value.trim();
      const reason = box.querySelector('#tagreq-why').value.trim();
      const msg = box.querySelector('#tagreq-msg');
      try {
        await json('/api/tags/request', 'POST', { name, reason });
        msg.innerHTML = `<p class="msg ok">Requested "${esc(name)}". A moderator will look.</p>`;
      } catch (err) {
        msg.innerHTML = `<p class="msg err">${esc(err.message)}</p>`;
      }
    }
  });

  render();
  return { get: () => [...picked] };
}

function reportBox(kind, target) {
  if (!state.user) return '';
  return `<details class="panel" style="margin-top:14px">
    <summary style="cursor:pointer;color:var(--ink-40)">Report this ${esc(kind)}</summary>
    <div class="form-grid" style="margin-top:12px">
      <label>Reason<select id="rp-reason">${state.reportReasons
        .map((r) => `<option value="${esc(r.id)}">${esc(r.label)} - ${esc(r.note)}</option>`)
        .join('')}</select></label>
      ${field('rp-note', 'Anything to add', '', state.limits.reportNote ?? 500, true)}
      <button class="btn danger" id="rp-send" data-kind="${esc(kind)}" data-target="${esc(target)}">
        Send report
      </button>
      <div id="rp-msg"></div>
    </div>
  </details>`;
}

function wireReport(root) {
  const btn = root.querySelector('#rp-send');
  if (!btn) return;
  wireCounters(root);
  btn.addEventListener('click', async () => {
    const msg = root.querySelector('#rp-msg');
    try {
      await json('/api/reports', 'POST', {
        kind: btn.dataset.kind,
        target: btn.dataset.target,
        reason: root.querySelector('#rp-reason').value,
        note: root.querySelector('#rp-note').value,
      });
      msg.innerHTML = '<p class="msg ok">Sent. A moderator will take a look.</p>';
    } catch (e) {
      msg.innerHTML = `<p class="msg err">${esc(e.message)}</p>`;
    }
  });
}

async function viewBrowse() {
  const data = await api('/api/sounds');
  const tags = state.tags.filter((t) => t.uses > 0);
  view.innerHTML = `
    <div class="page-head">
      <h1>Browse</h1>
      <span class="sub">${data.total} public ${data.total === 1 ? 'soundscape' : 'soundscapes'}</span>
    </div>
    <div class="filters">
      <input id="q" type="search" placeholder="Search soundscapes" style="min-width:260px" />
      <div class="pills" id="tagfilter">
        <span class="pill link ${!location.hash.includes('tag=') ? '' : ''}" data-tag="">all</span>
        ${tags.map((t) => `<span class="pill link" data-tag="${esc(t.name)}">${esc(t.name)} ${t.uses}</span>`).join('')}
      </div>
    </div>
    <div class="grid-app" id="grid">${data.sounds.map(cardHtml).join('') || '<p class="none">Nothing published yet.</p>'}</div>`;

  let tag = '';
  const refresh = async () => {
    const q = document.getElementById('q').value.trim();
    const d = await api(`/api/sounds?status=public${q ? `&q=${encodeURIComponent(q)}` : ''}${tag ? `&tag=${encodeURIComponent(tag)}` : ''}`);
    document.getElementById('grid').innerHTML =
      d.sounds.map(cardHtml).join('') || '<p class="none">Nothing matches.</p>';
    document.querySelector('.page-head .sub').textContent = `${d.total} shown`;
  };
  document.getElementById('q').addEventListener('input', () => refresh().catch(() => {}));
  document.getElementById('tagfilter').addEventListener('click', (e) => {
    const t = e.target.dataset?.tag;
    if (t === undefined) return;
    tag = t;
    refresh().catch(() => {});
  });
}

async function viewSound(slug) {
  const s = await api(`/api/sounds/${encodeURIComponent(slug)}`);
  const lic = state.licences.find((l) => l.id === s.licence);
  view.innerHTML = `
    <div class="page-head">
      <h1>${esc(s.name)}</h1>
      <span class="sub">${s.status}</span>
      ${s.openReports ? `<span class="flag">${s.openReports} open report${s.openReports === 1 ? '' : 's'}</span>` : ''}
    </div>
    <div class="detail">
      <div class="side">
        ${tileHtml(s, { on: true, badge: false })}
        <a class="btn primary" href="${esc(s.installUrl)}">Install in Nuru</a>
        ${canEdit(s) ? '<button class="btn" id="edit-toggle">Edit details</button>' : ''}
        ${canEdit(s) ? `<button class="btn danger" id="delete-sound" data-name="${esc(s.name)}">Delete</button>` : ''}
        <div class="pills">${s.tags.map((t) => `<span class="pill">${esc(t)}</span>`).join('') || '<span class="pill">untagged</span>'}</div>
      </div>
      <div>
        <div id="player">${playerHtml(s)}</div>
        <p style="margin-top:16px">${esc(s.description) || '<span class="none">No description.</span>'}</p>
        <dl class="facts">
          <div class="fact"><dt>licence</dt><dd>${esc(s.licence)}</dd></div>
          <div class="fact"><dt>uploader</dt><dd>${s.uploaderHandle ? `<a href="#/u/${esc(s.uploaderHandle)}">${esc(s.uploader)}</a>` : esc(s.uploader ?? '-')}</dd></div>
          <div class="fact"><dt>length</dt><dd>${fmtTime(s.durationSeconds)}</dd></div>
          <div class="fact"><dt>audio</dt><dd>${s.sampleRate} Hz ${s.channels}ch</dd></div>
          <div class="fact"><dt>size</dt><dd>${fmtBytes(s.bytes)}</dd></div>
          <div class="fact"><dt>from</dt><dd>${esc(s.sourceCodec ?? 'flac')}</dd></div>
          <div class="fact"><dt>scene</dt><dd>${esc(s.nook.channel)} / ${esc(s.nook.state)}</dd></div>
          <div class="fact"><dt>weight</dt><dd>${s.nook.weight}</dd></div>
        </dl>
        ${lic ? `<p class="none" style="padding:0">${esc(lic.note)}</p>` : ''}
        ${s.attribution ? `<p class="none" style="padding:6px 0 0">${esc(s.attribution)}</p>` : ''}
        <div id="editor"></div>
        ${reportBox('sound', s.slug)}
        <div class="panel" style="margin-top:18px">
          <h3>Cozy Room</h3>
          <p class="why">How this reads in the room when it is the loudest thing playing.</p>
          <div class="nook-preview"><canvas></canvas></div>
        </div>
      </div>
    </div>`;
  if (s.hasAudio) mountPlayer(document.getElementById('player'), s);
  const cv = view.querySelector('.nook-preview canvas');
  if (cv) nookPreview(cv, s.nook.channel, s.nook.state, s.nook.weight);

  wireReport(view);
  const editBtn = document.getElementById('edit-toggle');
  if (editBtn) editBtn.addEventListener('click', () => mountEditor(s));

  const del = document.getElementById('delete-sound');
  if (del) {
    del.addEventListener('click', async () => {
      if (del.dataset.armed !== 'yes') {
        del.dataset.armed = 'yes';
        del.textContent = 'Delete for good?';
        setTimeout(() => {
          del.dataset.armed = '';
          del.textContent = 'Delete';
        }, 4000);
        return;
      }
      await api(`/api/sounds/${s.slug}`, { method: 'DELETE' });
      await loadTags();
      location.hash = '#/browse';
    });
  }
}

function mountEditor(s) {
  const box = document.getElementById('editor');
  if (!box) return;
  const licOptions = state.licences
    .map((l) => `<option value="${esc(l.id)}" ${l.id === s.licence ? 'selected' : ''}>${esc(l.name)}</option>`)
    .join('');
  box.innerHTML = `
    <div class="panel">
      <h3>Edit</h3>
      <p class="why">Details can change freely. Replacing the audio sends it back to review.</p>
      <div class="form-grid">
        ${field('e-name', 'Name', s.name, state.limits.soundName ?? 48)}
        ${field('e-desc', 'Description', s.description, state.limits.soundDescription ?? 600, true)}
        <div class="tagpicker"></div>
        <div class="row">
          <label>Licence<select id="e-lic">${licOptions}</select></label>
          <label>Accent<input id="e-accent" type="color" value="${esc(s.accent)}" /></label>
        </div>
        ${field('e-attr', 'Attribution', s.attribution ?? '', state.limits.attribution ?? 160)}
        <div class="row">
          <label>Channel<select id="e-ch">${['none','weather','window','sky','hearth','life','transit']
            .map((c) => `<option ${c === s.nook.channel ? 'selected' : ''}>${c}</option>`).join('')}</select></label>
          <label>State<input id="e-st" value="${esc(s.nook.state)}" /></label>
          <label>Weight<input id="e-w" type="number" min="0" max="1.5" step="0.05" value="${s.nook.weight}" /></label>
        </div>
        <label class="drop" id="e-audio-drop">
          <strong id="e-audio-name">Replace the audio</strong>
          <span>this returns the soundscape to review</span>
          <input type="file" id="e-audio" />
        </label>
        <label class="drop" id="e-cover-drop">
          <strong id="e-cover-name">Replace the thumbnail</strong>
          <span>square, cropped to 480px</span>
          <input type="file" id="e-cover" accept="image/*" />
        </label>
        <div class="pills">
          <button class="btn primary" id="e-save">Save changes</button>
          <button class="btn" id="e-cancel">Cancel</button>
        </div>
        <div id="e-msg"></div>
      </div>
    </div>`;

  const tags = tagPicker(box, s.tags);
  wireCounters(box);
  document.getElementById('e-audio').addEventListener('change', (e) => {
    const f = e.target.files[0];
    if (f) document.getElementById('e-audio-name').textContent = `${f.name} (${fmtBytes(f.size)})`;
  });
  document.getElementById('e-cover').addEventListener('change', (e) => {
    const f = e.target.files[0];
    if (f) document.getElementById('e-cover-name').textContent = f.name;
  });
  document.getElementById('e-cancel').addEventListener('click', () => {
    box.innerHTML = '';
  });
  document.getElementById('e-save').addEventListener('click', async () => {
    const msg = document.getElementById('e-msg');
    msg.innerHTML = '<p class="msg">Saving...</p>';
    try {
      await json(`/api/sounds/${s.slug}`, 'PATCH', {
        name: document.getElementById('e-name').value,
        description: document.getElementById('e-desc').value,
        licence: document.getElementById('e-lic').value,
        attribution: document.getElementById('e-attr').value,
        accent: document.getElementById('e-accent').value,
        tags: tags.get(),
        nookChannel: document.getElementById('e-ch').value,
        nookState: document.getElementById('e-st').value,
        nookWeight: Number(document.getElementById('e-w').value),
      });
      const cover = document.getElementById('e-cover').files[0];
      if (cover) {
        await api(`/api/sounds/${s.slug}/cover`, {
          method: 'PUT',
          headers: { 'Content-Type': 'application/octet-stream' },
          body: await cover.arrayBuffer(),
        });
      }
      const audio = document.getElementById('e-audio').files[0];
      let requeued = false;
      if (audio) {
        const ext = audio.name.slice(audio.name.lastIndexOf('.')).toLowerCase();
        const r = await api(`/api/sounds/${s.slug}/audio?ext=${encodeURIComponent(ext)}`, {
          method: 'PUT',
          headers: { 'Content-Type': 'application/octet-stream' },
          body: await audio.arrayBuffer(),
        });
        requeued = r.returnedToReview;
      }
      await loadTags();
      msg.innerHTML = requeued
        ? '<p class="msg ok">Saved. The new audio sent this back to review.</p>'
        : '<p class="msg ok">Saved.</p>';
      setTimeout(() => viewSound(s.slug), 700);
    } catch (err) {
      msg.innerHTML = `<p class="msg err">${esc(err.message)}</p>`;
    }
  });
}

async function viewProfile(handle) {
  const d = await api(`/api/users/${encodeURIComponent(handle)}`);
  const p = d.profile;
  const mine = state.user?.handle === p.handle;
  view.innerHTML = `
    <div class="profile-head">
      <span class="avatar lg">${esc(initials(p.name))}</span>
      <div style="flex:1">
        <h1>${esc(p.name)} ${p.role === 'moderator' ? '<span class="role-chip">moderator</span>' : ''}</h1>
        <div class="handle">@${esc(p.handle)}</div>
        <p style="margin-top:6px">${esc(p.bio) || '<span class="none" style="padding:0">No bio yet.</span>'}</p>
      </div>
      <div style="text-align:right">
        <div style="color:var(--ink);font-size:1.6rem;font-family:var(--display)">${p.published}</div>
        <div class="handle">published</div>
      </div>
    </div>
    ${mine ? `<div class="panel"><h3>Your bio</h3><p class="why">Shown on your public profile.</p>
      <textarea id="bio" rows="3">${esc(p.bio)}</textarea>
      <button class="btn sm" id="save-bio" style="margin-top:10px">Save</button>
      <span id="bio-msg"></span></div>` : ''}
    <div class="grid-app">${d.sounds.map(cardHtml).join('') || '<p class="none">Nothing published yet.</p>'}</div>`;
  const save = document.getElementById('save-bio');
  if (save) {
    save.addEventListener('click', async () => {
      await json('/api/me', 'PATCH', { bio: document.getElementById('bio').value });
      document.getElementById('bio-msg').textContent = ' saved';
    });
  }
}

function mixCard(m) {
  const cover = m.hasCover ? at(`/cover/mix-${esc(m.slug)}.jpg`) : null;
  const l = tileLayers(m.accent, cover, false);
  return `<div>
    <button class="tile" data-mix="${esc(m.slug)}"
      style="--accent:${esc(m.accent)};background-image:${l.image};background-blend-mode:${l.blend}">
      <span class="label">${esc(m.title)}</span>
      <span class="badge ${m.status === 'public' ? 'public' : 'review'}">${m.count} ${m.count === 1 ? 'track' : 'tracks'}</span>
    </button>
    <div class="card-meta">${
      m.authorHandle ? `<a href="#/u/${esc(m.authorHandle)}">${esc(m.author)}</a>` : esc(m.author ?? '')
    }<span>&middot;</span><span>${esc(m.status)}</span></div>
  </div>`;
}

async function viewMixtapes() {
  const d = await api('/api/mixtapes');
  const mine = state.user
    ? await api(`/api/mixtapes?by=${encodeURIComponent(state.user.handle)}`)
    : { mixtapes: [] };
  const ownSlugs = new Set(mine.mixtapes.map((m) => m.slug));
  const others = d.mixtapes.filter((m) => !ownSlugs.has(m.slug));
  view.innerHTML = `
    <div class="page-head">
      <h1>Mixtapes</h1>
      <span class="sub">a set of soundscapes meant to be heard as one place</span>
    </div>
    ${
      state.user
        ? `<div class="panel">
             <h3>Start a mixtape</h3>
             <p class="why">Only your own soundscapes can go in it. Two or more to publish.</p>
             <div class="row">
               <label>Title<input id="m-title" placeholder="Coast at Night" /></label>
               <label>Accent<input id="m-accent" type="color" value="#ffb454" /></label>
             </div>
             <button class="btn primary" id="m-create" style="margin-top:10px">Create</button>
             <div id="m-msg"></div>
           </div>`
        : ''
    }
    ${
      mine.mixtapes.length
        ? `<h3 style="margin:18px 0 10px">Yours</h3>
           <div class="grid-app">${mine.mixtapes.map(mixCard).join('')}</div>`
        : ''
    }
    <h3 style="margin:22px 0 10px">${mine.mixtapes.length ? 'From everyone else' : 'Published'}</h3>
    <div class="grid-app">${
      others.map(mixCard).join('') || '<p class="none">Nothing else published yet.</p>'
    }</div>`;

  const create = document.getElementById('m-create');
  if (create) {
    create.addEventListener('click', async () => {
      const title = document.getElementById('m-title').value.trim();
      if (!title) return;
      try {
        const m = await json('/api/mixtapes', 'POST', {
          title,
          accent: document.getElementById('m-accent').value,
        });
        location.hash = `#/m/${m.slug}`;
      } catch (e) {
        document.getElementById('m-msg').innerHTML = `<p class="msg err">${esc(e.message)}</p>`;
      }
    });
  }
}

async function viewMixtape(slug) {
  const m = await api(`/api/mixtapes/${encodeURIComponent(slug)}`);
  const owner = state.user && (state.user.handle === m.authorHandle || state.user.role === 'moderator');
  let pool = { sounds: [] };
  if (owner) {
    try {
      pool = await api('/api/me/sounds');
    } catch {
      pool = { sounds: [] };
    }
  }
  const inTape = new Set(m.sounds.map((s) => s.slug));
  view.innerHTML = `
    <div class="page-head">
      <h1>${esc(m.title)}</h1>
      <span class="sub">${m.count} ${m.count === 1 ? 'track' : 'tracks'} &middot; ${esc(m.status)}</span>
    </div>
    <p>${esc(m.description) || '<span class="none" style="padding:0">No description.</span>'}</p>
    <p class="card-meta" style="margin:8px 0 18px">by ${
      m.authorHandle ? `<a href="#/u/${esc(m.authorHandle)}">${esc(m.author)}</a>` : esc(m.author ?? '')
    }</p>
    ${
      m.status === 'public'
        ? `<a class="btn primary" href="${esc(m.installUrl)}">Install the whole mixtape</a>`
        : m.status === 'review'
          ? '<span class="none" style="padding:0">Waiting on a moderator.</span>'
          : m.status === 'declined'
            ? `<span class="flag">Declined${m.reviewNote ? ': ' + esc(m.reviewNote) : ''}</span>`
            : '<span class="none" style="padding:0">Draft. Send it for review to publish.</span>'
    }
    ${reportBox('mixtape', m.slug)}
    <div class="grid-app" style="margin-top:18px">${
      m.sounds.map((s) => cardHtml(s)).join('') || '<p class="none">No tracks yet.</p>'
    }</div>
    ${
      owner
        ? `<div class="panel" style="margin-top:22px">
             <h3>Manage</h3>
             <p class="why">Add or remove your own soundscapes, then publish.</p>
             <div class="pills" style="margin-bottom:12px">
               ${pool.sounds
                 .map(
                   (s) =>
                     `<button class="pill link" data-toggle="${esc(s.slug)}">${
                       inTape.has(s.slug) ? 'remove' : 'add'
                     } ${esc(s.name)}</button>`,
                 )
                 .join('') || '<span class="none" style="padding:0">You have no uploads yet.</span>'}
             </div>
             <div class="form-grid">
               ${field('mm-title', 'Title', m.title, state.limits.mixtapeTitle ?? 48)}
               ${field('mm-desc', 'Description', m.description, state.limits.mixtapeDescription ?? 600, true)}
             </div>
             <div class="pills" style="margin-top:12px">
               <button class="btn" id="mm-save">Save</button>
               <button class="btn primary" id="mm-publish">${
                 m.status === 'public' || m.status === 'review'
                   ? 'Back to draft'
                   : 'Send for review'
               }</button>
               <button class="btn danger" id="mm-delete">Delete mixtape</button>
             </div>
             <div id="mm-msg"></div>
           </div>`
        : ''
    }`;

  wireReport(view);
  if (!owner) return;
  const msg = document.getElementById('mm-msg');
  wireCounters(view);
  view.querySelectorAll('[data-toggle]').forEach((b) => {
    b.addEventListener('click', async () => {
      const sl = b.dataset.toggle;
      await json(`/api/mixtapes/${m.slug}/sounds`, 'POST', {
        slug: sl,
        remove: inTape.has(sl),
      });
      viewMixtape(slug);
    });
  });
  document.getElementById('mm-save').addEventListener('click', async () => {
    await json(`/api/mixtapes/${m.slug}`, 'PATCH', {
      title: document.getElementById('mm-title').value,
      description: document.getElementById('mm-desc').value,
    });
    viewMixtape(slug);
  });
  document.getElementById('mm-publish').addEventListener('click', async () => {
    try {
      await json(`/api/mixtapes/${m.slug}`, 'PATCH', {
        status: m.status === 'public' || m.status === 'review' ? 'draft' : 'review',
      });
      viewMixtape(slug);
    } catch (e) {
      msg.innerHTML = `<p class="msg err">${esc(e.message)}</p>`;
    }
  });
  document.getElementById('mm-delete').addEventListener('click', async (e) => {
    const b = e.currentTarget;
    if (b.dataset.armed !== 'yes') {
      b.dataset.armed = 'yes';
      b.textContent = 'Delete for good?';
      setTimeout(() => {
        b.dataset.armed = '';
        b.textContent = 'Delete mixtape';
      }, 4000);
      return;
    }
    await api(`/api/mixtapes/${m.slug}`, { method: 'DELETE' });
    location.hash = '#/mixtapes';
  });
}

async function viewRules() {
  const d = await api('/api/rules');
  view.innerHTML = `
    <div class="page-head">
      <h1>Rules</h1>
      <span class="sub">seven of them, and mostly about keeping the library usable</span>
    </div>
    <div class="rulebook">
      <nav class="rule-index">
        ${d.rules.map((r, i) => `<a href="#rule-${i + 1}">${i + 1}. ${esc(r.title)}</a>`).join('')}
      </nav>
      <div>
        <p style="max-width:62ch;margin-bottom:var(--sp-4)">
          Someone browsing should find things they can drop straight into a mix. That is the
          whole point. A moderator checks every upload against these before it goes public.
        </p>
        ${d.rules
          .map(
            (r, i) => `<div class="rule" id="rule-${i + 1}">
              <span class="rule-num">${i + 1}</span>
              <div><h3>${esc(r.title)}</h3><p>${esc(r.body)}</p></div>
            </div>`,
          )
          .join('')}
      </div>
    </div>`;
}

async function viewMine() {
  if (!state.user) {
    view.innerHTML = '<p class="none">Sign in to see your uploads. <a href="#/account">Account</a></p>';
    return;
  }
  const d = await api('/api/me/sounds');
  const byStatus = {};
  for (const s of d.sounds) (byStatus[s.status] ??= []).push(s);
  view.innerHTML = `
    <div class="page-head"><h1>My uploads</h1><span class="sub">${d.sounds.length} total</span></div>
    ${Object.entries(byStatus)
      .map(
        ([st, list]) => `<h3 style="margin:16px 0 10px;text-transform:capitalize">${esc(st)} (${list.length})</h3>
         <div class="grid-app">${list.map(cardHtml).join('')}</div>`,
      )
      .join('') || '<p class="none">Nothing uploaded yet.</p>'}`;
}

async function viewMod(tab = 'sounds') {
  if (state.user?.role !== 'moderator') {
    view.innerHTML = '<p class="none">Moderators only.</p>';
    return;
  }
  const [sounds, tapes, reports, tags] = await Promise.all([
    api('/api/mod/queue'),
    api('/api/mod/mixtapes'),
    api('/api/mod/reports'),
    api('/api/mod/tags'),
  ]);

  const counts = {
    sounds: sounds.sounds.length,
    mixtapes: tapes.mixtapes.length,
    reports: reports.reports.length,
    tags: tags.requested.length,
  };

  const body = {
    sounds: () =>
      sounds.sounds.length
        ? `<div class="grid-app">${sounds.sounds
            .map(
              (x) => `<div>${tileHtml(x)}
                <div class="card-meta">${esc(x.uploader ?? '')}</div>
                <div class="pills" style="margin-top:8px">
                  <button class="btn sm primary" data-accept="${esc(x.slug)}">Accept</button>
                  <button class="btn sm danger" data-decline="${esc(x.slug)}">Decline</button>
                </div></div>`,
            )
            .join('')}</div>`
        : '<p class="none">Nothing waiting.</p>',

    mixtapes: () =>
      tapes.mixtapes.length
        ? tapes.mixtapes
            .map(
              (m) => `<div class="modrow">
                <span class="grow"><b>${esc(m.title)}</b>
                  <em>${esc(m.author ?? '')} - ${m.count} tracks - ${m.sounds
                    .map((x) => esc(x.name))
                    .join(', ')}</em></span>
                <button class="btn sm primary" data-mixaccept="${esc(m.slug)}">Accept</button>
                <button class="btn sm danger" data-mixdecline="${esc(m.slug)}">Decline</button>
              </div>`,
            )
            .join('')
        : '<p class="none">No mixtapes waiting.</p>',

    reports: () =>
      reports.reports.length
        ? reports.reports
            .map(
              (r) => `<div class="modrow">
                <span class="grow"><b>${esc(r.kind)}: ${esc(r.target)}</b>
                  <em>${esc(r.reason)}${r.note ? ' - ' + esc(r.note) : ''} - by ${esc(r.reporter ?? 'someone')}</em></span>
                <a class="btn sm" href="#/${r.kind === 'sound' ? 's' : 'm'}/${esc(r.target)}">Open</a>
                <button class="btn sm danger" data-uphold="${r.id}">Uphold</button>
                <button class="btn sm" data-dismiss="${r.id}">Dismiss</button>
              </div>`,
            )
            .join('')
        : '<p class="none">No open reports.</p>',

    tags: () =>
      tags.requested.length
        ? tags.requested
            .map(
              (t) => `<div class="modrow">
                <span class="grow"><b>${esc(t.name)}</b>
                  <em>${esc(t.reason || 'no reason given')} - by ${esc(t.by ?? 'someone')}</em></span>
                <select data-cat="${esc(t.name)}">${Object.keys(state.grouped)
                  .map((c) => `<option>${esc(c)}</option>`)
                  .join('')}</select>
                <button class="btn sm primary" data-tagok="${esc(t.name)}">Approve</button>
                <button class="btn sm danger" data-tagno="${esc(t.name)}">Reject</button>
              </div>`,
            )
            .join('')
        : '<p class="none">No tag requests.</p>',
  };

  view.innerHTML = `
    <div class="page-head"><h1>Moderation</h1></div>
    <div class="modtabs">
      ${['sounds', 'mixtapes', 'reports', 'tags']
        .map(
          (t) => `<button data-modtab="${t}" class="${t === tab ? 'on' : ''}">${
            t[0].toUpperCase() + t.slice(1)
          }${counts[t] ? ` (${counts[t]})` : ''}</button>`,
        )
        .join('')}
    </div>
    <div id="modbody">${body[tab]()}</div>`;

  view.querySelectorAll('[data-modtab]').forEach((b) =>
    b.addEventListener('click', () => viewMod(b.dataset.modtab)),
  );

  const act = async (fn) => {
    await fn();
    await refreshPips();
    viewMod(tab);
  };
  view.querySelectorAll('[data-mixaccept]').forEach((b) =>
    b.addEventListener('click', () =>
      act(() => json(`/api/mod/mixtapes/${b.dataset.mixaccept}/decision`, 'POST', { decision: 'accept' })),
    ),
  );
  view.querySelectorAll('[data-mixdecline]').forEach((b) =>
    b.addEventListener('click', () =>
      act(() => json(`/api/mod/mixtapes/${b.dataset.mixdecline}/decision`, 'POST', { decision: 'decline' })),
    ),
  );
  view.querySelectorAll('[data-uphold]').forEach((b) =>
    b.addEventListener('click', () =>
      act(() => json(`/api/mod/reports/${b.dataset.uphold}/resolve`, 'POST', { outcome: 'upheld' })),
    ),
  );
  view.querySelectorAll('[data-dismiss]').forEach((b) =>
    b.addEventListener('click', () =>
      act(() => json(`/api/mod/reports/${b.dataset.dismiss}/resolve`, 'POST', { outcome: 'dismissed' })),
    ),
  );
  view.querySelectorAll('[data-tagok]').forEach((b) =>
    b.addEventListener('click', () =>
      act(async () => {
        const cat = view.querySelector(`[data-cat="${b.dataset.tagok}"]`)?.value ?? 'other';
        await json(`/api/mod/tags/${encodeURIComponent(b.dataset.tagok)}/decision`, 'POST', {
          decision: 'approve',
          category: cat,
        });
        await loadTags();
      }),
    ),
  );
  view.querySelectorAll('[data-tagno]').forEach((b) =>
    b.addEventListener('click', () =>
      act(async () => {
        await json(`/api/mod/tags/${encodeURIComponent(b.dataset.tagno)}/decision`, 'POST', {
          decision: 'reject',
        });
        await loadTags();
      }),
    ),
  );
}

async function loadTags() {
  const t = await api('/api/tags');
  state.tags = t.tags;
  state.grouped = t.grouped;
}

async function refreshPips() {
  if (state.user?.role !== 'moderator') return;
  try {
    const [q, m, r, t] = await Promise.all([
      api('/api/mod/queue'),
      api('/api/mod/mixtapes'),
      api('/api/mod/reports'),
      api('/api/mod/tags'),
    ]);
    const n = q.sounds.length + m.mixtapes.length + r.reports.length + t.requested.length;
    const pip = document.getElementById('pip-queue');
    pip.textContent = String(n);
    pip.hidden = n === 0;
  } catch {
    void 0;
  }
}

function viewAccount() {
  if (state.user) {
    location.hash = `#/u/${state.user.handle}`;
    return;
  }
  view.innerHTML = `
    <div class="page-head"><h1>Account</h1></div>
    <div class="row" style="max-width:760px">
      <div class="panel">
        <h3>Sign in</h3>
        <p class="why">Use an existing Workshop account.</p>
        <form class="form-grid" id="login">
          <label>Email<input name="email" type="email" required /></label>
          <label>Password<input name="password" type="password" required /></label>
          <button class="btn primary" type="submit">Sign in</button>
        </form>
      </div>
      <div class="panel">
        <h3>Register</h3>
        <p class="why">Your display name becomes your public profile handle.</p>
        <form class="form-grid" id="register">
          <label>Email<input name="email" type="email" required /></label>
          <label>Display name<input name="name" required /></label>
          <label>Password, 8 characters or more<input name="password" type="password" minlength="8" required /></label>
          <button class="btn primary" type="submit">Create account</button>
        </form>
      </div>
    </div>
    <div id="acct-msg"></div>`;

  const handle = async (id, path, fields) => {
    document.getElementById(id).addEventListener('submit', async (e) => {
      e.preventDefault();
      const f = new FormData(e.target);
      const body = {};
      for (const k of fields) body[k] = f.get(k);
      try {
        await json(path, 'POST', body);
        await refreshUser();
        location.hash = `#/u/${state.user.handle}`;
      } catch (err) {
        document.getElementById('acct-msg').innerHTML = `<p class="msg err">${esc(err.message)}</p>`;
      }
    });
  };
  handle('login', '/api/login', ['email', 'password']);
  handle('register', '/api/register', ['email', 'name', 'password']);
}

function viewUpload() {
  if (!state.user) {
    view.innerHTML = '<p class="none">Sign in to upload. <a href="#/account">Account</a></p>';
    return;
  }
  if (!state.uploads) {
    view.innerHTML =
      '<p class="none">Uploads are offline on this workshop. Browsing and installing still work.</p>';
    return;
  }
  const licOptions = state.licences
    .map((l) => `<option value="${esc(l.id)}">${esc(l.name)}</option>`)
    .join('');
  view.innerHTML = `
    <div class="page-head"><h1>New soundscape</h1><span class="sub">goes to review when you submit</span></div>
    <div class="steps">
      <span class="on">1 describe</span><span>&rsaquo;</span>
      <span class="on">2 audio</span><span>&rsaquo;</span>
      <span class="on">3 how it looks</span><span>&rsaquo;</span>
      <span>4 review by a moderator</span>
    </div>
    <div class="detail">
      <div>
        <div class="panel">
          <h3>What is it</h3>
          <p class="why">The name shows on the tile. Two or three words reads best.</p>
          <div class="form-grid">
            ${field('f-name', 'Name', '', state.limits.soundName ?? 48)}
            ${field('f-desc', 'Description', '', state.limits.soundDescription ?? 600, true)}
          </div>
        </div>

        <div class="panel">
          <h3>Tags</h3>
          <p class="why">
            Pick from the official list. A fixed vocabulary keeps browsing from filling up
            with near duplicates like "rainy" and "rain sounds".
          </p>
          <div class="tagpicker"></div>
        </div>

        <div class="panel">
          <h3>Licence</h3>
          <p class="why" id="lic-note"></p>
          <div class="form-grid">
            <label>Terms<select id="f-lic">${licOptions}</select></label>
            <div id="attr-wrap">${field('f-attr', 'Attribution', '', state.limits.attribution ?? 160)}</div>
          </div>
        </div>

        <div class="panel">
          <h3>Audio</h3>
          <p class="why" id="fmt-note"></p>
          <label class="drop" id="audio-drop">
            <strong id="audio-name">Choose an audio file</strong>
            <span>converted to 48 kHz stereo FLAC on upload</span>
            <input type="file" id="f-audio" />
          </label>
        </div>

        <div class="panel">
          <h3>Scene</h3>
          <p class="why">Optional. Tells Cozy Mode what this sound does to the room.</p>
          <div class="row">
            <label>Channel<select id="f-ch">
              <option>none</option><option>weather</option><option>window</option>
              <option>sky</option><option>hearth</option><option>life</option><option>transit</option>
            </select></label>
            <label>State<input id="f-st" value="none" /></label>
            <label>Weight<input id="f-w" type="number" min="0" max="1.5" step="0.05" value="0.5" /></label>
          </div>
        </div>
      </div>

      <div>
        <div class="panel">
          <h3>Tile preview</h3>
          <p class="why">Exactly how it will sit in the app grid, idle and playing.</p>
          <div class="previews">
            <div>
              <div id="prev-tiles" style="display:grid;gap:10px"></div>
            </div>
            <div>
              <label class="drop" id="cover-drop" style="margin-bottom:10px">
                <strong id="cover-name">Choose a thumbnail</strong>
                <span>square, cropped to 480px</span>
                <input type="file" id="f-cover" accept="image/*" />
              </label>
              <label>Accent<input id="f-accent" type="color" value="#8fb8ff" /></label>
            </div>
          </div>
        </div>

        <div class="panel">
          <h3>Cozy Room</h3>
          <p class="why">What the scene settings above do to the room.</p>
          <div class="nook-preview"><canvas></canvas></div>
          <p class="nook-note" id="nook-note"></p>
        </div>

        <button class="btn primary" id="submit" style="width:100%;justify-content:center">Submit for review</button>
        <div id="up-msg"></div>
      </div>
    </div>`;

  const tags = tagPicker(view, []);
  wireCounters(view);
  const els = {
    name: document.getElementById('f-name'),
    accent: document.getElementById('f-accent'),
    lic: document.getElementById('f-lic'),
    ch: document.getElementById('f-ch'),
    st: document.getElementById('f-st'),
    w: document.getElementById('f-w'),
  };
  let coverUrl = null;

  const paintTiles = () => {
    const fake = {
      slug: 'preview',
      name: els.name.value || 'Untitled',
      accent: els.accent.value,
      status: 'review',
      hasCover: false,
    };
    const one = (on) => {
      const l = tileLayers(fake.accent, coverUrl, on);
      return `
      <button class="tile ${on ? 'active' : ''}" type="button" data-slug="preview"
        style="--accent:${esc(fake.accent)};background-image:${l.image};background-blend-mode:${l.blend}">
        <span class="label">${esc(fake.name)}</span>
      </button>
      <p class="nook-note" style="margin:0 0 6px">${on ? 'playing' : 'idle'}</p>`;
    };
    document.getElementById('prev-tiles').innerHTML = one(false) + one(true);
  };
  const paintNook = () => {
    const cv = view.querySelector('.nook-preview canvas');
    nookPreview(cv, els.ch.value, els.st.value, Number(els.w.value));
    document.getElementById('nook-note').textContent =
      els.ch.value === 'none'
        ? 'No scene channel, so this sound will not change the room.'
        : `Drives the ${els.ch.value} channel as "${els.st.value}" at weight ${els.w.value}.`;
  };
  const paintLic = () => {
    const l = state.licences.find((x) => x.id === els.lic.value);
    document.getElementById('lic-note').textContent = l?.note ?? '';
    document.getElementById('attr-wrap').style.opacity = l?.needsAttribution ? '1' : '0.5';
  };

  els.name.addEventListener('input', paintTiles);
  els.accent.addEventListener('input', paintTiles);
  els.lic.addEventListener('change', paintLic);
  for (const k of ['ch', 'st', 'w']) els[k].addEventListener('input', paintNook);
  els.ch.addEventListener('change', paintNook);

  document.getElementById('f-cover').addEventListener('change', (e) => {
    const f = e.target.files[0];
    if (!f) return;
    document.getElementById('cover-name').textContent = f.name;
    coverUrl = URL.createObjectURL(f);
    paintTiles();
  });
  document.getElementById('f-audio').addEventListener('change', (e) => {
    const f = e.target.files[0];
    if (f) document.getElementById('audio-name').textContent = `${f.name} (${fmtBytes(f.size)})`;
  });
  document.getElementById('fmt-note').textContent =
    `Accepted: ${state.accepted.join('  ')}. Anything else is rejected with a reason.`;

  paintTiles();
  paintNook();
  paintLic();

  document.getElementById('submit').addEventListener('click', async () => {
    const msg = document.getElementById('up-msg');
    const audio = document.getElementById('f-audio').files[0];
    const cover = document.getElementById('f-cover').files[0];
    if (!els.name.value.trim()) {
      msg.innerHTML = '<p class="msg err">A name is required.</p>';
      return;
    }
    msg.innerHTML = '<p class="msg">Uploading...</p>';
    try {
      const created = await json('/api/sounds', 'POST', {
        name: els.name.value,
        description: document.getElementById('f-desc').value,
        licence: els.lic.value,
        attribution: document.getElementById('f-attr').value,
        accent: els.accent.value,
        tags: tags.get(),
        nookChannel: els.ch.value,
        nookState: els.st.value,
        nookWeight: Number(els.w.value),
      });
      if (audio) {
        const ext = audio.name.slice(audio.name.lastIndexOf('.')).toLowerCase();
        await api(`/api/sounds/${created.slug}/audio?ext=${encodeURIComponent(ext)}`, {
          method: 'PUT',
          headers: { 'Content-Type': 'application/octet-stream' },
          body: await audio.arrayBuffer(),
        });
      }
      if (cover) {
        await api(`/api/sounds/${created.slug}/cover`, {
          method: 'PUT',
          headers: { 'Content-Type': 'application/octet-stream' },
          body: await cover.arrayBuffer(),
        });
      }
      await loadTags();
      location.hash = `#/s/${created.slug}`;
    } catch (err) {
      msg.innerHTML = `<p class="msg err">${esc(err.message)}</p>`;
    }
  });
}

async function refreshUser() {
  try {
    state.user = (await api('/api/me')).user;
  } catch {
    state.user = null;
  }
  const right = document.getElementById('hdr-right');
  right.innerHTML = state.user
    ? `<a class="btn ghost sm" href="#/u/${esc(state.user.handle)}">
         <span class="avatar" style="width:18px;height:18px;font-size:9px">${esc(initials(state.user.name))}</span>
         ${esc(state.user.name)}
       </a><button class="btn sm" id="logout">Sign out</button>`
    : '<a class="btn primary sm" href="#/account">Sign in</a>';
  const isMod = state.user?.role === 'moderator';
  document.querySelectorAll('.mod-only').forEach((el) => {
    el.hidden = !isMod;
  });
  const foot = document.getElementById('rail-foot');
  if (foot) {
    foot.innerHTML = state.user
      ? `Signed in as @${esc(state.user.handle)}<br />${esc(state.user.role)}`
      : '<a href="#/account">Sign in</a> to upload';
  }
}

async function route() {
  const hash = location.hash || '#/browse';
  const [, section, arg] = hash.slice(1).split('/');
  document
    .querySelectorAll('#nav a')
    .forEach((a) => a.classList.toggle('on', a.dataset.nav === section));
  try {
    if (section === 'browse') await viewBrowse();
    else if (section === 'upload') viewUpload();
    else if (section === 'mod') await viewMod();
    else if (section === 'account') viewAccount();
    else if (section === 'mixtapes') await viewMixtapes();
    else if (section === 'rules') await viewRules();
    else if (section === 'mine') await viewMine();
    else if (section === 's') await viewSound(arg);
    else if (section === 'm') await viewMixtape(arg);
    else if (section === 'u') await viewProfile(arg);
    else await viewBrowse();
  } catch (e) {
    view.innerHTML = `<p class="msg err">${esc(e.message)}</p>`;
  }
}

document.addEventListener('click', async (e) => {
  const t = e.target.closest('[data-slug], [data-mix], #logout, [data-accept], [data-decline]');
  if (!t) return;
  if (t.id === 'logout') {
    await api('/api/logout', { method: 'POST' });
    await refreshUser();
    location.hash = '#/browse';
    return;
  }
  if (t.dataset.accept || t.dataset.decline) {
    const slug = t.dataset.accept ?? t.dataset.decline;
    await json(`/api/mod/${slug}/decision`, 'POST', {
      decision: t.dataset.accept ? 'accept' : 'decline',
    });
    await viewMod();
    return;
  }
  if (t.dataset.mix) {
    location.hash = `#/m/${t.dataset.mix}`;
    return;
  }
  if (t.dataset.slug && t.dataset.slug !== 'preview') location.hash = `#/s/${t.dataset.slug}`;
});

window.addEventListener('hashchange', route);

(async () => {
  try {
    const l = await api('/api/licences');
    state.licences = l.licences;
    state.accepted = l.accepted;
  } catch {
    state.licences = [];
  }
  try {
    const lim = await api('/api/limits');
    state.limits = lim.limits;
    state.reportReasons = lim.reportReasons;
    state.uploads = lim.uploads !== false;
    state.live = Boolean(lim.live);
  } catch {
    state.limits = {};
  }
  paintModeChip();
  try {
    await loadTags();
  } catch {
    state.tags = [];
    state.grouped = {};
  }
  await refreshUser();
  await refreshPips();
  await route();
})();
