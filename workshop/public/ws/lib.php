<?php

declare(strict_types=1);

const WS_APP = 'nuru';
const TAGS = ['nature', 'weather', 'water', 'fire', 'life', 'urban', 'transit', 'indoor', 'people', 'noise'];
const LICENCES = ['CC0-1.0', 'CC-BY-4.0', 'CC-BY-SA-4.0', 'CC-PDM-1.0', 'OWN-WORK'];
const CREDIT_NEEDED = ['CC-BY-4.0', 'CC-BY-SA-4.0'];
const COZY = [
    'none' => [],
    'weather' => ['rain', 'downpour', 'storm', 'snow', 'wind'],
    'window' => ['forest', 'city', 'beach', 'hills'],
    'sky' => ['night', 'morning', 'dusk'],
    'hearth' => null,
    'life' => null,
    'transit' => null,
];
const KIND_FILES = ['sound' => ['audio.flac', 'cover.jpg'], 'pack' => ['cover.jpg']];
const REPORT_REASONS = ['licence', 'offensive', 'broken', 'spam', 'other'];
const ACCOUNT_ID = '/^a-[a-z2-7]{16}$/D';

final class Unavailable extends RuntimeException
{
}

function dataDir(): string
{
    $dir = getenv('NURU_WS_DATA');
    return is_string($dir) && $dir !== '' ? rtrim($dir, '/\\') : dirname(__DIR__) . '/ws-data';
}

function config(): ?array
{
    static $config;
    if ($config === null) {
        $file = dataDir() . '/config.php';
        $config = is_file($file) ? require $file : [];
    }
    return is_array($config) && $config ? $config : null;
}

function setting(string $key, mixed $default): mixed
{
    return config()[$key] ?? $default;
}

function out(array $data, int $status = 200): never
{
    http_response_code($status);
    header('Content-Type: application/json; charset=utf-8');
    echo json_encode($data, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_INVALID_UTF8_SUBSTITUTE);
    exit;
}

function refuse(int $status, string $code, string $message, array $extra = []): never
{
    out(['error' => $code, 'message' => $message] + $extra, $status);
}

function start(): void
{
    ini_set('display_errors', '0');
    header('Cache-Control: no-store');
    header('X-Content-Type-Options: nosniff');
    header('Referrer-Policy: no-referrer');
    set_exception_handler(function (Throwable $e) {
        if ($e instanceof Unavailable) refuse(503, 'unavailable', $e->getMessage());
        error_log('workshop: ' . get_class($e) . ': ' . $e->getMessage() . ' at ' . basename($e->getFile()) . ':' . $e->getLine());
        refuse(500, 'server_error', 'Server error. Try again in a moment.');
    });
    if (!config()) throw new Unavailable('The Workshop isn\'t set up yet.');
}

function body(): array
{
    $raw = file_get_contents('php://input', false, null, 0, 262144);
    $data = is_string($raw) && trim($raw) !== '' ? json_decode($raw, true) : [];
    if (!is_array($data)) refuse(400, 'bad_json', 'The request isn\'t JSON.');
    return $data;
}

function address(): string
{
    return (string)($_SERVER['REMOTE_ADDR'] ?? '');
}

function now(): int
{
    return time();
}


function db(): PDO
{
    static $pdo;
    if ($pdo) return $pdo;
    $c = setting('db', []);
    $dsn = is_array($c) ? (string)($c['dsn'] ?? '') : '';
    $driver = strtolower((string)strstr($dsn, ':', true));
    if ($driver !== 'sqlite' && $driver !== 'mysql') throw new Unavailable('The Workshop database isn\'t set up.');
    if ($driver === 'sqlite' && !is_dir(dirname(substr($dsn, 7)))) mkdir(dirname(substr($dsn, 7)), 0700, true);
    if ($driver === 'mysql' && !preg_match('/;\s*charset=/i', $dsn)) $dsn .= ';charset=utf8mb4';
    try {
        $pdo = new PDO($dsn, $c['user'] ?? null, $c['password'] ?? null, [
            PDO::ATTR_ERRMODE => PDO::ERRMODE_EXCEPTION,
            PDO::ATTR_DEFAULT_FETCH_MODE => PDO::FETCH_ASSOC,
            PDO::ATTR_TIMEOUT => 5,
        ]);
    } catch (PDOException $e) {
        error_log('workshop: database: ' . $e->getMessage());
        throw new Unavailable('The Workshop can\'t be reached right now. Try again in a few minutes.');
    }
    if ($driver === 'sqlite') $pdo->exec('PRAGMA busy_timeout = 5000');
    try {
        $pdo->query('SELECT v.rev FROM ws_audit a, ws_versions v WHERE 1 = 0');
    } catch (PDOException $e) {
        $schema = (string)file_get_contents(__DIR__ . "/schema.$driver.sql");
        foreach (explode(';', $schema) as $statement) if (trim($statement) !== '') $pdo->exec($statement);
    }
    return $pdo;
}

function sql(string $query, array $args = []): PDOStatement
{
    $s = db()->prepare($query);
    $s->execute($args);
    return $s;
}

function row(string $query, array $args = []): ?array
{
    $r = sql($query, $args)->fetch();
    return is_array($r) ? $r : null;
}

function rows(string $query, array $args = []): array
{
    return sql($query, $args)->fetchAll();
}

function value(string $query, array $args = []): mixed
{
    $v = sql($query, $args)->fetchColumn();
    return $v === false ? null : $v;
}

function duplicate(PDOException $e): bool
{
    return (string)$e->getCode() === '23000' || str_contains($e->getMessage(), 'UNIQUE');
}

function atomic(callable $work): mixed
{
    $pdo = db();
    $pdo->beginTransaction();
    try {
        $result = $work();
        $pdo->commit();
        return $result;
    } catch (Throwable $e) {
        $pdo->rollBack();
        throw $e;
    }
}

function tidy(): void
{
    if (random_int(1, 50) !== 1) return;
    sql('DELETE FROM ws_limits WHERE window_start < ?', [now() - 86400 * 2]);
}

function audit(?string $account, string $action, string $detail = ''): void
{
    sql('INSERT INTO ws_audit (account, action, detail, at) VALUES (?, ?, ?, ?)', [$account, $action, mb_substr($detail, 0, 2000), now()]);
}


function hit(string $bucket, string $who, int $max, int $window): int
{
    $id = substr(hash_hmac('sha256', "$bucket\n$who", (string)setting('secret', '')), 0, 40);
    $now = now();
    $r = row('SELECT count, window_start FROM ws_limits WHERE id = ?', [$id]);
    if (!$r || $now - (int)$r['window_start'] >= $window) {
        sql('DELETE FROM ws_limits WHERE id = ?', [$id]);
        try {
            sql('INSERT INTO ws_limits (id, count, window_start) VALUES (?, 1, ?)', [$id, $now]);
        } catch (PDOException $e) {
            if (!duplicate($e)) throw $e;
        }
        return 0;
    }
    if ((int)$r['count'] >= $max) return max(1, $window - ($now - (int)$r['window_start']));
    sql('UPDATE ws_limits SET count = count + 1 WHERE id = ?', [$id]);
    return 0;
}

function rateLimit(string $bucket, string $who, string $what = 'Too many tries.'): void
{
    $limits = setting('limits', []) + [
        'create' => [30, 86400],
        'upload' => [200, 3600],
        'edit' => [300, 3600],
        'subscribe' => [600, 3600],
        'vote' => [300, 3600],
        'report' => [20, 86400],
        'moderate' => [2000, 3600],
    ];
    [$max, $window] = $limits[$bucket] ?? [100, 3600];
    $wait = hit($bucket, $who, (int)$max, (int)$window);
    if (!$wait) return;
    header("Retry-After: $wait");
    refuse(429, 'too_many', "$what Try again in " . ($wait < 120 ? "$wait seconds" : ceil($wait / 60) . ' minutes') . '.', ['retry' => $wait]);
}


function b64url(string $bytes): string
{
    return rtrim(strtr(base64_encode($bytes), '+/', '-_'), '=');
}

function unb64url(string $text): ?string
{
    if (!preg_match('/^[A-Za-z0-9_-]*$/D', $text)) return null;
    $bytes = base64_decode(str_pad(strtr($text, '-_', '+/'), (int)ceil(strlen($text) / 4) * 4, '='), true);
    return $bytes === false ? null : $bytes;
}

function newId(string $prefix): string
{
    $id = $prefix;
    foreach (str_split(random_bytes(16)) as $b) $id .= 'abcdefghijklmnopqrstuvwxyz234567'[ord($b) & 31];
    return $id;
}

function jsonText(mixed $value): string
{
    return json_encode($value, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_INVALID_UTF8_SUBSTITUTE);
}

function jsonArray(?string $text): array
{
    $v = $text === null ? null : json_decode($text, true);
    return is_array($v) ? $v : [];
}


function ticketHeader(): ?string
{
    $t = $_SERVER['HTTP_X_HEXAGON_TICKET'] ?? null;
    return is_string($t) && $t !== '' ? $t : null;
}

function ticketClaims(string $ticket): ?array
{
    if (strlen($ticket) > 2000 || !function_exists('sodium_crypto_sign_verify_detached')) return null;
    $key = base64_decode((string)setting('ticket_public_key', ''), true);
    if (!is_string($key) || strlen($key) !== SODIUM_CRYPTO_SIGN_PUBLICKEYBYTES) return null;
    $parts = explode('.', $ticket);
    if (count($parts) !== 2) return null;
    $payload = unb64url($parts[0]);
    $signature = unb64url($parts[1]);
    if ($payload === null || $signature === null || strlen($signature) !== SODIUM_CRYPTO_SIGN_BYTES) return null;
    try {
        if (!sodium_crypto_sign_verify_detached($signature, $payload, $key)) return null;
    } catch (Throwable $e) {
        return null;
    }
    $t = json_decode($payload, true);
    $now = now();
    if (!is_array($t) || ($t['v'] ?? null) !== 1 || ($t['app'] ?? null) !== WS_APP) return null;
    if (!is_int($t['exp'] ?? null) || $t['exp'] <= $now || !is_int($t['iat'] ?? null) || $t['iat'] > $now + 300) return null;
    if (!is_string($t['sub'] ?? null) || !preg_match(ACCOUNT_ID, $t['sub'])) return null;
    $name = is_string($t['name'] ?? null) ? trim(preg_replace('/[\x00-\x1f\x7f]+/u', ' ', $t['name']) ?? '') : '';
    return ['id' => $t['sub'], 'name' => $name !== '' ? mb_substr($name, 0, 64) : 'Someone'];
}

function isMod(?string $account): bool
{
    return $account !== null && in_array($account, (array)setting('moderators', []), true);
}

function viewer(): ?array
{
    static $done = false, $who = null;
    if ($done) return $who;
    $done = true;
    $ticket = ticketHeader();
    if ($ticket === null) return null;
    $claims = ticketClaims($ticket);
    if ($claims === null) refuse(401, 'invalid_ticket', 'Sign in again.');
    $person = row('SELECT * FROM ws_people WHERE id = ?', [$claims['id']]);
    $now = now();
    if (!$person) {
        try {
            sql('INSERT INTO ws_people (id, name, seen, banned) VALUES (?, ?, ?, 0)', [$claims['id'], $claims['name'], $now]);
        } catch (PDOException $e) {
            if (!duplicate($e)) throw $e;
        }
        $person = ['id' => $claims['id'], 'name' => $claims['name'], 'banned' => 0];
    } elseif ($person['name'] !== $claims['name'] || $now - (int)$person['seen'] > 600) {
        sql('UPDATE ws_people SET name = ?, seen = ? WHERE id = ?', [$claims['name'], $now, $claims['id']]);
    }
    $who = ['id' => $claims['id'], 'name' => $claims['name'], 'banned' => (bool)(int)$person['banned'], 'mod' => isMod($claims['id'])];
    return $who;
}

function signedIn(): array
{
    $who = viewer();
    if ($who === null) refuse(401, 'signed_out', 'Sign in to Hexagon Cloud first.');
    if ($who['banned']) refuse(403, 'banned', 'This account can\'t make changes in the Workshop.');
    return $who;
}

function moderator(): array
{
    $who = signedIn();
    if (!$who['mod']) refuse(403, 'forbidden', 'Only moderators can do that.');
    rateLimit('moderate', $who['id']);
    return $who;
}

function personName(string $id): string
{
    static $names = [];
    if (!array_key_exists($id, $names)) $names[$id] = (string)(value('SELECT name FROM ws_people WHERE id = ?', [$id]) ?? 'Someone');
    return $names[$id];
}
