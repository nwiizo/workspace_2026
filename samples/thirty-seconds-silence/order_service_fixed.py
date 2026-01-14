"""
注文処理サービス（修正版）

物語「三十秒の沈黙」に登場するレースコンディションを修正したコード。
トランザクションとselect_for_updateでロックを取得し、競合を防ぐ。
"""

from dataclasses import dataclass, field
from decimal import Decimal
from typing import Optional
from contextlib import contextmanager
import threading
import time


@dataclass
class Item:
    price: Decimal
    quantity: int


@dataclass
class Order:
    id: int
    items: list[Item] = field(default_factory=list)
    total: Decimal = Decimal("0")
    _locked: bool = False

    def save(self):
        print(f"  [DB] Order {self.id} saved: total={self.total}")


# シンプルなインメモリストア（DBの代わり）
orders_db: dict[int, Order] = {}
db_lock = threading.Lock()


def create_order(order_id: int, items: list[Item]) -> Order:
    """注文を作成する"""
    order = Order(id=order_id, items=items)
    orders_db[order_id] = order
    order.save()
    return order


def get_order(order_id: int) -> Optional[Order]:
    """DBから注文を取得する"""
    return orders_db.get(order_id)


def get_order_for_update(order_id: int) -> Optional[Order]:
    """
    DBから注文を取得する（行ロック付き）
    実際のDjangoでは: Order.objects.select_for_update().get(id=order_id)
    """
    order = orders_db.get(order_id)
    if order:
        order._locked = True
    return order


@contextmanager
def atomic():
    """
    トランザクションをシミュレート
    実際のDjangoでは: with transaction.atomic():
    """
    db_lock.acquire()
    try:
        yield
    finally:
        db_lock.release()


def calculate_discount(order: Order, coupon_code: str) -> Decimal:
    """クーポンによる割引を計算する"""
    # 実際にはクーポンコードに応じた割引ロジック
    if coupon_code == "SAVE500":
        return Decimal("500")
    elif coupon_code == "SAVE10PCT":
        base = sum(item.price * item.quantity for item in order.items)
        return base * Decimal("0.1")
    return Decimal("0")


# === 修正後のコード ===


def process_order_with_coupon(order_id: int, coupon_code: Optional[str] = None):
    """
    価格計算とクーポン適用を一つのトランザクションで実行

    修正のポイント:
    1. transaction.atomic()でトランザクションを開始
    2. select_for_update()で行ロックを取得
    3. 価格計算とクーポン適用を一つの処理にまとめる
    """
    print(f"[process_order] Start for order {order_id}, coupon: {coupon_code}")

    with atomic():
        order = get_order_for_update(order_id)
        if order is None:
            print(f"[process_order] Order {order_id} not found")
            return

        # 価格計算
        base_total = sum(
            item.price * item.quantity
            for item in order.items
        )
        print(f"[process_order] Base total: {base_total}")

        # 時間のかかる処理をシミュレート
        time.sleep(0.1)

        # クーポン適用
        if coupon_code:
            discount = calculate_discount(order, coupon_code)
            order.total = base_total - discount
            print(f"[process_order] Applied discount: {discount}")
        else:
            order.total = base_total

        order.save()
        print(f"[process_order] Done for order {order_id}, total: {order.total}")


def demonstrate_fixed_version():
    """修正版のデモ"""
    print("=== 修正版のデモ ===\n")

    # 注文を作成
    items = [
        Item(price=Decimal("3000"), quantity=2),
        Item(price=Decimal("4000"), quantity=1),
    ]
    order = create_order(order_id=1, items=items)
    print(f"Order created: id={order.id}, items={len(order.items)}, total={order.total}\n")

    # 期待される結果: 3000*2 + 4000*1 - 500 = 9500

    # 並行して実行しても安全
    t1 = threading.Thread(
        target=process_order_with_coupon,
        args=(1, None)
    )
    t2 = threading.Thread(
        target=process_order_with_coupon,
        args=(1, "SAVE500")
    )

    print("Starting concurrent operations (with locking)...\n")
    t1.start()
    time.sleep(0.05)
    t2.start()

    t1.join()
    t2.join()

    print(f"\n=== 結果 ===")
    final_order = get_order(1)
    print(f"Final total: {final_order.total}")

    # 最後に実行された処理の結果が反映される（上書きではなく正しい順序で処理）
    print("→ ロックにより、処理が順序正しく実行された")


if __name__ == "__main__":
    demonstrate_fixed_version()
