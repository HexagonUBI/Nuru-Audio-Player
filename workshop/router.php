<?php

$path = (string)(parse_url((string)($_SERVER['REQUEST_URI'] ?? '/'), PHP_URL_PATH) ?? '/');
$plain = strtolower(str_replace('\\', '/', rawurldecode($path)));
$plain = (string)preg_replace('#[. ]+(?=/|$)#', '', $plain);
$public = realpath(__DIR__ . '/public');
$real = realpath(__DIR__ . '/public' . rawurldecode($path));
$hidden = preg_match('#^/+(ws|ws-data)(/|$)#', $plain)
    || ($real !== false && (str_starts_with($real, $public . DIRECTORY_SEPARATOR . 'ws') || !str_starts_with($real, $public)));
if ($hidden) {
    http_response_code(403);
    exit;
}
if ($real !== false && $path !== '/' && is_file($real) && !str_ends_with(strtolower($real), '.php')) return false;
require __DIR__ . '/public/index.php';
