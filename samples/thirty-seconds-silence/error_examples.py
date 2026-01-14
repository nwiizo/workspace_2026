"""
エラーメッセージの例

物語「三十秒の沈黙」に登場するエラーメッセージを再現するコード。
これらのエラーに対して「三十秒考える」ことで、根本原因を理解できる。
"""


def example_attribute_error():
    """
    AttributeError: 'NoneType' object has no attribute 'items'

    原因: orderがNoneなのにorder.itemsにアクセスしようとしている
    対策: Noneチェックを事前に行う、またはOptionalを明示する
    """

    class Order:
        def __init__(self, items):
            self.items = items

    class Item:
        def __init__(self, price, quantity):
            self.price = price
            self.quantity = quantity

    def get_order(order_id):
        # 存在しないorder_idの場合、Noneを返す
        if order_id == 999:
            return None
        return Order([Item(100, 2), Item(200, 1)])

    def process_order(order_id):
        order = get_order(order_id)
        # ここでorderがNoneの場合にAttributeErrorが発生
        total = sum(item.price * item.quantity for item in order.items)
        return total

    # 正常系
    print(f"Order 1 total: {process_order(1)}")

    # 異常系 - AttributeError発生
    try:
        process_order(999)
    except AttributeError as e:
        print(f"Error: {e}")


def example_key_error():
    """
    KeyError: 'user_id'

    原因: APIのレスポンス形式がエラー時と正常時で異なる
    対策: dict.get()を使うか、事前にキーの存在チェックをする
    """

    def get_api_response(success=True):
        if success:
            return {"user_id": 12345, "name": "Taro"}
        else:
            return {"error": "User not found", "code": 404}

    def get_user_id(response):
        # エラー時のレスポンスには'user_id'がない
        return response["user_id"]

    def get_user_id_safe(response):
        # 安全な方法: dict.get()を使う
        return response.get("user_id")

    # 正常系
    print(f"User ID: {get_user_id(get_api_response(success=True))}")

    # 異常系 - KeyError発生
    try:
        get_user_id(get_api_response(success=False))
    except KeyError as e:
        print(f"Error: KeyError {e}")

    # 安全な方法
    user_id = get_user_id_safe(get_api_response(success=False))
    print(f"User ID (safe): {user_id}")  # None


def example_type_error():
    """
    TypeError: unsupported operand type(s) for +: 'int' and 'str'

    原因: 型が混在している。入力値のバリデーション不足、または型変換忘れ
    対策: 型ヒント、Pydanticによるバリデーション、isinstance()チェック
    """

    def calculate_total(price, quantity):
        # quantityが文字列として渡されるとTypeError
        return price * quantity

    def calculate_total_safe(price: int, quantity: int) -> int:
        # 型変換を行う
        return int(price) * int(quantity)

    # 正常系
    print(f"Total: {calculate_total(100, 2)}")

    # 異常系 - TypeError発生
    try:
        result = calculate_total(100, "2") + calculate_total(200, "1")
        print(result)
    except TypeError as e:
        print(f"Error: {e}")

    # 安全な方法
    result = calculate_total_safe(100, "2") + calculate_total_safe(200, "1")
    print(f"Total (safe): {result}")


def example_runtime_error_asyncio():
    """
    RuntimeError: asyncio.run() cannot be called from a running event loop

    原因: 非同期コンテキスト内でasyncio.run()を呼んでいる
    対策: awaitを使う、またはloop.run_until_complete()に変える
    """
    import asyncio

    async def fetch_data():
        await asyncio.sleep(0.1)
        return "data"

    def sync_wrapper():
        # すでにイベントループが動いている場合、これはエラーになる
        return asyncio.run(fetch_data())

    async def async_wrapper():
        # 正しい方法: awaitを使う
        return await fetch_data()

    # 通常のコンテキストでは動作する
    result = asyncio.run(fetch_data())
    print(f"Result: {result}")

    # 非同期コンテキスト内での正しい使い方
    async def main():
        result = await async_wrapper()
        print(f"Result (async): {result}")

    asyncio.run(main())


if __name__ == "__main__":
    print("=== AttributeError Example ===")
    example_attribute_error()
    print()

    print("=== KeyError Example ===")
    example_key_error()
    print()

    print("=== TypeError Example ===")
    example_type_error()
    print()

    print("=== RuntimeError (asyncio) Example ===")
    example_runtime_error_asyncio()
