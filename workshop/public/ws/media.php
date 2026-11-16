<?php

declare(strict_types=1);

function filesDir(string $item, int $number): string
{
    return dataDir() . "/files/$item/$number";
}

function dropDir(string $dir): void
{
    if (!is_dir($dir)) return;
    foreach (scandir($dir) ?: [] as $name) {
        if ($name === '.' || $name === '..') continue;
        $path = "$dir/$name";
        is_dir($path) && !is_link($path) ? dropDir($path) : @unlink($path);
    }
    @rmdir($dir);
}

function ownerLock(string $account): mixed
{
    $dir = dataDir() . '/locks';
    if (!is_dir($dir)) @mkdir($dir, 0700, true);
    $lock = @fopen("$dir/" . substr(hash('sha256', $account), 0, 24) . '.lock', 'c');
    if (!$lock) return null;
    if (!flock($lock, LOCK_EX | LOCK_NB)) refuse(409, 'busy', 'Another upload is still running. Try again when it finishes.');
    return $lock;
}

function storedName(string $name, string $sha256): string
{
    $dot = strrpos($name, '.');
    return substr($name, 0, $dot) . '.' . substr($sha256, 0, 16) . substr($name, $dot);
}

function receiveUpload(string $dir, string $name, int $max): array
{
    $length = $_SERVER['CONTENT_LENGTH'] ?? $_SERVER['HTTP_CONTENT_LENGTH'] ?? null;
    if (!is_string($length) || !preg_match('/^\d{1,12}$/D', $length)) refuse(411, 'length_required', 'Send the file size with the upload.');
    $length = (int)$length;
    if ($length === 0) refuse(400, 'empty', 'The file is empty.');
    if ($length > $max) refuse(413, 'too_big', 'The file is too big. The limit is ' . round($max / 1048576) . ' MB.', ['max' => $max]);
    if (!is_dir($dir) && !mkdir($dir, 0700, true) && !is_dir($dir)) throw new RuntimeException("can't make $dir");
    $tmp = "$dir/.$name." . bin2hex(random_bytes(6)) . '.part';
    $in = fopen('php://input', 'rb');
    $out = fopen($tmp, 'xb');
    if (!$in || !$out) throw new RuntimeException('can\'t open the upload');
    $hash = hash_init('sha256');
    $bytes = 0;
    $written = true;
    while (!feof($in)) {
        $chunk = fread($in, 65536);
        if ($chunk === false || $chunk === '') break;
        $bytes += strlen($chunk);
        if ($bytes > $length) break;
        hash_update($hash, $chunk);
        if (fwrite($out, $chunk) !== strlen($chunk)) {
            $written = false;
            break;
        }
    }
    fclose($in);
    $written = fclose($out) && $written;
    if (!$written) {
        @unlink($tmp);
        throw new RuntimeException('can\'t write the upload');
    }
    if ($bytes !== $length) {
        @unlink($tmp);
        refuse(400, 'short_upload', 'The upload didn\'t match its size. Try again.');
    }
    return ['tmp' => $tmp, 'bytes' => $bytes, 'sha256' => hash_final($hash)];
}

function flacInfo(string $path): ?array
{
    $f = fopen($path, 'rb');
    if (!$f) return null;
    $head = fread($f, 42);
    fclose($f);
    if (!is_string($head) || strlen($head) < 42 || substr($head, 0, 4) !== 'fLaC') return null;
    if ((ord($head[4]) & 0x7f) !== 0) return null;
    $blockLength = (ord($head[5]) << 16) | (ord($head[6]) << 8) | ord($head[7]);
    if ($blockLength !== 34) return null;
    $maxBlock = (ord($head[10]) << 8) | ord($head[11]);
    $packed = unpack('J', substr($head, 18, 8))[1];
    $rate = ($packed >> 44) & 0xfffff;
    $channels = (($packed >> 41) & 0x7) + 1;
    $bits = (($packed >> 36) & 0x1f) + 1;
    $samples = $packed & 0xfffffffff;
    if ($maxBlock < 16 || $rate <= 0 || $samples <= 0) return null;
    return ['rate' => $rate, 'channels' => $channels, 'bits' => $bits, 'samples' => $samples, 'seconds' => round($samples / $rate, 3)];
}

function jpegInfo(string $path): ?array
{
    $f = fopen($path, 'rb');
    if (!$f) return null;
    $data = fread($f, 262144);
    fclose($f);
    if (!is_string($data) || strlen($data) < 4 || substr($data, 0, 3) !== "\xFF\xD8\xFF") return null;
    $pos = 2;
    $length = strlen($data);
    while ($pos + 9 < $length) {
        if (ord($data[$pos]) !== 0xFF) return null;
        while ($pos < $length && ord($data[$pos]) === 0xFF) $pos++;
        if ($pos >= $length) return null;
        $marker = ord($data[$pos]);
        $pos++;
        if ($marker === 0xD8 || ($marker >= 0xD0 && $marker <= 0xD7) || $marker === 0x01) continue;
        if ($marker === 0xD9 || $marker === 0xDA) return null;
        if ($pos + 2 > $length) return null;
        $segment = (ord($data[$pos]) << 8) | ord($data[$pos + 1]);
        if ($segment < 2) return null;
        if (in_array($marker, [0xC0, 0xC1, 0xC2, 0xC3, 0xC5, 0xC6, 0xC7, 0xC9, 0xCA, 0xCB, 0xCD, 0xCE, 0xCF], true)) {
            if ($pos + 7 > $length) return null;
            $height = (ord($data[$pos + 3]) << 8) | ord($data[$pos + 4]);
            $width = (ord($data[$pos + 5]) << 8) | ord($data[$pos + 6]);
            return $width > 0 && $height > 0 ? ['width' => $width, 'height' => $height] : null;
        }
        $pos += $segment;
    }
    return null;
}

function sendFile(string $path, string $type, string $etag, bool $public, string $name): never
{
    $size = filesize($path);
    if ($size === false) refuse(404, 'not_found', 'There is no such file.');
    header_remove('Cache-Control');
    header('Content-Type: ' . $type);
    header('Accept-Ranges: bytes');
    header('ETag: "' . $etag . '"');
    header($public ? 'Cache-Control: public, no-cache' : 'Cache-Control: private, no-store');
    header("Content-Security-Policy: default-src 'none'; sandbox");
    header('Content-Disposition: inline; filename="' . $name . '"');
    $match = (string)($_SERVER['HTTP_IF_NONE_MATCH'] ?? '');
    if ($match !== '' && in_array('"' . $etag . '"', array_map('trim', explode(',', $match)), true)) {
        http_response_code(304);
        exit;
    }
    $start = 0;
    $end = $size - 1;
    $range = (string)($_SERVER['HTTP_RANGE'] ?? '');
    if ($range !== '') {
        if (!preg_match('/^bytes=(\d{0,15})-(\d{0,15})$/D', $range, $m) || ($m[1] === '' && $m[2] === '')) {
            header("Content-Range: bytes */$size");
            http_response_code(416);
            exit;
        }
        if ($m[1] === '') {
            $start = max(0, $size - (int)$m[2]);
        } else {
            $start = (int)$m[1];
            if ($m[2] !== '') $end = min($end, (int)$m[2]);
        }
        if ($start > $end || $start >= $size) {
            header("Content-Range: bytes */$size");
            http_response_code(416);
            exit;
        }
        http_response_code(206);
        header("Content-Range: bytes $start-$end/$size");
    } else {
        http_response_code(200);
    }
    $length = $end - $start + 1;
    header('Content-Length: ' . $length);
    if (($_SERVER['REQUEST_METHOD'] ?? 'GET') === 'HEAD') exit;
    $f = fopen($path, 'rb');
    if (!$f) exit;
    fseek($f, $start);
    while ($length > 0 && !feof($f)) {
        $chunk = fread($f, (int)min(65536, $length));
        if ($chunk === false || $chunk === '') break;
        echo $chunk;
        $length -= strlen($chunk);
        flush();
    }
    fclose($f);
    exit;
}
