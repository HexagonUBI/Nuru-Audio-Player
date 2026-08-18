(function () {
  'use strict';

  var reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  document.documentElement.classList.add('js');

  function setFill(input) {
    var min = Number(input.min || 0);
    var max = Number(input.max || 100);
    var pct = Math.round(((Number(input.value) - min) / (max - min)) * 1000) / 10;
    input.style.setProperty('--fill', pct + '%');
  }

  var I18N = window.NuruI18n || {
    t: function (k) {
      return k;
    },
    onChange: function () {
      void 0;
    }
  };

  var SKY = {
    night: ['#080a14', '#131a2e', '#1b2136'],
    morning: ['#1a2436', '#4a5a6b', '#6b6553'],
    dusk: ['#0d1016', '#171c26', '#1e232c']
  };

  var VIEW = {
    forest: ['#16281c', '#0f1f16', '#07110c'],
    city: ['#141c26', '#0d141d', '#070b10'],
    beach: ['#13303b', '#0d2531', '#071820'],
    hills: ['#151a20', '#0f1319', '#080a0d']
  };

  var nook = document.getElementById('nook');
  var cozyCanvas = document.getElementById('cozy-canvas');
  var lightsEl = document.getElementById('city-lights');
  var lightningEl = document.getElementById('nook-lightning');
  var viewSeg = document.getElementById('view-seg');
  var skySeg = document.getElementById('sky-seg');

  var dials = {
    rain: document.getElementById('d-rain'),
    snow: document.getElementById('d-snow'),
    wind: document.getElementById('d-wind'),
    fire: document.getElementById('d-fire'),
    storm: document.getElementById('d-storm')
  };
  var dialOut = {
    rain: document.getElementById('v-rain'),
    snow: document.getElementById('v-snow'),
    wind: document.getElementById('v-wind'),
    fire: document.getElementById('v-fire'),
    storm: document.getElementById('v-storm')
  };

  var scene = { view: 'forest', sky: 'night', rain: 0.6, snow: 0, wind: 0.35, hearth: 0.5, storm: 0 };

  function dial(k) {
    return dials[k] ? Number(dials[k].value) / 100 : 0;
  }

  function readScene() {
    scene.rain = dial('rain');
    scene.snow = dial('snow');
    scene.wind = dial('wind');
    scene.hearth = dial('fire');
    scene.storm = dial('storm');
  }

  function cityLights() {
    if (!lightsEl) return;
    if (scene.view !== 'city') {
      lightsEl.innerHTML = '';
      return;
    }
    if (lightsEl.childElementCount) return;
    var rows = 7;
    var cols = 26;
    var html = '';
    for (var c = 0; c < cols; c++) {
      for (var r = 0; r < rows; r++) {
        var seed = Math.sin(c * 12.9898 + r * 78.233) * 43758.5453;
        var rnd = seed - Math.floor(seed);
        if (rnd > 0.55) {
          html +=
            '<span style="left:' +
            ((c / cols) * 100).toFixed(2) +
            '%;top:' +
            ((r / rows) * 100).toFixed(2) +
            '%;opacity:' +
            (0.3 + rnd * 0.7).toFixed(2) +
            '"></span>';
        }
      }
    }
    lightsEl.innerHTML = html;
  }

  function paintNook() {
    if (!nook) return;
    readScene();
    var sky = SKY[scene.sky] || SKY.dusk;
    var view = VIEW[scene.view] || VIEW.hills;
    nook.style.setProperty('--sky-0', sky[0]);
    nook.style.setProperty('--sky-1', sky[1]);
    nook.style.setProperty('--sky-2', sky[2]);
    nook.style.setProperty('--view-0', view[0]);
    nook.style.setProperty('--view-1', view[1]);
    nook.style.setProperty('--view-2', view[2]);
    nook.style.setProperty('--wet', String(Math.min(1, scene.rain)));
    nook.style.setProperty('--gale', String(Math.min(1, scene.wind)));
    nook.style.setProperty('--fire', String(Math.min(1, scene.hearth)));
    nook.setAttribute('data-view', scene.view);
    if (lightningEl) {
      lightningEl.style.display = scene.storm > 0.05 ? '' : 'none';
      lightningEl.style.setProperty('--rate', Math.max(3, 12 - scene.storm * 8).toFixed(1) + 's');
    }
    cityLights();
    seedWeather();
  }

  var particles = [];
  var dpr = 1;
  var cctx = cozyCanvas ? cozyCanvas.getContext('2d') : null;

  function sizeCanvas() {
    if (!cozyCanvas) return;
    dpr = Math.max(2, Math.min(window.devicePixelRatio || 1, 2));
    var w = cozyCanvas.clientWidth || 640;
    var h = cozyCanvas.clientHeight || 360;
    cozyCanvas.width = Math.max(1, Math.floor(w * dpr));
    cozyCanvas.height = Math.max(1, Math.floor(h * dpr));
    if (nook) nook.style.setProperty('--frame', Math.max(10, Math.round(w * 0.055)) + 'px');
  }

  function unit() {
    return (cozyCanvas.width || 1280) / 1280;
  }

  function tilt() {
    return (scene.wind - 0.5) * 2.2;
  }

  function spawn(kind, anywhere) {
    var w = cozyCanvas.width;
    var h = cozyCanvas.height;
    var k = unit();
    if (kind === 'snow') {
      return {
        kind: 'snow',
        x: Math.random() * w * 1.6 - w * 0.3,
        y: anywhere ? Math.random() * h : -12 * k,
        len: (2.2 + Math.random() * 2.6) * k,
        v: (1.1 + Math.random() * 1.5) * k,
        drift: (Math.random() - 0.5) * 1.2,
        phase: Math.random() * Math.PI * 2,
        o: 0.35 + Math.random() * 0.45
      };
    }
    return {
      kind: 'rain',
      x: Math.random() * w * 1.6 - w * 0.3,
      y: anywhere ? Math.random() * h : -40 * k,
      len: (12 + Math.random() * 26) * k,
      v: (9 + Math.random() * 11) * k,
      o: 0.12 + Math.random() * 0.3
    };
  }

  function seedWeather() {
    if (!cozyCanvas || !cctx) return;
    var rainCount = Math.round(Math.min(scene.rain, 1.4) * 330);
    var snowCount = Math.round(Math.min(scene.snow, 1.4) * 170);
    particles = [];
    var i;
    for (i = 0; i < rainCount; i++) particles.push(spawn('rain', true));
    for (i = 0; i < snowCount; i++) particles.push(spawn('snow', true));
    drawWeather();
  }

  function drawWeather() {
    if (!cctx || !cozyCanvas) return;
    var k = unit();
    var lean = tilt();
    cctx.clearRect(0, 0, cozyCanvas.width, cozyCanvas.height);
    cctx.lineCap = 'round';
    for (var i = 0; i < particles.length; i++) {
      var p = particles[i];
      if (p.kind === 'snow') {
        cctx.beginPath();
        cctx.arc(p.x, p.y, p.len, 0, Math.PI * 2);
        cctx.fillStyle = 'rgba(233,240,255,' + p.o + ')';
        cctx.fill();
      } else {
        cctx.beginPath();
        cctx.moveTo(p.x, p.y);
        cctx.lineTo(p.x + lean * p.len * 0.8, p.y + p.len);
        cctx.strokeStyle = 'rgba(190,225,255,' + p.o * (0.4 + scene.rain * 0.6) + ')';
        cctx.lineWidth = 1.3 * k;
        cctx.stroke();
      }
    }
  }

  var visible = false;
  var raf = null;

  function frame() {
    if (!cctx || !cozyCanvas) return;
    var w = cozyCanvas.width;
    var h = cozyCanvas.height;
    var k = unit();
    var lean = tilt();
    for (var i = 0; i < particles.length; i++) {
      var p = particles[i];
      if (p.kind === 'snow') {
        p.phase += 0.02;
        p.y += p.v * (0.55 + scene.snow * 0.7);
        p.x += (Math.sin(p.phase) * 0.5 + p.drift - lean * 1.4) * k;
      } else {
        p.y += p.v * (0.6 + scene.rain * 0.9);
        p.x += lean * 4.5 * k;
      }
      if (p.y > h + p.len) {
        p.y = -p.len;
        p.x = Math.random() * w * 1.6 - w * 0.3;
      }
      if (p.x < -w * 0.35) p.x += w * 1.6;
      if (p.x > w * 1.3) p.x -= w * 1.6;
    }
    drawWeather();
    raf = visible && !reduced ? requestAnimationFrame(frame) : null;
  }

  function segment(el, attr, onPick) {
    if (!el) return;
    el.addEventListener('click', function (ev) {
      var btn = ev.target.closest('button[' + attr + ']');
      if (!btn) return;
      Array.prototype.forEach.call(el.querySelectorAll('button'), function (b) {
        b.setAttribute('aria-pressed', String(b === btn));
      });
      onPick(btn.getAttribute(attr));
    });
  }

  if (nook && cozyCanvas) {
    Object.keys(dials).forEach(function (k) {
      var el = dials[k];
      if (!el) return;
      setFill(el);
      el.addEventListener('input', function () {
        setFill(el);
        dialOut[k].textContent = el.value;
        paintNook();
      });
    });

    segment(viewSeg, 'data-view', function (v) {
      scene.view = v;
      if (lightsEl) lightsEl.innerHTML = '';
      paintNook();
    });
    segment(skySeg, 'data-sky', function (v) {
      scene.sky = v;
      paintNook();
    });

    sizeCanvas();
    paintNook();

    window.addEventListener('resize', function () {
      sizeCanvas();
      seedWeather();
    });

    if ('IntersectionObserver' in window && !reduced) {
      new IntersectionObserver(
        function (entries) {
          visible = entries[0].isIntersecting;
          if (visible) {
            sizeCanvas();
            seedWeather();
            if (!raf) raf = requestAnimationFrame(frame);
          }
        },
        { threshold: 0.05 }
      ).observe(cozyCanvas);
    }
  }

  var seamState = { mode: 'bad', playing: false, ctx: null, src: null };
  var scope = document.getElementById('scope');
  var seamPlay = document.getElementById('seam-play');
  var seamPlayPath = document.getElementById('seam-play-path');
  var seamSeg = document.getElementById('seam-seg');
  var roLen = document.getElementById('ro-len');
  var roStep = document.getElementById('ro-step');
  var roRes = document.getElementById('ro-res');

  var PLAY_D = 'M8 5.5v13l11-6.5-11-6.5Z';
  var PAUSE_D = 'M8 5h3v14H8zM13 5h3v14h-3z';

  var TONE = [
    { f: 110, a: 0.34 },
    { f: 165, a: 0.2 },
    { f: 220, a: 0.11 },
    { f: 330, a: 0.05 }
  ];

  function toneAt(t) {
    var v = 0;
    for (var i = 0; i < TONE.length; i++) v += TONE[i].a * Math.sin(2 * Math.PI * TONE[i].f * t);
    return v;
  }

  function seamCtx() {
    if (!seamState.ctx) {
      var AC = window.AudioContext || window.webkitAudioContext;
      if (!AC) return null;
      seamState.ctx = new AC();
    }
    return seamState.ctx;
  }

  function seamStats(exact) {
    var sr = seamState.ctx ? seamState.ctx.sampleRate : 48000;
    var len = exact ? sr : sr - Math.round(sr / 220);
    var step = Math.abs(toneAt(0) - toneAt((len - 1) / sr));
    var worst = 0;
    for (var i = 1; i < 4096; i++) {
      var d = Math.abs(toneAt(i / sr) - toneAt((i - 1) / sr));
      if (d > worst) worst = d;
    }
    return {
      sr: sr,
      len: len,
      seconds: len / sr,
      cycles: (len / sr) * 110,
      step: step,
      ratio: step / worst
    };
  }

  function drawScope() {
    if (!scope) return;
    var c = scope.getContext('2d');
    var w = scope.width;
    var h = scope.height;
    var accent =
      getComputedStyle(document.documentElement).getPropertyValue('--accent').trim() || '#ffb454';
    var exact = seamState.mode === 'good';
    var st = seamStats(exact);
    var span = Math.round(st.sr * 0.009);

    c.clearRect(0, 0, w, h);
    c.strokeStyle = 'rgba(255,255,255,0.05)';
    c.lineWidth = 1;
    for (var gx = 0; gx <= 8; gx++) {
      c.beginPath();
      c.moveTo((gx / 8) * w, 0);
      c.lineTo((gx / 8) * w, h);
      c.stroke();
    }
    c.beginPath();
    c.moveTo(0, h / 2);
    c.lineTo(w, h / 2);
    c.stroke();

    c.strokeStyle = exact ? accent : '#ff7a70';
    c.lineWidth = 2.4;
    c.lineJoin = 'round';
    c.beginPath();
    var n = span * 2;
    for (var i = 0; i < n; i++) {
      var sample = i < span ? toneAt((st.len - span + i) / st.sr) : toneAt((i - span) / st.sr);
      var px = (i / (n - 1)) * w;
      var py = h / 2 - sample * (h * 0.4);
      if (i === 0) c.moveTo(px, py);
      else c.lineTo(px, py);
    }
    c.stroke();

    c.strokeStyle = 'rgba(255,255,255,0.28)';
    c.setLineDash([5, 6]);
    c.lineWidth = 1.4;
    c.beginPath();
    c.moveTo(w / 2, 0);
    c.lineTo(w / 2, h);
    c.stroke();
    c.setLineDash([]);

    if (!exact) {
      c.strokeStyle = '#ff7a70';
      c.lineWidth = 3;
      c.beginPath();
      c.moveTo(w / 2, h / 2 - toneAt((st.len - 1) / st.sr) * (h * 0.4));
      c.lineTo(w / 2, h / 2 - toneAt(0) * (h * 0.4));
      c.stroke();
      c.fillStyle = 'rgba(255,122,112,0.14)';
      c.fillRect(w / 2 - 3, 0, 6, h);
    }
  }

  function paintReadout() {
    if (!roLen) return;
    var exact = seamState.mode === 'good';
    var st = seamStats(exact);
    roLen.textContent =
      st.seconds.toFixed(4) + ' s / ' + st.cycles.toFixed(1) + ' ' + I18N.t('loop.cycles');
    roStep.textContent =
      st.step.toFixed(3) +
      ' / ' +
      (st.ratio < 1.5 ? I18N.t('loop.same') : st.ratio.toFixed(0) + I18N.t('loop.xnormal'));
    roRes.textContent = exact ? I18N.t('loop.silent') : I18N.t('loop.click');
    roStep.className = exact ? 'good' : 'bad';
    roRes.className = exact ? 'good' : 'bad';
  }

  function seamStop() {
    if (seamState.src) {
      try {
        seamState.src.stop();
      } catch (e) {
        seamState.src = null;
      }
    }
    seamState.src = null;
    seamState.playing = false;
    if (seamPlayPath) seamPlayPath.setAttribute('d', PLAY_D);
  }

  function seamStart() {
    var c = seamCtx();
    if (!c) return;
    if (c.state === 'suspended') c.resume();
    seamStop();
    var exact = seamState.mode === 'good';
    var sr = c.sampleRate;
    var len = exact ? sr : sr - Math.round(sr / 220);
    var buf = c.createBuffer(1, len, sr);
    var d = buf.getChannelData(0);
    for (var i = 0; i < len; i++) d[i] = toneAt(i / sr);
    var s = c.createBufferSource();
    s.buffer = buf;
    s.loop = true;
    var g = c.createGain();
    g.gain.value = 0;
    g.gain.setTargetAtTime(0.3, c.currentTime, 0.08);
    s.connect(g);
    g.connect(c.destination);
    s.start();
    seamState.src = s;
    seamState.playing = true;
    if (seamPlayPath) seamPlayPath.setAttribute('d', PAUSE_D);
  }

  if (scope) {
    paintReadout();
    drawScope();
    seamPlay.addEventListener('click', function () {
      if (seamState.playing) seamStop();
      else seamStart();
    });
    seamSeg.addEventListener('click', function (ev) {
      var btn = ev.target.closest('button[data-seam]');
      if (!btn) return;
      seamState.mode = btn.getAttribute('data-seam');
      Array.prototype.forEach.call(seamSeg.querySelectorAll('button'), function (b) {
        b.setAttribute('aria-pressed', String(b === btn));
      });
      paintReadout();
      drawScope();
      if (seamState.playing) seamStart();
    });
  }

  var themeBtns = document.querySelectorAll('[data-theme-pick]');
  function applyTheme(name) {
    document.documentElement.setAttribute('data-theme', name);
    Array.prototype.forEach.call(themeBtns, function (b) {
      b.setAttribute('aria-pressed', String(b.getAttribute('data-theme-pick') === name));
    });
    try {
      localStorage.setItem('nuru.site.theme', name);
    } catch (e) {
      void 0;
    }
    drawScope();
  }
  Array.prototype.forEach.call(themeBtns, function (b) {
    b.addEventListener('click', function () {
      applyTheme(b.getAttribute('data-theme-pick'));
    });
  });
  try {
    var savedTheme = localStorage.getItem('nuru.site.theme');
    if (savedTheme) applyTheme(savedTheme);
  } catch (e) {
    void 0;
  }

  I18N.onChange(function () {
    paintReadout();
  });

  var shot = document.getElementById('hero-shot');
  if (shot && !reduced) {
    var onLean = function () {
      var rect = shot.getBoundingClientRect();
      var travelled = Math.min(1, Math.max(0, 1 - rect.top / window.innerHeight));
      shot.style.setProperty('--lean', String(1 - travelled));
    };
    window.addEventListener('scroll', onLean, { passive: true });
    window.addEventListener('resize', onLean);
    onLean();
  }

  var hdr = document.getElementById('hdr');
  if (hdr) {
    var onScroll = function () {
      hdr.classList.toggle('stuck', window.scrollY > 8);
    };
    window.addEventListener('scroll', onScroll, { passive: true });
    onScroll();
  }

  var reveals = document.querySelectorAll('.reveal');
  function revealAll() {
    Array.prototype.forEach.call(reveals, function (el) {
      el.classList.add('in');
    });
  }
  setTimeout(revealAll, 2500);
  if ('IntersectionObserver' in window && !reduced) {
    var io = new IntersectionObserver(
      function (entries) {
        entries.forEach(function (e) {
          if (e.isIntersecting) {
            e.target.classList.add('in');
            io.unobserve(e.target);
          }
        });
      },
      { threshold: 0.12 }
    );
    Array.prototype.forEach.call(reveals, function (el) {
      io.observe(el);
    });
  } else {
    revealAll();
  }

  window.addEventListener('resize', function () {
    drawScope();
  });
})();
