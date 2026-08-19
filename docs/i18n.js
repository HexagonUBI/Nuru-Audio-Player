(function () {
  'use strict';

  var STRINGS = {
    en: {
      "nav.mixer": "Mixer",
      "nav.loop": "The loop",
      "nav.features": "Features",
      "nav.cozy": "Cozy Mode",
      "nav.download": "Download",
      "hdr.download": "Download",
      "hero.eyebrow": "Windows desktop app",
      "hero.h1a": "Every sound you want, ",
      "hero.h1b": "all at once",
      "hero.lede": "Rain over a cafe over a distant train. Nuru layers as many ambient sounds as you like, each one looped at an exact sample and mixed on your own machine. No account, no streaming, nothing uploaded.",
      "cta.download": "Download for Windows",
      "cta.downloadTag": "Download Nuru",
      "cta.source": "Source code",
      "demo.note": "Nuru on Windows. Nineteen soundscapes in the grid, the mix building up the left.",
      "rail.1a": "Sample exact",
      "rail.1b": "loops with no seam",
      "rail.2a": "Local",
      "rail.2b": "nothing streams",
      "rail.3a": "Unlimited",
      "rail.3b": "sounds at once",
      "rail.4a": "Free",
      "rail.4b": "and open source",
      "loop.eyebrow": "The whole point",
      "loop.h2": "A loop you cannot hear turning over.",
      "loop.p1": "Most ambient players watch the playback position and seek back near the end of the file. That cannot be seamless. The event fires every quarter second, the seek lands on a codec frame rather than a sample, and compressed formats carry encoder padding on top.",
      "loop.p2": "Nuru decodes lossless audio, wraps at an exact sample index, and blends the join when the material was not authored to loop. Press play and switch between the two. Same tone, same length, one join.",
      "loop.bad": "Seek and hope",
      "loop.good": "Sample exact",
      "loop.join": "the join",
      "loop.len": "Loop length",
      "loop.step": "Step at the join",
      "loop.result": "Result",
      "loop.click": "Audible click",
      "loop.silent": "Silent join",
      "loop.same": "same as any sample",
      "loop.xnormal": "x a normal sample",
      "loop.cycles": "cycles",
      "feat.eyebrow": "What you get",
      "feat.h2": "Small app, few opinions.",
      "feat.p": "Nuru does one job. Everything below is there because it was missing from the thing it replaces.",
      "feat.1h": "Nothing streams",
      "feat.1p": "Every sound is a local lossless file, checked against its hash before it is allowed to play. Once a pack is on your machine, a dropped connection cannot turn into a gap in the audio.",
      "feat.2h": "Mix and save",
      "feat.2p": "Layer what you want, set each level, keep the result as a named mix.",
      "feat.3h": "Sleep timer",
      "feat.3p": "Fades everything out after a set time, so falling asleep to it does not mean waking up to it.",
      "feat.4h": "Pick your output",
      "feat.4p": "Send Nuru to any device on the machine and leave the rest of your audio where it is. The choice is remembered.",
      "feat.5h": "Three themes, four tile sizes",
      "feat.5p": "The grid keeps the size you chose whatever the window does. The switch at the top of this page uses the same three palettes.",
      "feat.6h": "Updates that stay out of the way",
      "feat.6p": "Nuru installs per user, so an update applies silently with no admin prompt. A small separate window covers the restart, and the release notes are waiting for you when it comes back up.",
      "cozy.eyebrow": "Cozy Mode",
      "cozy.h2": "A room that listens to the mix.",
      "cozy.p": "Full screen, no controls, just the weather you asked for. Add rain and it rains. Add wind and it drives the rain sideways. Turn the fire up and the room warms over. It follows the whole mix rather than picking one sound and ignoring the rest.",
      "cozy.rain": "Rain",
      "cozy.snow": "Snow",
      "cozy.wind": "Wind",
      "cozy.fire": "Fire",
      "cozy.storm": "Storm",
      "cozy.view": "View",
      "cozy.forest": "Forest",
      "cozy.city": "City",
      "cozy.beach": "Beach",
      "cozy.hills": "Hills",
      "cozy.sky": "Sky",
      "cozy.night": "Night",
      "cozy.dusk": "Dusk",
      "cozy.morning": "Morning",
      "dl.eyebrow": "Download",
      "dl.h2": "Windows 10 and 11, 64-bit.",
      "dl.p": "Installs per user in a couple of clicks. No admin prompt.",
      "dl.installer": "Installer",
      "dl.hint": "Latest release on GitHub",
      "dl.win": "Windows 10 and 11, 64-bit",
      "dl.winInstaller": "Windows installer",
      "dl.note": "Nuru is in early development. Builds are unsigned, so Windows may warn you before running the installer.",
      "dl.note2": "Builds are unsigned, so Windows may warn you before running the installer. Nuru installs per user, so there is no admin prompt.",
      "dl.offline": "Could not reach GitHub. Try the releases page directly.",
      "next.eyebrow": "Next",
      "next.h2": "A sound database, later.",
      "next.p": "The plan is a place to share soundscapes: upload a recording, crop it, tag it, and have it installable straight into Nuru with one button. Not built yet.",
      "next.btn": "Follow along on GitHub",
      "foot.source": "Source",
      "foot.releases": "Releases",
      "foot.issues": "Report a problem",
      "foot.kofi": "Support on Ko-fi",
      "aria.theme": "Site theme",
      "aria.lang": "Site language"
    },

    ru: {
      "nav.mixer": "Микшер",
      "nav.loop": "Петля",
      "nav.features": "Возможности",
      "nav.cozy": "Уютный режим",
      "nav.download": "Скачать",
      "hdr.download": "Скачать",
      "hero.eyebrow": "Приложение для Windows",
      "hero.h1a": "Любые звуки, ",
      "hero.h1b": "все сразу",
      "hero.lede": "Дождь поверх кафе поверх далекого поезда. Nuru наслаивает столько фоновых звуков, сколько нужно: каждый зациклен с точностью до сэмпла и сведен на вашем компьютере. Без аккаунта, без стриминга, ничего не уходит в сеть.",
      "cta.download": "Скачать для Windows",
      "cta.downloadTag": "Скачать Nuru",
      "cta.source": "Исходный код",
      "demo.note": "Nuru в Windows. Девятнадцать звуков в сетке, микс собирается слева.",
      "rail.1a": "До сэмпла",
      "rail.1b": "петли без шва",
      "rail.2a": "Локально",
      "rail.2b": "ничего не стримится",
      "rail.3a": "Без лимита",
      "rail.3b": "звуков сразу",
      "rail.4a": "Бесплатно",
      "rail.4b": "и с открытым кодом",
      "loop.eyebrow": "Самое главное",
      "loop.h2": "Петля, оборота которой не слышно.",
      "loop.p1": "Большинство эмбиент-плееров следят за позицией воспроизведения и перематывают назад ближе к концу файла. Бесшовным это быть не может: событие срабатывает раз в четверть секунды, перемотка попадает на кадр кодека, а не на сэмпл, а сжатые форматы вдобавок несут паддинг кодировщика.",
      "loop.p2": "Nuru декодирует lossless, заворачивает петлю на точном индексе сэмпла и смешивает стык, если материал не был сведен под зацикливание. Нажмите play и переключайтесь между режимами. Тот же тон, та же длина, один стык.",
      "loop.bad": "Перемотка наугад",
      "loop.good": "Точно до сэмпла",
      "loop.join": "стык",
      "loop.len": "Длина петли",
      "loop.step": "Скачок на стыке",
      "loop.result": "Результат",
      "loop.click": "Слышимый щелчок",
      "loop.silent": "Стыка не слышно",
      "loop.same": "как у любого сэмпла",
      "loop.xnormal": "x от обычного сэмпла",
      "loop.cycles": "циклов",
      "feat.eyebrow": "Что внутри",
      "feat.h2": "Маленькое приложение без лишнего.",
      "feat.p": "Nuru делает одно дело. Все, что ниже, появилось потому, что этого не хватало в том, что оно заменяет.",
      "feat.1h": "Ничего не стримится",
      "feat.1p": "Каждый звук - локальный lossless-файл, который проверяется по хешу перед воспроизведением. Когда пак уже на диске, обрыв связи не превратится в паузу в звуке.",
      "feat.2h": "Микс и сохранение",
      "feat.2p": "Наслаивайте что угодно, задайте уровни и сохраните результат как именованный микс.",
      "feat.3h": "Таймер сна",
      "feat.3p": "Плавно гасит все через заданное время, чтобы уснуть под звук не значило проснуться под него.",
      "feat.4h": "Выбор устройства",
      "feat.4p": "Отправьте Nuru на любое устройство вывода, а остальной звук оставьте где был. Выбор запоминается.",
      "feat.5h": "Три темы, четыре размера плиток",
      "feat.5p": "Сетка держит выбранный размер, что бы ни делало окно. Переключатель вверху этой страницы использует те же три палитры.",
      "feat.6h": "Обновления, которые не мешают",
      "feat.6p": "Nuru ставится для текущего пользователя, поэтому обновление проходит тихо, без запроса прав администратора. Перезапуск прикрывает отдельное маленькое окно, а после него вас уже ждет список изменений.",
      "cozy.eyebrow": "Уютный режим",
      "cozy.h2": "Комната, которая слушает микс.",
      "cozy.p": "Полный экран, без интерфейса, только та погода, которую вы набрали. Добавьте дождь - и пойдет дождь. Добавьте ветер - и он погонит дождь вбок. Прибавьте огонь - и комната потеплеет. Сцена следует за всем миксом, а не выбирает один звук и игнорирует остальные.",
      "cozy.rain": "Дождь",
      "cozy.snow": "Снег",
      "cozy.wind": "Ветер",
      "cozy.fire": "Огонь",
      "cozy.storm": "Гроза",
      "cozy.view": "Вид",
      "cozy.forest": "Лес",
      "cozy.city": "Город",
      "cozy.beach": "Пляж",
      "cozy.hills": "Холмы",
      "cozy.sky": "Небо",
      "cozy.night": "Ночь",
      "cozy.dusk": "Сумерки",
      "cozy.morning": "Утро",
      "dl.eyebrow": "Скачать",
      "dl.h2": "Windows 10 и 11, 64-бит.",
      "dl.p": "Ставится для текущего пользователя в пару кликов. Без запроса прав администратора.",
      "dl.installer": "Установщик",
      "dl.hint": "Последний релиз на GitHub",
      "dl.win": "Windows 10 и 11, 64-бит",
      "dl.winInstaller": "Установщик для Windows",
      "dl.note": "Nuru на ранней стадии разработки. Сборки не подписаны, поэтому Windows может предупредить перед запуском установщика.",
      "dl.note2": "Сборки не подписаны, поэтому Windows может предупредить перед запуском установщика. Nuru ставится для текущего пользователя, так что прав администратора не потребуется.",
      "dl.offline": "Не удалось связаться с GitHub. Откройте страницу релизов напрямую.",
      "next.eyebrow": "Дальше",
      "next.h2": "База звуков, позже.",
      "next.p": "В планах - место для обмена звуками: загрузить запись, обрезать, проставить теги и поставить ее в Nuru одной кнопкой. Пока не сделано.",
      "next.btn": "Следить на GitHub",
      "foot.source": "Исходники",
      "foot.releases": "Релизы",
      "foot.issues": "Сообщить о проблеме",
      "foot.kofi": "Поддержать на Ko-fi",
      "aria.theme": "Тема сайта",
      "aria.lang": "Язык сайта"
    },

    uk: {
      "nav.mixer": "Мікшер",
      "nav.loop": "Петля",
      "nav.features": "Можливості",
      "nav.cozy": "Затишний режим",
      "nav.download": "Завантажити",
      "hdr.download": "Завантажити",
      "hero.eyebrow": "Застосунок для Windows",
      "hero.h1a": "Будь-які звуки, ",
      "hero.h1b": "усі одразу",
      "hero.lede": "Дощ поверх кафе поверх далекого потяга. Nuru нашаровує стільки фонових звуків, скільки потрібно: кожен зациклений з точністю до семпла і зведений на вашому комп'ютері. Без облікового запису, без стримінгу, нічого не йде в мережу.",
      "cta.download": "Завантажити для Windows",
      "cta.downloadTag": "Завантажити Nuru",
      "cta.source": "Вихідний код",
      "demo.note": "Nuru у Windows. Дев'ятнадцять звуків у сітці, мікс збирається ліворуч.",
      "rail.1a": "До семпла",
      "rail.1b": "петлі без шва",
      "rail.2a": "Локально",
      "rail.2b": "нічого не стримиться",
      "rail.3a": "Без ліміту",
      "rail.3b": "звуків одразу",
      "rail.4a": "Безкоштовно",
      "rail.4b": "і з відкритим кодом",
      "loop.eyebrow": "Найголовніше",
      "loop.h2": "Петля, оберту якої не чути.",
      "loop.p1": "Більшість ембієнт-плеєрів стежать за позицією відтворення і перемотують назад ближче до кінця файлу. Безшовним це бути не може: подія спрацьовує раз на чверть секунди, перемотка потрапляє на кадр кодека, а не на семпл, а стиснуті формати на додачу несуть паддинг кодувальника.",
      "loop.p2": "Nuru декодує lossless, загортає петлю на точному індексі семпла і змішує стик, якщо матеріал не був зведений під зациклення. Натисніть play і перемикайтеся між режимами. Той самий тон, та сама довжина, один стик.",
      "loop.bad": "Перемотка навмання",
      "loop.good": "Точно до семпла",
      "loop.join": "стик",
      "loop.len": "Довжина петлі",
      "loop.step": "Стрибок на стику",
      "loop.result": "Результат",
      "loop.click": "Чутне клацання",
      "loop.silent": "Стику не чути",
      "loop.same": "як у будь-якого семпла",
      "loop.xnormal": "x від звичайного семпла",
      "loop.cycles": "циклів",
      "feat.eyebrow": "Що всередині",
      "feat.h2": "Маленький застосунок без зайвого.",
      "feat.p": "Nuru робить одну справу. Усе, що нижче, з'явилося тому, що цього бракувало в тому, що воно замінює.",
      "feat.1h": "Нічого не стримиться",
      "feat.1p": "Кожен звук - локальний lossless-файл, який перевіряється за хешем перед відтворенням. Коли пак вже на диску, обрив зв'язку не перетвориться на паузу у звуці.",
      "feat.2h": "Мікс і збереження",
      "feat.2p": "Нашаровуйте що завгодно, задайте рівні та збережіть результат як іменований мікс.",
      "feat.3h": "Таймер сну",
      "feat.3p": "Плавно гасить усе через заданий час, щоб заснути під звук не означало прокинутися під нього.",
      "feat.4h": "Вибір пристрою",
      "feat.4p": "Надішліть Nuru на будь-який пристрій виводу, а решту звуку залиште де був. Вибір запам'ятовується.",
      "feat.5h": "Три теми, чотири розміри плиток",
      "feat.5p": "Сітка тримає обраний розмір, хоч би що робило вікно. Перемикач вгорі цієї сторінки використовує ті самі три палітри.",
      "feat.6h": "Оновлення, які не заважають",
      "feat.6p": "Nuru встановлюється для поточного користувача, тому оновлення проходить тихо, без запиту прав адміністратора. Перезапуск прикриває окреме маленьке вікно, а після нього на вас вже чекає список змін.",
      "cozy.eyebrow": "Затишний режим",
      "cozy.h2": "Кімната, яка слухає мікс.",
      "cozy.p": "Повний екран, без інтерфейсу, лише та погода, яку ви набрали. Додайте дощ - і піде дощ. Додайте вітер - і він пожене дощ убік. Додайте вогонь - і кімната потеплішає. Сцена стежить за всім міксом, а не обирає один звук та ігнорує решту.",
      "cozy.rain": "Дощ",
      "cozy.snow": "Сніг",
      "cozy.wind": "Вітер",
      "cozy.fire": "Вогонь",
      "cozy.storm": "Гроза",
      "cozy.view": "Краєвид",
      "cozy.forest": "Ліс",
      "cozy.city": "Місто",
      "cozy.beach": "Пляж",
      "cozy.hills": "Пагорби",
      "cozy.sky": "Небо",
      "cozy.night": "Ніч",
      "cozy.dusk": "Сутінки",
      "cozy.morning": "Ранок",
      "dl.eyebrow": "Завантажити",
      "dl.h2": "Windows 10 і 11, 64-біт.",
      "dl.p": "Встановлюється для поточного користувача за пару кліків. Без запиту прав адміністратора.",
      "dl.installer": "Інсталятор",
      "dl.hint": "Останній реліз на GitHub",
      "dl.win": "Windows 10 і 11, 64-біт",
      "dl.winInstaller": "Інсталятор для Windows",
      "dl.note": "Nuru на ранній стадії розробки. Збірки не підписані, тому Windows може попередити перед запуском інсталятора.",
      "dl.note2": "Збірки не підписані, тому Windows може попередити перед запуском інсталятора. Nuru встановлюється для поточного користувача, тож прав адміністратора не знадобиться.",
      "dl.offline": "Не вдалося зв'язатися з GitHub. Відкрийте сторінку релізів напряму.",
      "next.eyebrow": "Далі",
      "next.h2": "База звуків, згодом.",
      "next.p": "У планах - місце для обміну звуками: завантажити запис, обрізати, проставити теги і поставити його в Nuru однією кнопкою. Поки не зроблено.",
      "next.btn": "Стежити на GitHub",
      "foot.source": "Вихідники",
      "foot.releases": "Релізи",
      "foot.issues": "Повідомити про проблему",
      "foot.kofi": "Підтримати на Ko-fi",
      "aria.theme": "Тема сайту",
      "aria.lang": "Мова сайту"
    }
  };

  var HTML_LANG = { en: 'en', ru: 'ru', uk: 'uk' };
  var STORE_KEY = 'nuru.site.lang';
  var listeners = [];
  var lang = 'en';

  function detect() {
    try {
      var saved = localStorage.getItem(STORE_KEY);
      if (saved && STRINGS[saved]) return saved;
    } catch (e) {
      void 0;
    }
    return 'en';
  }

  function t(key) {
    var table = STRINGS[lang] || STRINGS.en;
    var v = table[key];
    if (v == null) v = STRINGS.en[key];
    return v == null ? key : v;
  }

  function paint() {
    document.documentElement.setAttribute('lang', HTML_LANG[lang] || 'en');
    Array.prototype.forEach.call(document.querySelectorAll('[data-i18n]'), function (el) {
      el.textContent = t(el.getAttribute('data-i18n'));
    });
    Array.prototype.forEach.call(document.querySelectorAll('[data-i18n-aria]'), function (el) {
      el.setAttribute('aria-label', t(el.getAttribute('data-i18n-aria')));
    });
    Array.prototype.forEach.call(document.querySelectorAll('[data-lang-pick]'), function (b) {
      b.setAttribute('aria-pressed', String(b.getAttribute('data-lang-pick') === lang));
    });
  }

  function set(next) {
    if (!STRINGS[next]) return;
    lang = next;
    try {
      localStorage.setItem(STORE_KEY, next);
    } catch (e) {
      void 0;
    }
    paint();
    listeners.forEach(function (fn) {
      fn(lang);
    });
  }

  window.NuruI18n = {
    t: t,
    set: set,
    paint: paint,
    onChange: function (fn) {
      listeners.push(fn);
    },
    get lang() {
      return lang;
    }
  };

  lang = detect();

  document.addEventListener('DOMContentLoaded', function () {
    paint();
    Array.prototype.forEach.call(document.querySelectorAll('[data-lang-pick]'), function (b) {
      b.addEventListener('click', function () {
        set(b.getAttribute('data-lang-pick'));
      });
    });
  });
})();
