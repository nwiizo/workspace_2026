"""
悪い例外処理の例

物語「三十秒の沈黙」に登場する、レビューで指摘されたコード。
例外を握りつぶすと、障害時の調査が困難になる。
"""

import logging

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


class ExternalAPI:
    """外部APIのモック"""

    def __init__(self, should_fail=False):
        self.should_fail = should_fail

    def call(self):
        if self.should_fail:
            raise ConnectionError("External API is down")
        return {"status": "ok", "data": [1, 2, 3]}


# === 悪い例 ===


def bad_api_call():
    """
    悪い例: 例外を握りつぶしている

    問題点:
    - 例外を捕捉しているが何もしていない
    - ログも出していない
    - 障害時に何が起きたか分からない
    """
    external_api = ExternalAPI(should_fail=True)
    try:
        result = external_api.call()
        return result
    except Exception:
        pass  # ← これ


def bad_api_call_v2():
    """
    悪い例2: 例外を握りつぶしてNoneを返す

    問題点:
    - Noneが返ってきても、なぜNoneなのか分からない
    - 正常系でNoneを返すケースと区別がつかない
    """
    external_api = ExternalAPI(should_fail=True)
    try:
        result = external_api.call()
        return result
    except Exception:
        return None  # 正常系のNoneと区別がつかない


def bad_api_call_v3():
    """
    悪い例3: 広すぎる例外捕捉

    問題点:
    - Exception全体を捕捉している
    - プログラムのバグ（TypeError, KeyErrorなど）も握りつぶされる
    """
    external_api = ExternalAPI(should_fail=False)
    try:
        result = external_api.call()
        # 意図しないバグ
        return result["nonexistent_key"]  # KeyError!
    except Exception:
        return {"status": "error"}  # バグなのにエラーとして処理される


# === 良い例 ===


def good_api_call():
    """
    良い例: 適切な例外処理

    ポイント:
    - 特定の例外のみ捕捉
    - ログを出力
    - 適切なリトライまたはフォールバック
    """
    external_api = ExternalAPI(should_fail=True)
    try:
        result = external_api.call()
        return result
    except ConnectionError as e:
        logger.error(f"External API call failed: {e}")
        # 必要に応じてリトライロジックを入れる
        raise  # または適切なカスタム例外を投げる


def good_api_call_with_fallback():
    """
    良い例: フォールバック値を返す場合

    ポイント:
    - ログを出力
    - フォールバック値であることが分かる
    - 呼び出し元でハンドリングできる
    """
    external_api = ExternalAPI(should_fail=True)
    try:
        result = external_api.call()
        return result
    except ConnectionError as e:
        logger.warning(f"External API unavailable, using fallback: {e}")
        return {"status": "fallback", "data": [], "error": str(e)}


class APIError(Exception):
    """API呼び出しエラーを表すカスタム例外"""

    def __init__(self, message: str, original_error: Exception | None = None):
        super().__init__(message)
        self.original_error = original_error


def good_api_call_with_custom_exception():
    """
    良い例: カスタム例外でラップ

    ポイント:
    - 元の例外を保持
    - 呼び出し元で適切にハンドリングできる
    - スタックトレースが失われない
    """
    external_api = ExternalAPI(should_fail=True)
    try:
        result = external_api.call()
        return result
    except ConnectionError as e:
        logger.error(f"External API call failed: {e}")
        raise APIError("Failed to fetch data from external API", e) from e


if __name__ == "__main__":
    print("=== 悪い例のデモ ===\n")

    print("1. bad_api_call():")
    result = bad_api_call()
    print(f"   Result: {result}")  # None - 何が起きたか分からない

    print("\n2. bad_api_call_v2():")
    result = bad_api_call_v2()
    print(f"   Result: {result}")  # None - 正常系と区別がつかない

    print("\n3. bad_api_call_v3():")
    result = bad_api_call_v3()
    print(f"   Result: {result}")  # バグがエラーとして処理される

    print("\n=== 良い例のデモ ===\n")

    print("4. good_api_call_with_fallback():")
    result = good_api_call_with_fallback()
    print(f"   Result: {result}")  # フォールバックであることが分かる

    print("\n5. good_api_call_with_custom_exception():")
    try:
        good_api_call_with_custom_exception()
    except APIError as e:
        print(f"   Caught APIError: {e}")
        print(f"   Original error: {e.original_error}")
