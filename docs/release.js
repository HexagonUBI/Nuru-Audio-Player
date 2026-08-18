(function () {
  'use strict';

  var REPO = 'HexagonUBI/Nuru-Audio-Player';
  var RELEASES = 'https://github.com/' + REPO + '/releases';

  var I18N = window.NuruI18n || {
    t: function (k) {
      return k;
    },
    onChange: function () {
      void 0;
    }
  };
  var latest = null;

  function size(bytes) {
    if (!bytes) return null;
    return bytes > 1e9 ? (bytes / 1e9).toFixed(2) + ' GB' : Math.round(bytes / 1e6) + ' MB';
  }

  function when(iso) {
    if (!iso) return null;
    try {
      var locale = { en: 'en-GB', ru: 'ru-RU', uk: 'uk-UA' }[I18N.lang] || 'en-GB';
      return new Date(iso).toLocaleDateString(locale, {
        year: 'numeric',
        month: 'long',
        day: 'numeric'
      });
    } catch (e) {
      return null;
    }
  }

  function el(id) {
    return document.getElementById(id);
  }

  function fileCard(name, hint, url) {
    var a = document.createElement('a');
    a.className = 'file';
    a.href = url;
    a.innerHTML =
      '<span class="fi"><svg viewBox="0 0 24 24" width="20" height="20" fill="none" ' +
      'stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">' +
      '<path d="M12 3v11M7.5 10.5 12 15l4.5-4.5M4 20h16"/></svg></span>' +
      '<span class="fx"><span class="name"></span><span class="hint"></span></span>';
    a.querySelector('.name').textContent = name;
    a.querySelector('.hint').textContent = hint;
    return a;
  }

  function render() {
    var primary = el('download-primary');
    var label = el('download-label');
    var meta = el('release-meta');
    var files = el('files');
    var note = el('release-note');
    var hdrDl = el('hdr-dl');

    if (!latest) {
      if (label) label.textContent = I18N.t('cta.download');
      if (note) note.textContent = I18N.t('dl.offline');
      if (primary) primary.href = RELEASES;
      if (hdrDl) hdrDl.href = RELEASES;
      return;
    }

    (function (release) {
        var assets = release.assets || [];
        var setup = null;
        for (var i = 0; i < assets.length; i++) {
          var n = (assets[i].name || '').toLowerCase();
          if (n.indexOf('setup') !== -1 && n.slice(-4) === '.exe') {
            setup = assets[i];
            break;
          }
        }

        var tag = release.tag_name || '';
        var published = when(release.published_at);

        if (setup) {
          if (primary) primary.href = setup.browser_download_url;
          if (hdrDl) hdrDl.href = setup.browser_download_url;
          if (label) label.textContent = I18N.t('cta.downloadTag') + ' ' + tag;
          if (files) {
            files.innerHTML = '';
            var hint = I18N.t('dl.winInstaller');
            var s = size(setup.size);
            if (s) hint += ' - ' + s;
            files.appendChild(fileCard('Nuru ' + tag, hint, setup.browser_download_url));
          }
        }

        if (meta) {
          var bits = [I18N.t('dl.win')];
          if (tag) bits.push(tag);
          if (published) bits.push(published);
          meta.textContent = bits.join('  /  ');
        }

        if (note) note.textContent = I18N.t('dl.note2');
    })(latest);
  }

  fetch('https://api.github.com/repos/' + REPO + '/releases/latest')
    .then(function (res) {
      if (!res.ok) throw new Error(String(res.status));
      return res.json();
    })
    .then(function (release) {
      latest = release;
      render();
    })
    .catch(function () {
      latest = null;
      render();
    });

  I18N.onChange(render);
})();
