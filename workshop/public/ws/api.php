<?php

declare(strict_types=1);

const SLUG = '([a-z0-9](?:[a-z0-9-]{0,62}[a-z0-9])?)';
const VERSION_NUMBER = '([1-9][0-9]{0,5})';
const FILE_NAME = '(audio\.flac|cover\.jpg)';
const ITEM_ID = '/^w-[a-z2-7]{16}$/D';

final class Conflict extends RuntimeException
{
}

const ROUTES = [
    ['GET', '#^/api/health$#D', 'apiHealth'],
    ['GET', '#^/api/options$#D', 'apiOptions'],
    ['GET', '#^/api/me$#D', 'apiMe'],
    ['GET', '#^/api/me/items$#D', 'apiMyItems'],
    ['GET', '#^/api/me/subs$#D', 'apiMySubs'],
    ['GET', '#^/api/tags$#D', 'apiTags'],
    ['GET', '#^/api/items$#D', 'apiItems'],
    ['POST', '#^/api/items$#D', 'apiCreate'],
    ['GET', '#^/api/items/' . SLUG . '$#D', 'apiItem'],
    ['DELETE', '#^/api/items/' . SLUG . '$#D', 'apiDeleteItem'],
    ['POST', '#^/api/items/' . SLUG . '/versions$#D', 'apiNewVersion'],
    ['PATCH', '#^/api/items/' . SLUG . '/versions/' . VERSION_NUMBER . '$#D', 'apiEditVersion'],
    ['DELETE', '#^/api/items/' . SLUG . '/versions/' . VERSION_NUMBER . '$#D', 'apiDeleteVersion'],
    ['PUT', '#^/api/items/' . SLUG . '/versions/' . VERSION_NUMBER . '/' . FILE_NAME . '$#D', 'apiUpload'],
    ['POST', '#^/api/items/' . SLUG . '/versions/' . VERSION_NUMBER . '/submit$#D', 'apiSubmit'],
    ['POST', '#^/api/items/' . SLUG . '/versions/' . VERSION_NUMBER . '/withdraw$#D', 'apiWithdraw'],
    ['PUT', '#^/api/items/' . SLUG . '/vote$#D', 'apiVote'],
    ['POST', '#^/api/items/' . SLUG . '/report$#D', 'apiReport'],
    ['PUT', '#^/api/subs/' . SLUG . '$#D', 'apiSubscribe'],
    ['DELETE', '#^/api/subs/' . SLUG . '$#D', 'apiUnsubscribe'],
    ['GET', '#^/api/mod/queue$#D', 'apiQueue'],
    ['POST', '#^/api/mod/versions/(v-[a-z2-7]{16})$#D', 'apiDecide'],
    ['POST', '#^/api/mod/items/' . SLUG . '$#D', 'apiModItem'],
    ['GET', '#^/api/mod/reports$#D', 'apiReports'],
    ['POST', '#^/api/mod/reports/(r-[a-z2-7]{16})$#D', 'apiCloseReport'],
    ['POST', '#^/api/mod/accounts/(a-[a-z2-7]{16})$#D', 'apiModAccount'],
    ['GET', '#^/f/(w-[a-z2-7]{16})/' . VERSION_NUMBER . '/' . FILE_NAME . '$#D', 'serveFile'],
    ['HEAD', '#^/f/(w-[a-z2-7]{16})/' . VERSION_NUMBER . '/' . FILE_NAME . '$#D', 'serveFile'],
];

function route(): never
{
    $method = strtoupper((string)($_SERVER['REQUEST_METHOD'] ?? 'GET'));
    $path = (string)(parse_url((string)($_SERVER['REQUEST_URI'] ?? '/'), PHP_URL_PATH) ?? '/');
    $allowed = [];
    foreach (ROUTES as [$m, $pattern, $handler]) {
        if (strlen($path) > 300 || !preg_match($pattern, $path, $args)) continue;
        if ($m !== $method) {
            $allowed[] = $m;
            continue;
        }
        array_shift($args);
        start();
        tidy();
        $handler(...$args);
    }
    start();
    if ($allowed) {
        header('Allow: ' . implode(', ', array_unique($allowed)));
        refuse(405, 'bad_method', 'Bad request.');
    }
    refuse(404, 'not_found', 'There is no such page.');
}

function bad(string $field, string $message): never
{
    refuse(400, 'bad_input', $message, ['field' => $field]);
}

function cleanText(mixed $value, int $max, bool $lines = false): ?string
{
    if (!is_string($value) || !preg_match('//u', $value)) return null;
    $value = str_replace("\r\n", "\n", $value);
    $value = (string)preg_replace('/[\x{200B}\x{200E}\x{200F}\x{202A}-\x{202E}\x{2060}-\x{2064}\x{2066}-\x{2069}\x{FEFF}]/u', '', $value);
    $value = (string)preg_replace($lines ? '/(?!\n)[\p{Cc}\x{2028}\x{2029}]/u' : '/[\p{Cc}\x{2028}\x{2029}]/u', ' ', $value);
    if ($lines) $value = (string)preg_replace("/[ \t]*\n[ \t]*\n\s*\n/u", "\n\n", $value);
    $value = (string)preg_replace('/^[\s\p{Z}]+|[\s\p{Z}]+$/u', '', $value);
    return mb_strlen($value) <= $max ? $value : null;
}

function tagsOf(string $stored): array
{
    return array_values(array_filter(explode(',', $stored), fn($t) => $t !== ''));
}

function storedTags(array $tags): string
{
    return $tags ? ',' . implode(',', $tags) . ',' : '';
}

function readMeta(string $kind, mixed $in, array $base): array
{
    if (!is_array($in)) bad('meta', 'The details aren\'t in the right shape.');
    $m = $base;
    if (array_key_exists('title', $in)) {
        $t = cleanText($in['title'], 60);
        if ($t === null || mb_strlen($t) < 2) bad('title', 'Give it a title of 2 to 60 characters.');
        $m['title'] = $t;
    }
    if (array_key_exists('summary', $in)) {
        $s = cleanText($in['summary'], 600, true);
        if ($s === null) bad('summary', 'Keep the description under 600 characters.');
        $m['summary'] = $s;
    }
    if (array_key_exists('tags', $in)) {
        $tags = $in['tags'];
        if (!is_array($tags) || !array_is_list($tags) || count($tags) > 5) bad('tags', 'Pick up to 5 tags.');
        foreach ($tags as $t) if (!is_string($t) || !in_array($t, TAGS, true)) bad('tags', 'That tag isn\'t one of the Workshop tags.');
        $m['tags'] = array_values(array_unique($tags));
    }
    if (array_key_exists('licence', $in)) {
        if (!is_string($in['licence']) || !in_array($in['licence'], LICENCES, true)) bad('licence', 'Pick one of the listed licences.');
        $m['licence'] = $in['licence'];
    }
    foreach (['credit' => 'Keep the credit under 300 characters.', 'source' => 'Keep the source under 300 characters.'] as $field => $message) {
        if (!array_key_exists($field, $in)) continue;
        $v = cleanText($in[$field], 300);
        if ($v === null) bad($field, $message);
        $m[$field] = $v;
    }
    if ($kind === 'sound') {
        if (array_key_exists('accent', $in)) {
            if (!is_string($in['accent']) || !preg_match('/^#[0-9a-fA-F]{6}$/D', $in['accent'])) bad('accent', 'The accent must be a colour like #8FB8FF.');
            $m['accent'] = strtoupper($in['accent']);
        }
        if (array_key_exists('cozy', $in)) $m['cozy'] = readCozy($in['cozy']);
        if (array_key_exists('loop', $in)) $m['loop'] = readLoop($in['loop']);
        if (array_key_exists('lufs', $in)) {
            if (!is_int($in['lufs']) && !is_float($in['lufs']) || $in['lufs'] < -70 || $in['lufs'] > 0) bad('lufs', 'Loudness must be between -70 and 0 LUFS.');
            $m['lufs'] = round((float)$in['lufs'], 2);
        }
    } elseif (array_key_exists('sounds', $in)) {
        $sounds = $in['sounds'];
        if (!is_array($sounds) || !array_is_list($sounds) || count($sounds) > 60) bad('sounds', 'A pack holds up to 60 sounds.');
        foreach ($sounds as $id) if (!is_string($id) || !preg_match(ITEM_ID, $id)) bad('sounds', 'That isn\'t a Workshop sound.');
        $m['sounds'] = array_values(array_unique($sounds));
    }
    return $m;
}

function readCozy(mixed $c): array
{
    if (!is_array($c) || !is_string($c['channel'] ?? null) || !array_key_exists($c['channel'], COZY)) bad('cozy', 'Pick one of the Cozy Mode roles.');
    $channel = $c['channel'];
    if ($channel === 'none') return ['channel' => 'none', 'state' => '', 'weight' => 0];
    $state = $c['state'] ?? null;
    $states = COZY[$channel];
    if (!is_string($state) || ($states === null ? !preg_match('/^[a-z][a-z-]{0,23}$/D', $state) : !in_array($state, $states, true))) bad('cozy', 'Pick one of the Cozy Mode roles.');
    $weight = $c['weight'] ?? null;
    if (!is_int($weight) && !is_float($weight) || $weight < 0 || $weight > 1) bad('cozy', 'The Cozy Mode weight must be between 0 and 1.');
    return ['channel' => $channel, 'state' => $state, 'weight' => round((float)$weight, 3)];
}

function readLoop(mixed $l): array
{
    $ok = is_array($l) && is_int($l['start'] ?? null) && $l['start'] >= 0
        && (($l['end'] ?? null) === null || is_int($l['end']) && $l['end'] > $l['start'])
        && is_int($l['crossfade_ms'] ?? null) && $l['crossfade_ms'] >= 0 && $l['crossfade_ms'] <= 10000;
    if (!$ok) bad('loop', 'The loop points don\'t make sense.');
    return ['start' => $l['start'], 'end' => $l['end'] ?? null, 'crossfade_ms' => $l['crossfade_ms']];
}

function missing(array $item, array $meta, array $files): ?array
{
    if (($meta['title'] ?? '') === '') return ['title', 'Give it a title.'];
    if (empty($meta['tags'])) return ['tags', 'Pick at least one tag.'];
    if (!isset($meta['licence'])) return ['licence', 'Pick a licence.'];
    if (in_array($meta['licence'], CREDIT_NEEDED, true) && ($meta['credit'] ?? '') === '') return ['credit', 'This licence needs a credit line.'];
    foreach (KIND_FILES[$item['kind']] as $name) if (!isset($files[$name])) return [$name, $name === 'cover.jpg' ? 'Add a cover.' : 'Add the audio.'];
    if ($item['kind'] === 'sound') {
        if (!isset($meta['accent'])) return ['accent', 'Pick an accent colour.'];
        $loop = $meta['loop'] ?? null;
        $samples = (int)($files['audio.flac']['samples'] ?? 0);
        if ($loop && ($loop['start'] >= $samples || ($loop['end'] !== null && $loop['end'] > $samples))) return ['loop', 'The loop points are past the end of the audio.'];
    } elseif (empty($meta['sounds'])) {
        return ['sounds', 'Add at least one sound to the pack.'];
    }
    return null;
}

function itemBySlug(string $slug): ?array
{
    return row('SELECT * FROM ws_items WHERE slug = ?', [$slug]);
}

function versionOf(string $item, int $number): ?array
{
    return row('SELECT * FROM ws_versions WHERE item = ? AND number = ?', [$item, $number]);
}

function fileUrl(string $item, int $number, string $name): string
{
    return "/f/$item/$number/$name";
}

function filesOut(array $item, array $v): array
{
    $out = [];
    foreach (jsonArray($v['files']) as $name => $info) {
        unset($info['file']);
        $out[$name] = ['url' => fileUrl($item['id'], (int)$v['number'], $name)] + $info;
    }
    return $out;
}

function canSee(array $item, ?array $who): bool
{
    if ($item['status'] === 'removed') return false;
    if ($item['status'] === 'public') return true;
    return $who !== null && ($who['id'] === $item['owner'] || $who['mod']);
}

function ownItem(string $slug): array
{
    $who = signedIn();
    $item = itemBySlug($slug);
    if (!$item || $item['status'] === 'removed' || ($item['owner'] !== $who['id'] && !canSee($item, $who))) refuse(404, 'not_found', 'There is no such item.');
    if ($item['owner'] !== $who['id']) refuse(403, 'forbidden', 'Only the creator can change this.');
    return [$who, $item];
}

function card(array $item, ?string $meta, ?string $files): array
{
    $m = jsonArray($meta);
    $f = jsonArray($files);
    $n = $item['current_version'] === null ? null : (int)$item['current_version'];
    return [
        'id' => $item['id'],
        'slug' => $item['slug'],
        'kind' => $item['kind'],
        'title' => $item['title'],
        'summary' => $item['summary'],
        'tags' => tagsOf($item['tags']),
        'owner' => ['id' => $item['owner'], 'name' => personName($item['owner'])],
        'status' => $item['status'],
        'version' => $n,
        'cover' => $n !== null && isset($f['cover.jpg']) ? fileUrl($item['id'], $n, 'cover.jpg') : null,
        'seconds' => $f['audio.flac']['seconds'] ?? null,
        'accent' => $m['accent'] ?? null,
        'subs' => (int)$item['subs'],
        'up' => (int)$item['up'],
        'down' => (int)$item['down'],
        'published' => $item['published'] === null ? null : (int)$item['published'],
        'updated' => (int)$item['updated'],
    ];
}

function versionOut(array $item, array $v, bool $private): array
{
    $out = [
        'id' => $v['id'],
        'number' => (int)$v['number'],
        'status' => $v['status'],
        'rev' => (int)$v['rev'],
        'note' => $v['note'],
        'meta' => jsonArray($v['meta']),
        'files' => filesOut($item, $v),
        'bytes' => (int)$v['bytes'],
        'created' => (int)$v['created'],
        'submitted' => $v['submitted'] === null ? null : (int)$v['submitted'],
        'decided' => $v['decided'] === null ? null : (int)$v['decided'],
    ];
    if ($private) $out['decision_note'] = $v['decision_note'];
    return $out;
}

function packSounds(array $meta, bool $publicOnly): array
{
    $out = [];
    foreach ($meta['sounds'] ?? [] as $id) {
        $s = row("SELECT i.*, v.meta AS v_meta, v.files AS v_files FROM ws_items i LEFT JOIN ws_versions v ON v.item = i.id AND v.number = i.current_version WHERE i.id = ? AND i.kind = 'sound'", [$id]);
        if (!$s || ($publicOnly && $s['status'] !== 'public')) continue;
        $entry = card($s, $s['v_meta'], $s['v_files']);
        if ($s['current_version'] !== null) {
            $entry['meta'] = jsonArray($s['v_meta']);
            $entry['files'] = filesOut($s, ['number' => $s['current_version'], 'files' => $s['v_files']]);
        }
        $out[] = $entry;
    }
    return $out;
}

function itemOut(array $item, ?array $who): array
{
    $current = $item['current_version'] === null ? null : versionOf($item['id'], (int)$item['current_version']);
    $out = card($item, $current['meta'] ?? null, $current['files'] ?? null);
    $out['meta'] = $current ? jsonArray($current['meta']) : null;
    $out['files'] = $current ? filesOut($item, $current) : null;
    if ($item['kind'] === 'pack' && $current) $out['sounds'] = packSounds(jsonArray($current['meta']), true);
    $private = $who !== null && ($who['id'] === $item['owner'] || $who['mod']);
    if ($private) {
        $out['versions'] = array_map(fn($v) => versionOut($item, $v, true), rows('SELECT * FROM ws_versions WHERE item = ? ORDER BY number DESC', [$item['id']]));
    }
    if ($who !== null) {
        $out['me'] = [
            'subscribed' => (bool)value('SELECT 1 FROM ws_subs WHERE account = ? AND item = ?', [$who['id'], $item['id']]),
            'vote' => (int)(value('SELECT value FROM ws_votes WHERE account = ? AND item = ?', [$who['id'], $item['id']]) ?? 0),
            'owner' => $who['id'] === $item['owner'],
        ];
    }
    return $out;
}

function slugFor(string $title, string $kind): string
{
    $ascii = function_exists('iconv') ? (string)@iconv('UTF-8', 'ASCII//TRANSLIT//IGNORE', $title) : $title;
    $base = trim((string)preg_replace('/[^a-z0-9]+/', '-', strtolower($ascii)), '-');
    $base = rtrim(substr($base, 0, 48), '-');
    if ($base === '') $base = $kind;
    $slug = $base;
    for ($i = 2; itemBySlug($slug) !== null; $i++) {
        $slug = $i < 100 ? "$base-$i" : $base . '-' . substr(newId(''), 0, 6);
    }
    return $slug;
}

function recount(string $item): void
{
    sql('UPDATE ws_items SET up = (SELECT COUNT(*) FROM ws_votes WHERE item = ? AND value = 1), down = (SELECT COUNT(*) FROM ws_votes WHERE item = ? AND value = -1), subs = (SELECT COUNT(*) FROM ws_subs WHERE item = ?) WHERE id = ?',
        [$item, $item, $item, $item]);
    $r = row('SELECT up, down FROM ws_items WHERE id = ?', [$item]);
    if (!$r) return;
    $up = (int)$r['up'];
    $n = $up + (int)$r['down'];
    $score = 0.0;
    if ($n > 0) {
        $z = 1.96;
        $p = $up / $n;
        $score = ($p + $z * $z / (2 * $n) - $z * sqrt(($p * (1 - $p) + $z * $z / (4 * $n)) / $n)) / (1 + $z * $z / $n);
    }
    sql('UPDATE ws_items SET score = ? WHERE id = ?', [round($score, 6), $item]);
}

function publicSounds(array $meta): int
{
    $count = 0;
    foreach ($meta['sounds'] ?? [] as $id) {
        if (value("SELECT status FROM ws_items WHERE id = ? AND kind = 'sound'", [$id]) === 'public') $count++;
    }
    return $count;
}

function settlePacks(): void
{
    $packs = rows("SELECT i.id, v.meta FROM ws_items i JOIN ws_versions v ON v.item = i.id AND v.number = i.current_version WHERE i.kind = 'pack' AND i.status = 'public'");
    foreach ($packs as $p) {
        if (publicSounds(jsonArray($p['meta'])) === 0) {
            sql("UPDATE ws_items SET status = 'hidden', updated = ? WHERE id = ? AND status = 'public'", [now(), $p['id']]);
            audit(null, 'hide', "{$p['id']} has no public sounds left");
        }
    }
}

function purgeItem(array $item, bool $keepRow): void
{
    atomic(function () use ($item, $keepRow) {
        sql('DELETE FROM ws_versions WHERE item = ?', [$item['id']]);
        sql('DELETE FROM ws_subs WHERE item = ?', [$item['id']]);
        sql('DELETE FROM ws_votes WHERE item = ?', [$item['id']]);
        if ($keepRow) {
            sql("UPDATE ws_items SET status = 'removed', current_version = NULL, subs = 0, up = 0, down = 0, score = 0, updated = ? WHERE id = ?", [now(), $item['id']]);
        } else {
            sql('DELETE FROM ws_reports WHERE item = ?', [$item['id']]);
            sql('DELETE FROM ws_items WHERE id = ?', [$item['id']]);
        }
    });
    dropDir(dataDir() . '/files/' . $item['id']);
}

function apiHealth(): never
{
    db();
    out(['ok' => true]);
}

function apiOptions(): never
{
    header_remove('Cache-Control');
    header('Cache-Control: public, max-age=3600');
    out([
        'kinds' => array_keys(KIND_FILES),
        'files' => KIND_FILES,
        'tags' => TAGS,
        'licences' => LICENCES,
        'credit_needed' => CREDIT_NEEDED,
        'cozy' => array_map(fn($s) => $s, COZY),
        'report_reasons' => REPORT_REASONS,
        'limits' => [
            'audio_bytes' => (int)setting('audio_bytes', 100 << 20),
            'cover_bytes' => (int)setting('cover_bytes', 1 << 20),
            'min_seconds' => (int)setting('min_seconds', 5),
            'max_seconds' => (int)setting('max_seconds', 3600),
            'rates' => [44100, 48000],
            'cover_side' => [256, 1024],
            'title' => 60,
            'summary' => 600,
            'tags' => 5,
            'pack_sounds' => 60,
        ],
    ]);
}

function apiMe(): never
{
    $who = viewer();
    if ($who === null) refuse(401, 'signed_out', 'Sign in to Hexagon Cloud first.');
    out(['account' => ['id' => $who['id'], 'name' => $who['name']], 'moderator' => $who['mod'], 'banned' => $who['banned']]);
}

function apiTags(): never
{
    $counts = array_fill_keys(TAGS, 0);
    foreach (rows("SELECT tags FROM ws_items WHERE status = 'public'") as $r) {
        foreach (tagsOf($r['tags']) as $t) if (isset($counts[$t])) $counts[$t]++;
    }
    out(['tags' => array_map(fn($t) => ['tag' => $t, 'count' => $counts[$t]], TAGS)]);
}

function apiItems(): never
{
    $where = ["i.status = 'public'"];
    $args = [];
    $kind = $_GET['kind'] ?? '';
    if ($kind !== '' && (!is_string($kind) || !isset(KIND_FILES[$kind]))) bad('kind', 'Filter by sound or pack.');
    if ($kind !== '') {
        $where[] = 'i.kind = ?';
        $args[] = $kind;
    }
    $tag = $_GET['tag'] ?? '';
    if ($tag !== '' && (!is_string($tag) || !in_array($tag, TAGS, true))) bad('tag', 'That tag isn\'t one of the Workshop tags.');
    if ($tag !== '') {
        $where[] = 'i.tags LIKE ?';
        $args[] = "%,$tag,%";
    }
    $owner = $_GET['owner'] ?? '';
    if ($owner !== '' && (!is_string($owner) || !preg_match(ACCOUNT_ID, $owner))) bad('owner', 'That isn\'t an account id.');
    if ($owner !== '') {
        $where[] = 'i.owner = ?';
        $args[] = $owner;
    }
    $q = $_GET['q'] ?? '';
    if ($q !== '' && (!is_string($q) || ($q = cleanText($q, 80)) === null)) bad('q', 'Search with up to 80 characters.');
    if ($q !== '') {
        foreach (array_slice(preg_split('/\s+/u', mb_strtolower($q)) ?: [], 0, 5) as $word) {
            $like = '%' . strtr($word, ['!' => '!!', '%' => '!%', '_' => '!_']) . '%';
            $where[] = "(LOWER(i.title) LIKE ? ESCAPE '!' OR LOWER(i.summary) LIKE ? ESCAPE '!' OR i.tags LIKE ? ESCAPE '!')";
            array_push($args, $like, $like, $like);
        }
    }
    $order = match ($_GET['sort'] ?? 'new') {
        'new' => 'i.published DESC',
        'popular' => 'i.subs DESC, i.published DESC',
        'top' => 'i.score DESC, i.published DESC',
        'updated' => 'i.updated DESC',
        default => bad('sort', 'Sort by new, popular, top or updated.'),
    };
    $page = max(1, min(1000, (int)($_GET['page'] ?? 1)));
    $per = max(1, min(48, (int)($_GET['per'] ?? 24)));
    $clause = implode(' AND ', $where);
    $total = (int)value("SELECT COUNT(*) FROM ws_items i WHERE $clause", $args);
    $offset = ($page - 1) * $per;
    $list = rows("SELECT i.*, v.meta AS v_meta, v.files AS v_files FROM ws_items i LEFT JOIN ws_versions v ON v.item = i.id AND v.number = i.current_version WHERE $clause ORDER BY $order, i.id LIMIT $per OFFSET $offset", $args);
    out([
        'items' => array_map(fn($r) => card($r, $r['v_meta'], $r['v_files']), $list),
        'page' => $page,
        'per' => $per,
        'total' => $total,
    ]);
}

function apiItem(string $slug): never
{
    $who = viewer();
    $item = itemBySlug($slug);
    if (!$item || !canSee($item, $who)) refuse(404, 'not_found', 'There is no such item.');
    out(['item' => itemOut($item, $who)]);
}

function apiCreate(): never
{
    $who = signedIn();
    $in = body();
    $kind = $in['kind'] ?? null;
    if (!is_string($kind) || !isset(KIND_FILES[$kind])) bad('kind', 'Pick a sound or a pack.');
    $meta = readMeta($kind, $in['meta'] ?? [], []);
    if (!isset($meta['title'])) bad('title', 'Give it a title of 2 to 60 characters.');
    $drafts = (int)value("SELECT COUNT(*) FROM ws_items WHERE owner = ? AND status = 'new'", [$who['id']]);
    if ($drafts >= (int)setting('max_unpublished', 50)) refuse(409, 'too_many_drafts', 'Finish or delete some unpublished items first.');
    rateLimit('create', $who['id'], 'That is a lot of new items for one day.');
    $now = now();
    $id = newId('w-');
    for ($try = 1; ; $try++) {
        try {
            $item = atomic(function () use ($who, $kind, $meta, $now, $id) {
                $slug = slugFor($meta['title'], $kind);
                sql('INSERT INTO ws_items (id, kind, slug, owner, status, current_version, title, summary, tags, subs, up, down, score, created, published, updated) VALUES (?, ?, ?, ?, ?, NULL, ?, ?, ?, 0, 0, 0, 0, ?, NULL, ?)',
                    [$id, $kind, $slug, $who['id'], 'new', $meta['title'], $meta['summary'] ?? '', storedTags($meta['tags'] ?? []), $now, $now]);
                sql('INSERT INTO ws_versions (id, item, number, status, note, meta, files, bytes, created, submitted, decided, decided_by, decision_note) VALUES (?, ?, 1, ?, ?, ?, ?, 0, ?, NULL, NULL, NULL, ?)',
                    [newId('v-'), $id, 'draft', '', jsonText($meta), '{}', $now, '']);
                return itemBySlug($slug);
            });
            break;
        } catch (PDOException $e) {
            if (!duplicate($e) || $try >= 3) throw $e;
        }
    }
    audit($who['id'], 'create', $item['slug']);
    out(['item' => itemOut($item, $who)], 201);
}

function apiDeleteItem(string $slug): never
{
    [$who, $item] = ownItem($slug);
    purgeItem($item, $item['status'] !== 'new');
    if ($item['kind'] === 'sound') settlePacks();
    audit($who['id'], 'delete', $item['slug']);
    out(['ok' => true]);
}

function apiNewVersion(string $slug): never
{
    [$who, $item] = ownItem($slug);
    $in = body();
    rateLimit('edit', $who['id']);
    $open = value("SELECT number FROM ws_versions WHERE item = ? AND status IN ('draft', 'review')", [$item['id']]);
    if ($open !== null) refuse(409, 'open_version', 'Finish or withdraw the version that is already open.', ['version' => (int)$open]);
    $base = $item['current_version'] === null ? [] : jsonArray(versionOf($item['id'], (int)$item['current_version'])['meta'] ?? null);
    $meta = readMeta($item['kind'], $in['meta'] ?? [], $base);
    $note = cleanText($in['note'] ?? '', 1000, true);
    if ($note === null) bad('note', 'Keep the change note under 1000 characters.');
    $now = now();
    try {
        $v = atomic(function () use ($item, $meta, $note, $now) {
            if (value("SELECT 1 FROM ws_versions WHERE item = ? AND status IN ('draft', 'review')", [$item['id']]) !== null) throw new Conflict();
            $number = (int)value('SELECT COALESCE(MAX(number), 0) FROM ws_versions WHERE item = ?', [$item['id']]) + 1;
            sql('INSERT INTO ws_versions (id, item, number, status, note, meta, files, bytes, created, submitted, decided, decided_by, decision_note) VALUES (?, ?, ?, ?, ?, ?, ?, 0, ?, NULL, NULL, NULL, ?)',
                [newId('v-'), $item['id'], $number, 'draft', $note, jsonText($meta), '{}', $now, '']);
            return versionOf($item['id'], $number);
        });
    } catch (Conflict $e) {
        refuse(409, 'open_version', 'Finish or withdraw the version that is already open.');
    } catch (PDOException $e) {
        if (!duplicate($e)) throw $e;
        refuse(409, 'open_version', 'Finish or withdraw the version that is already open.');
    }
    out(['version' => versionOut($item, $v, true)], 201);
}

function apiEditVersion(string $slug, string $number): never
{
    [$who, $item] = ownItem($slug);
    $v = versionOf($item['id'], (int)$number);
    if (!$v) refuse(404, 'not_found', 'There is no such version.');
    if ($v['status'] !== 'draft') refuse(409, 'not_draft', 'Only a draft can be changed.');
    $in = body();
    rateLimit('edit', $who['id']);
    $meta = readMeta($item['kind'], $in['meta'] ?? [], jsonArray($v['meta']));
    $note = $v['note'];
    if (array_key_exists('note', $in)) {
        $note = cleanText($in['note'], 1000, true);
        if ($note === null) bad('note', 'Keep the change note under 1000 characters.');
    }
    $changed = sql("UPDATE ws_versions SET meta = ?, note = ?, rev = rev + 1 WHERE id = ? AND status = 'draft' AND rev = ?", [jsonText($meta), $note, $v['id'], $v['rev']])->rowCount();
    if ($changed !== 1) refuse(409, 'not_draft', 'Only a draft can be changed.');
    if ($item['status'] === 'new') {
        sql('UPDATE ws_items SET title = ?, summary = ?, tags = ?, updated = ? WHERE id = ?',
            [$meta['title'] ?? $item['title'], $meta['summary'] ?? '', storedTags($meta['tags'] ?? []), now(), $item['id']]);
    }
    out(['version' => versionOut($item, versionOf($item['id'], (int)$number), true)]);
}

function apiDeleteVersion(string $slug, string $number): never
{
    [$who, $item] = ownItem($slug);
    $v = versionOf($item['id'], (int)$number);
    if (!$v) refuse(404, 'not_found', 'There is no such version.');
    if (!in_array($v['status'], ['draft', 'declined'], true)) refuse(409, 'not_draft', 'Only a draft or a declined version can be deleted.');
    sql('DELETE FROM ws_versions WHERE id = ?', [$v['id']]);
    dropDir(filesDir($item['id'], (int)$number));
    out(['ok' => true]);
}

function apiUpload(string $slug, string $number, string $name): never
{
    [$who, $item] = ownItem($slug);
    $v = versionOf($item['id'], (int)$number);
    if (!$v) refuse(404, 'not_found', 'There is no such version.');
    if ($v['status'] !== 'draft') refuse(409, 'not_draft', 'Only a draft can be changed.');
    if (!in_array($name, KIND_FILES[$item['kind']], true)) refuse(400, 'bad_file', 'That file doesn\'t belong to this kind of item.');
    rateLimit('upload', $who['id'], 'That is a lot of uploads.');
    $lock = ownerLock($who['id']);
    $max = $name === 'audio.flac' ? (int)setting('audio_bytes', 100 << 20) : (int)setting('cover_bytes', 1 << 20);
    $length = (int)($_SERVER['CONTENT_LENGTH'] ?? $_SERVER['HTTP_CONTENT_LENGTH'] ?? 0);
    $replacing = (int)(jsonArray($v['files'])[$name]['bytes'] ?? 0);
    $pending = (int)value("SELECT COALESCE(SUM(v.bytes), 0) FROM ws_versions v JOIN ws_items i ON i.id = v.item WHERE i.owner = ? AND v.status IN ('draft', 'review')", [$who['id']]);
    if ($pending - $replacing + $length > (int)setting('pending_bytes', 2 << 30)) refuse(409, 'pending_full', 'Your unpublished uploads are using all their space. Delete some drafts first.');
    $total = (int)value('SELECT COALESCE(SUM(bytes), 0) FROM ws_versions');
    if ($total - $replacing + $length > (int)setting('disk_bytes', 20 << 30)) refuse(507, 'full', 'The Workshop is out of space right now.');
    $dir = filesDir($item['id'], (int)$number);
    $got = receiveUpload($dir, $name, $max);
    if ($name === 'audio.flac') {
        $info = flacInfo($got['tmp']);
        $minSeconds = (int)setting('min_seconds', 5);
        $maxSeconds = (int)setting('max_seconds', 3600);
        $problem = match (true) {
            $info === null => 'That isn\'t a FLAC file.',
            !in_array($info['rate'], [44100, 48000], true) => 'Use 44.1 or 48 kHz audio.',
            $info['channels'] > 2 => 'Use mono or stereo audio.',
            !in_array($info['bits'], [16, 24], true) => 'Use 16 or 24 bit audio.',
            $info['seconds'] < $minSeconds || $info['seconds'] > $maxSeconds => "Sounds must be $minSeconds seconds to " . round($maxSeconds / 60) . ' minutes long.',
            default => null,
        };
        $code = 'bad_audio';
    } else {
        $info = jpegInfo($got['tmp']);
        $problem = match (true) {
            $info === null => 'That isn\'t a JPEG picture.',
            $info['width'] !== $info['height'] => 'The cover must be square.',
            $info['width'] < 256 || $info['width'] > 1024 => 'The cover must be 256 to 1024 pixels wide.',
            default => null,
        };
        $code = 'bad_cover';
    }
    if ($problem !== null) {
        @unlink($got['tmp']);
        refuse(400, $code, $problem);
    }
    $stored = storedName($name, $got['sha256']);
    $record = ['file' => $stored, 'bytes' => $got['bytes'], 'sha256' => $got['sha256']] + $info;
    if (is_file("$dir/$stored")) {
        @unlink($got['tmp']);
    } elseif (!rename($got['tmp'], "$dir/$stored")) {
        @unlink($got['tmp']);
        throw new RuntimeException("can't store $name");
    }
    for ($try = 0; $try < 8; $try++) {
        $current = row('SELECT status, rev, files FROM ws_versions WHERE id = ?', [$v['id']]);
        $files = $current ? jsonArray($current['files']) : [];
        if (!$current || $current['status'] !== 'draft') {
            if (!in_array($stored, array_column($files, 'file'), true)) @unlink("$dir/$stored");
            @rmdir($dir);
            refuse(409, 'not_draft', 'Only a draft can be changed.');
        }
        $old = $files[$name]['file'] ?? null;
        $files[$name] = $record;
        $bytes = array_sum(array_map(fn($f) => (int)$f['bytes'], $files));
        $changed = sql("UPDATE ws_versions SET files = ?, bytes = ?, rev = rev + 1 WHERE id = ? AND status = 'draft' AND rev = ?", [jsonText($files), $bytes, $v['id'], $current['rev']])->rowCount();
        if ($changed === 1) {
            if ($old !== null && $old !== $stored) @unlink("$dir/$old");
            unset($record['file']);
            out(['file' => ['url' => fileUrl($item['id'], (int)$number, $name)] + $record]);
        }
    }
    throw new RuntimeException('upload kept losing the race for ' . $v['id']);
}

function apiSubmit(string $slug, string $number): never
{
    [$who, $item] = ownItem($slug);
    $v = versionOf($item['id'], (int)$number);
    if (!$v) refuse(404, 'not_found', 'There is no such version.');
    if ($v['status'] !== 'draft') refuse(409, 'not_draft', 'Only a draft can be sent for review.');
    rateLimit('edit', $who['id']);
    $meta = jsonArray($v['meta']);
    $gap = missing($item, $meta, jsonArray($v['files']));
    if ($gap === null && $item['kind'] === 'sound' && !is_file(filesDir($item['id'], (int)$number) . '/' . (jsonArray($v['files'])['audio.flac']['file'] ?? '-'))) $gap = ['audio.flac', 'Add the audio.'];
    if ($gap !== null) refuse(409, 'incomplete', $gap[1], ['field' => $gap[0]]);
    if ($item['kind'] === 'pack') {
        foreach ($meta['sounds'] as $id) {
            $s = row('SELECT * FROM ws_items WHERE id = ?', [$id]);
            $ready = $s && $s['kind'] === 'sound' && $s['owner'] === $who['id']
                && ($s['status'] === 'public' || value("SELECT 1 FROM ws_versions WHERE item = ? AND status = 'review'", [$id]));
            if (!$ready) refuse(409, 'incomplete', 'A pack can only hold your own sounds that are published or in review.', ['field' => 'sounds', 'sound' => $id]);
        }
    }
    $changed = sql("UPDATE ws_versions SET status = 'review', submitted = ?, rev = rev + 1 WHERE id = ? AND status = 'draft' AND rev = ?", [now(), $v['id'], $v['rev']])->rowCount();
    if ($changed !== 1) refuse(409, 'changed', 'This version changed while it was being sent. Try again.');
    audit($who['id'], 'submit', "{$item['slug']} v$number");
    out(['version' => versionOut($item, versionOf($item['id'], (int)$number), true)]);
}

function apiWithdraw(string $slug, string $number): never
{
    [$who, $item] = ownItem($slug);
    $v = versionOf($item['id'], (int)$number);
    if (!$v) refuse(404, 'not_found', 'There is no such version.');
    if ($v['status'] !== 'review') refuse(409, 'not_review', 'That version isn\'t waiting for review.');
    sql("UPDATE ws_versions SET status = 'draft', submitted = NULL, rev = rev + 1 WHERE id = ? AND status = 'review'", [$v['id']]);
    out(['version' => versionOut($item, versionOf($item['id'], (int)$number), true)]);
}

function publicItem(string $slug): array
{
    $item = itemBySlug($slug);
    if (!$item || $item['status'] !== 'public') refuse(404, 'not_found', 'There is no such item.');
    return $item;
}

function apiSubscribe(string $slug): never
{
    $who = signedIn();
    $item = publicItem($slug);
    rateLimit('subscribe', $who['id']);
    try {
        sql('INSERT INTO ws_subs (account, item, since) VALUES (?, ?, ?)', [$who['id'], $item['id'], now()]);
    } catch (PDOException $e) {
        if (!duplicate($e)) throw $e;
    }
    recount($item['id']);
    out(['subscribed' => true, 'subs' => (int)value('SELECT subs FROM ws_items WHERE id = ?', [$item['id']])]);
}

function apiUnsubscribe(string $slug): never
{
    $who = signedIn();
    rateLimit('subscribe', $who['id']);
    $item = itemBySlug($slug);
    if ($item && sql('DELETE FROM ws_subs WHERE account = ? AND item = ?', [$who['id'], $item['id']])->rowCount() > 0) recount($item['id']);
    out(['subscribed' => false]);
}

function apiMySubs(): never
{
    $who = viewer();
    if ($who === null) refuse(401, 'signed_out', 'Sign in to Hexagon Cloud first.');
    $list = rows("SELECT i.* FROM ws_subs s JOIN ws_items i ON i.id = s.item WHERE s.account = ? AND i.status = 'public' ORDER BY s.since", [$who['id']]);
    out(['items' => array_map(fn($i) => itemOut($i, null), $list)]);
}

function apiMyItems(): never
{
    $who = viewer();
    if ($who === null) refuse(401, 'signed_out', 'Sign in to Hexagon Cloud first.');
    $list = rows("SELECT * FROM ws_items WHERE owner = ? AND status <> 'removed' ORDER BY updated DESC", [$who['id']]);
    out(['items' => array_map(fn($i) => itemOut($i, $who), $list)]);
}

function apiVote(string $slug): never
{
    $who = signedIn();
    $item = publicItem($slug);
    if ($item['owner'] === $who['id']) refuse(409, 'own_item', 'You can\'t rate your own item.');
    $in = body();
    $value = $in['value'] ?? null;
    if (!in_array($value, [-1, 0, 1], true)) bad('value', 'Vote 1, -1 or 0.');
    rateLimit('vote', $who['id']);
    atomic(function () use ($who, $item, $value) {
        if ($value === 0) {
            sql('DELETE FROM ws_votes WHERE account = ? AND item = ?', [$who['id'], $item['id']]);
            return;
        }
        if (sql('UPDATE ws_votes SET value = ?, at = ? WHERE account = ? AND item = ?', [$value, now(), $who['id'], $item['id']])->rowCount() > 0) return;
        try {
            sql('INSERT INTO ws_votes (account, item, value, at) VALUES (?, ?, ?, ?)', [$who['id'], $item['id'], $value, now()]);
        } catch (PDOException $e) {
            if (!duplicate($e)) throw $e;
            sql('UPDATE ws_votes SET value = ?, at = ? WHERE account = ? AND item = ?', [$value, now(), $who['id'], $item['id']]);
        }
    });
    recount($item['id']);
    $r = row('SELECT up, down FROM ws_items WHERE id = ?', [$item['id']]);
    out(['vote' => $value, 'up' => (int)$r['up'], 'down' => (int)$r['down']]);
}

function apiReport(string $slug): never
{
    $who = signedIn();
    $item = publicItem($slug);
    $in = body();
    $reason = $in['reason'] ?? null;
    if (!is_string($reason) || !in_array($reason, REPORT_REASONS, true)) bad('reason', 'Pick a reason.');
    $text = cleanText($in['text'] ?? '', 500, true);
    if ($text === null) bad('text', 'Keep it under 500 characters.');
    rateLimit('report', $who['id'], 'That is a lot of reports for one day.');
    $open = value('SELECT id FROM ws_reports WHERE item = ? AND account = ? AND closed IS NULL', [$item['id'], $who['id']]);
    if ($open !== null) {
        sql('UPDATE ws_reports SET reason = ?, text = ?, created = ? WHERE id = ?', [$reason, $text, now(), $open]);
    } else {
        sql('INSERT INTO ws_reports (id, item, account, reason, text, created, closed, closed_by) VALUES (?, ?, ?, ?, ?, ?, NULL, NULL)',
            [newId('r-'), $item['id'], $who['id'], $reason, $text, now()]);
    }
    out(['ok' => true]);
}

function apiQueue(): never
{
    $who = moderator();
    $list = rows("SELECT v.*, i.slug AS i_slug FROM ws_versions v JOIN ws_items i ON i.id = v.item WHERE v.status = 'review' AND i.status <> 'removed' ORDER BY v.submitted LIMIT 200");
    $out = [];
    foreach ($list as $v) {
        $item = itemBySlug($v['i_slug']);
        $entry = ['item' => itemOut($item, null), 'version' => versionOut($item, $v, true)];
        if ($item['kind'] === 'pack') $entry['sounds'] = packSounds(jsonArray($v['meta']), false);
        $out[] = $entry;
    }
    out(['queue' => $out]);
}

function apiDecide(string $versionId): never
{
    $who = moderator();
    $in = body();
    $decision = $in['decision'] ?? null;
    if ($decision !== 'accept' && $decision !== 'decline') bad('decision', 'Accept or decline.');
    $rev = $in['rev'] ?? null;
    if (!is_int($rev)) bad('rev', 'Reload the queue and try again.');
    $note = cleanText($in['note'] ?? '', 1000, true);
    if ($note === null) bad('note', 'Keep the note under 1000 characters.');
    if ($decision === 'decline' && $note === '') bad('note', 'Say why it was declined.');
    $v = row('SELECT * FROM ws_versions WHERE id = ?', [$versionId]);
    if (!$v) refuse(404, 'not_found', 'There is no such version.');
    $item = row('SELECT * FROM ws_items WHERE id = ?', [$v['item']]);
    if (!$item || $item['status'] === 'removed') refuse(404, 'not_found', 'There is no such item.');
    if ($v['status'] !== 'review') refuse(409, 'not_review', 'That version isn\'t waiting for review.');
    if ((int)$v['rev'] !== $rev) refuse(409, 'changed', 'The creator changed this version after the queue was loaded. Reload it.');
    $now = now();
    $number = (int)$v['number'];
    if ($decision === 'decline') {
        $changed = sql("UPDATE ws_versions SET status = 'declined', decided = ?, decided_by = ?, decision_note = ?, files = '{}', bytes = 0, rev = rev + 1 WHERE id = ? AND status = 'review' AND rev = ?",
            [$now, $who['id'], $note, $v['id'], $rev])->rowCount();
        if ($changed !== 1) refuse(409, 'changed', 'This version changed. Reload the queue.');
        dropDir(filesDir($item['id'], $number));
        audit($who['id'], 'decline', "{$item['slug']} v$number");
        out(['version' => versionOut($item, versionOf($item['id'], $number), true)]);
    }
    if ((int)value('SELECT banned FROM ws_people WHERE id = ?', [$item['owner']]) === 1) refuse(409, 'owner_banned', 'The creator is banned.');
    $meta = jsonArray($v['meta']);
    if ($item['kind'] === 'pack') {
        foreach ($meta['sounds'] ?? [] as $id) {
            $sound = row("SELECT owner, status FROM ws_items WHERE id = ? AND kind = 'sound'", [$id]);
            if (!$sound || $sound['owner'] !== $item['owner']) refuse(409, 'foreign_sound', 'A pack can only hold its creator\'s own sounds.', ['sound' => $id]);
            if ($sound['status'] !== 'public') refuse(409, 'sounds_pending', 'Accept the sounds in this pack first.', ['sound' => $id]);
        }
    }
    try {
        $replaced = atomic(function () use ($item, $v, $meta, $who, $note, $now, $number, $rev) {
            $changed = sql("UPDATE ws_versions SET status = 'accepted', decided = ?, decided_by = ?, decision_note = ?, rev = rev + 1 WHERE id = ? AND status = 'review' AND rev = ?",
                [$now, $who['id'], $note, $v['id'], $rev])->rowCount();
            if ($changed !== 1) throw new Conflict();
            $changed = sql("UPDATE ws_items SET current_version = ?, status = CASE WHEN status = 'new' THEN 'public' ELSE status END, title = ?, summary = ?, tags = ?, published = COALESCE(published, ?), updated = ? WHERE id = ? AND status <> 'removed' AND (current_version IS NULL OR current_version < ?)",
                [$number, $meta['title'], $meta['summary'] ?? '', storedTags($meta['tags'] ?? []), $now, $now, $item['id'], $number])->rowCount();
            if ($changed !== 1) throw new Conflict();
            $old = array_map('intval', array_column(rows("SELECT number FROM ws_versions WHERE item = ? AND status = 'accepted' AND number <> ?", [$item['id'], $number]), 'number'));
            sql("UPDATE ws_versions SET status = 'replaced', files = '{}', bytes = 0 WHERE item = ? AND status = 'accepted' AND number <> ?", [$item['id'], $number]);
            return $old;
        });
    } catch (Conflict $e) {
        refuse(409, 'changed', 'This version or its item changed. Reload the queue.');
    }
    foreach ($replaced as $old) dropDir(filesDir($item['id'], $old));
    audit($who['id'], 'accept', "{$item['slug']} v$number");
    $item = row('SELECT * FROM ws_items WHERE id = ?', [$item['id']]);
    out(['version' => versionOut($item, versionOf($item['id'], $number), true), 'item' => itemOut($item, $who)]);
}

function apiModItem(string $slug): never
{
    $who = moderator();
    $item = itemBySlug($slug);
    if (!$item || $item['status'] === 'removed') refuse(404, 'not_found', 'There is no such item.');
    $in = body();
    $action = $in['action'] ?? null;
    $note = cleanText($in['note'] ?? '', 1000, true) ?? '';
    if ($action === 'hide') {
        if ($item['status'] !== 'public') refuse(409, 'not_public', 'That item isn\'t public.');
        sql("UPDATE ws_items SET status = 'hidden', updated = ? WHERE id = ?", [now(), $item['id']]);
    } elseif ($action === 'unhide') {
        if ($item['status'] !== 'hidden' || $item['current_version'] === null) refuse(409, 'not_hidden', 'That item isn\'t hidden.');
        if ((int)value('SELECT banned FROM ws_people WHERE id = ?', [$item['owner']]) === 1) refuse(409, 'owner_banned', 'The creator is banned.');
        if ($item['kind'] === 'pack' && publicSounds(jsonArray(versionOf($item['id'], (int)$item['current_version'])['meta'] ?? null)) === 0) refuse(409, 'empty_pack', 'None of this pack\'s sounds are public.');
        sql("UPDATE ws_items SET status = 'public', updated = ? WHERE id = ? AND status = 'hidden'", [now(), $item['id']]);
    } elseif ($action === 'remove') {
        purgeItem($item, true);
    } else {
        bad('action', 'Hide, unhide or remove.');
    }
    if ($item['kind'] === 'sound') settlePacks();
    audit($who['id'], $action, trim("{$item['slug']} $note"));
    $item = itemBySlug($slug);
    out(['item' => $item['status'] === 'removed' ? ['slug' => $slug, 'status' => 'removed'] : itemOut($item, $who)]);
}

function apiReports(): never
{
    moderator();
    $list = rows('SELECT r.*, i.slug AS i_slug, i.title AS i_title, i.status AS i_status FROM ws_reports r JOIN ws_items i ON i.id = r.item WHERE r.closed IS NULL ORDER BY r.created LIMIT 200');
    out(['reports' => array_map(fn($r) => [
        'id' => $r['id'],
        'item' => ['slug' => $r['i_slug'], 'title' => $r['i_title'], 'status' => $r['i_status']],
        'by' => ['id' => $r['account'], 'name' => personName($r['account'])],
        'reason' => $r['reason'],
        'text' => $r['text'],
        'created' => (int)$r['created'],
    ], $list)]);
}

function apiCloseReport(string $id): never
{
    $who = moderator();
    $r = row('SELECT * FROM ws_reports WHERE id = ?', [$id]);
    if (!$r) refuse(404, 'not_found', 'There is no such report.');
    sql('UPDATE ws_reports SET closed = ?, closed_by = ? WHERE id = ? AND closed IS NULL', [now(), $who['id'], $id]);
    audit($who['id'], 'close_report', $id);
    out(['ok' => true]);
}

function apiModAccount(string $account): never
{
    $who = moderator();
    $in = body();
    $action = $in['action'] ?? null;
    if ($action !== 'ban' && $action !== 'unban') bad('action', 'Ban or unban.');
    if (isMod($account)) refuse(409, 'moderator', 'Moderators can\'t be banned here.');
    if (!value('SELECT 1 FROM ws_people WHERE id = ?', [$account])) {
        try {
            sql('INSERT INTO ws_people (id, name, seen, banned) VALUES (?, ?, ?, 0)', [$account, 'Someone', 0]);
        } catch (PDOException $e) {
            if (!duplicate($e)) throw $e;
        }
    }
    if ($action === 'ban') {
        atomic(function () use ($account) {
            sql('UPDATE ws_people SET banned = 1 WHERE id = ?', [$account]);
            sql("UPDATE ws_items SET status = 'hidden', updated = ? WHERE owner = ? AND status = 'public'", [now(), $account]);
            sql("UPDATE ws_versions SET status = 'draft', submitted = NULL, rev = rev + 1 WHERE status = 'review' AND item IN (SELECT id FROM ws_items WHERE owner = ?)", [$account]);
        });
        settlePacks();
    } else {
        sql('UPDATE ws_people SET banned = 0 WHERE id = ?', [$account]);
    }
    audit($who['id'], $action, $account);
    out(['ok' => true]);
}

function serveFile(string $itemId, string $number, string $name): never
{
    $item = row('SELECT * FROM ws_items WHERE id = ?', [$itemId]);
    $v = $item ? versionOf($itemId, (int)$number) : null;
    if (!$item || !$v || $item['status'] === 'removed') refuse(404, 'not_found', 'There is no such file.');
    $public = $v['status'] === 'accepted' && $item['status'] === 'public' && (int)$item['current_version'] === (int)$number;
    if (!$public) {
        $who = viewer();
        if ($who === null || ($who['id'] !== $item['owner'] && !$who['mod'])) refuse(404, 'not_found', 'There is no such file.');
    }
    $record = jsonArray($v['files'])[$name] ?? null;
    $stored = is_array($record) && is_string($record['file'] ?? null) && preg_match('/^(audio|cover)\.[0-9a-f]{16}\.(flac|jpg)$/D', $record['file']) ? $record['file'] : null;
    $path = filesDir($itemId, (int)$number) . "/$stored";
    if ($stored === null || !is_file($path)) refuse(404, 'not_found', 'There is no such file.');
    $files = [$name => $record];
    sendFile($path, $name === 'audio.flac' ? 'audio/flac' : 'image/jpeg', (string)$files[$name]['sha256'], $public, $name);
}
