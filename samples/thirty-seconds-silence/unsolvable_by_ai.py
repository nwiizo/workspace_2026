"""
生成AIが解決できない問題の例

この問題は以下の理由でAIには解決が困難:

1. **コードとログが別々のコンテキスト**
   - ログだけ渡されても、コードの構造が分からない
   - コードだけ渡されても、実行時の状態が分からない

2. **暗黙の設定・インフラ依存**
   - 設定ファイルの内容がコードに現れない
   - 環境変数やインフラの状態が関係

3. **時間・状態依存**
   - 特定の時刻、特定の負荷でのみ発生
   - キャッシュやコネクションプールの状態に依存

4. **複数システムの相互作用**
   - ログに出ていない別サービスが原因
   - ネットワーク経路の問題

---

シナリオ: 毎週月曜日の朝9時頃だけAPIレスポンスが遅くなる

ログ（AIに渡される情報）:
```
2024-01-08 09:02:15 WARNING: Slow query detected: 3.2s
2024-01-08 09:02:18 ERROR: Request timeout after 5s
2024-01-08 09:02:22 WARNING: Slow query detected: 4.1s
2024-01-08 09:02:25 ERROR: Request timeout after 5s
```

AIの分析:
「クエリが遅いのでインデックスを追加してください」
「タイムアウト値を増やしてください」
→ 根本原因を見逃している

真の原因（AIには見えない）:
- Celeryのスケジュールタスクが月曜9時に週次レポートを生成
- そのタスクがDBコネクションを大量に消費
- 通常のAPIリクエストがコネクション待ちでタイムアウト
"""

import threading
import time
from dataclasses import dataclass
from datetime import datetime
from queue import Queue, Empty
import logging
from typing import Any

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(levelname)s: %(message)s"
)
logger = logging.getLogger(__name__)


# === インフラ層（AIには見えにくい部分） ===


@dataclass
class DBConnection:
    id: int
    in_use: bool = False


class ConnectionPool:
    """DBコネクションプール（簡略化）"""

    def __init__(self, max_connections: int = 5):
        self.max_connections = max_connections
        self.connections: list[DBConnection] = [
            DBConnection(id=i) for i in range(max_connections)
        ]
        self.lock = threading.Lock()
        self.wait_queue: Queue = Queue()

    def acquire(self, timeout: float = 5.0) -> DBConnection | None:
        """コネクションを取得"""
        start = time.time()

        while time.time() - start < timeout:
            with self.lock:
                for conn in self.connections:
                    if not conn.in_use:
                        conn.in_use = True
                        return conn

            # 空きがなければ待機
            time.sleep(0.1)

        return None  # タイムアウト

    def release(self, conn: DBConnection):
        """コネクションを解放"""
        with self.lock:
            conn.in_use = False

    def get_stats(self) -> dict:
        with self.lock:
            in_use = sum(1 for c in self.connections if c.in_use)
            return {
                "max": self.max_connections,
                "in_use": in_use,
                "available": self.max_connections - in_use
            }


# グローバルなコネクションプール
db_pool = ConnectionPool(max_connections=5)


# === アプリケーション層（AIに渡されるコード） ===


def api_handler(request_id: int) -> dict[str, Any]:
    """
    APIハンドラー

    このコードだけ見ても問題は分からない。
    一見、正常に見える。
    """
    start = time.time()

    # DBコネクションを取得
    conn = db_pool.acquire(timeout=5.0)

    if conn is None:
        logger.error(f"Request {request_id}: Request timeout after 5s")
        return {"error": "timeout", "request_id": request_id}

    try:
        # クエリ実行（シミュレート）
        time.sleep(0.1)  # 通常は100ms程度

        elapsed = time.time() - start
        if elapsed > 1.0:
            logger.warning(f"Slow query detected: {elapsed:.1f}s")

        return {"status": "ok", "request_id": request_id}
    finally:
        db_pool.release(conn)


# === バックグラウンドジョブ（別ファイル、別リポジトリにあるかも） ===


def weekly_report_task():
    """
    週次レポート生成タスク

    このタスクは別のCeleryワーカーで動いている。
    設定ファイルで月曜9時に実行されるようスケジュールされている。

    問題: 複数のDBコネクションを長時間占有する
    """
    logger.info("[Weekly Report] Starting...")

    # 複数のコネクションを取得して長時間保持
    connections = []
    for i in range(5):  # 5個全てを占有！
        conn = db_pool.acquire(timeout=30.0)
        if conn:
            connections.append(conn)
            logger.info(f"[Weekly Report] Acquired connection {conn.id}")

    # 重い処理をシミュレート（10秒）
    for i in range(10):
        time.sleep(1)
        logger.info(f"[Weekly Report] Processing... {i+1}/10")

    # コネクションを解放
    for conn in connections:
        db_pool.release(conn)

    logger.info("[Weekly Report] Done")


# === デモ ===


def simulate_monday_morning():
    """
    月曜朝9時の状況をシミュレート

    - 週次レポートタスクが起動
    - 同時にAPIリクエストが来る
    - コネクション枯渇でタイムアウト発生
    """
    print("=" * 60)
    print("シミュレーション: 月曜朝9時の状況")
    print("=" * 60)
    print()

    # 週次レポートタスクをバックグラウンドで開始
    report_thread = threading.Thread(target=weekly_report_task)
    report_thread.start()

    # レポートタスクがコネクションを確保するまで待つ
    time.sleep(1.0)

    print("\n--- APIリクエスト開始 ---\n")

    # 複数のAPIリクエストを発行
    def make_request(req_id):
        result = api_handler(req_id)
        if result.get("error"):
            print(f"  Request {req_id}: FAILED - {result['error']}")
        else:
            print(f"  Request {req_id}: OK")

    threads = []
    for i in range(5):
        t = threading.Thread(target=make_request, args=(i,))
        threads.append(t)
        t.start()
        time.sleep(0.2)

    # 全てのスレッドを待機
    for t in threads:
        t.join()

    report_thread.join()

    print()
    print("=" * 60)
    print("分析")
    print("=" * 60)
    print("""
AIに渡される情報:
- api_handler()のコード
- "Slow query detected" "Request timeout" のログ

AIの推測:
- 「クエリが遅い → インデックスを追加」
- 「タイムアウト → タイムアウト値を増やす」
- 「コネクションプールを増やす」

実際の原因:
- 週次レポートタスクがコネクションを占有
- Celeryの設定ファイルに月曜9時の実行スケジュール
- この情報はapi_handler()のコードにもログにも現れない

解決策:
- 週次レポートタスクを別のコネクションプールで実行
- または、実行時間をオフピークにずらす
- または、コネクション使用数を制限
""")


def simulate_normal_day():
    """
    通常日の状況をシミュレート

    週次レポートタスクが動いていない日は問題なし
    """
    print("=" * 60)
    print("シミュレーション: 通常日（火曜〜日曜）")
    print("=" * 60)
    print()

    print("--- APIリクエスト開始 ---\n")

    def make_request(req_id):
        result = api_handler(req_id)
        if result.get("error"):
            print(f"  Request {req_id}: FAILED - {result['error']}")
        else:
            print(f"  Request {req_id}: OK")

    threads = []
    for i in range(5):
        t = threading.Thread(target=make_request, args=(i,))
        threads.append(t)
        t.start()
        time.sleep(0.1)

    for t in threads:
        t.join()

    print("\n→ 全てのリクエストが正常に処理された")


if __name__ == "__main__":
    import sys
    # 月曜朝だけシミュレート（問題を確実に再現）
    print("\n" + "=" * 60, flush=True)
    print("生成AIが解決できない問題のデモ", flush=True)
    print("=" * 60 + "\n", flush=True)

    simulate_monday_morning()
