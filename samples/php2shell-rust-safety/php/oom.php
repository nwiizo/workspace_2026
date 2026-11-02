<?php

declare(strict_types=1);

// A bounded OOM observation, not a memory corruption or RCE exploit.
// Run in a disposable container with memory_limit=16M.
$mode = $argv[1] ?? 'oom';
if (!in_array($mode, ['ok', 'oom'], true)) {
    fwrite(STDERR, "usage: php oom.php [ok|oom]\n");
    exit(2);
}

printf("PHP %s, mode=%s, memory_limit=%s\n", PHP_VERSION, $mode, ini_get('memory_limit'));
register_shutdown_function(static function (): void {
    echo "shutdown callback\n";
});

echo "before allocation\n";
try {
    $chunks = [];
    $count = $mode === 'ok' ? 2 : 32;
    for ($index = 0; $index < $count; $index++) {
        $chunks[] = str_repeat('x', 1024 * 1024);
    }
    echo "allocation completed\n";
} finally {
    echo "finally block\n";
}
echo "after allocation\n";
