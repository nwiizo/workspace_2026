"""
注文処理サービス（バグあり版）

物語「三十秒の沈黙」に登場するレースコンディションのあるコード。
calculate_total（非同期）とapply_coupon（同期）の間で競合が発生する。

問題のシナリオ:
───────────────────────────────────────────────────────────
時刻    calculate_total (非同期)     apply_coupon (同期)
───────────────────────────────────────────────────────────
T1      order取得 (total=0)
T2                                    order取得 (total=0)
T3                                    割引計算 (-500円)
T4                                    total = 0 - 500 = -500 ← おかしい！
T5                                    save()
T6      total計算 (10000円)
T7      total = 10000
T8      save() ← apply_couponの結果を上書き！
───────────────────────────────────────────────────────────
"""

from dataclasses import dataclass, field
from decimal import Decimal
from typing import Optional
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

    def save(self):
        # 実際にはDBに保存する
        print(f"  [DB] Order {self.id} saved: total={self.total}")


# シンプルなインメモリストア（DBの代わり）
orders_db: dict[int, Order] = {}


def create_order(order_id: int, items: list[Item]) -> Order:
    """注文を作成する"""
    order = Order(id=order_id, items=items)
    orders_db[order_id] = order
    order.save()
    return order


def get_order(order_id: int) -> Optional[Order]:
    """DBから注文を取得する（実際にはORMのget()）"""
    return orders_db.get(order_id)


# === 問題のコード ===


def calculate_total(order_id: int):
    """
    価格計算（Celeryの非同期タスクとして実行されることがある）

    問題点:
    - ロックを取得していない
    - 他の処理と並行して実行される可能性がある
    """
    print(f"[calculate_total] Start for order {order_id}")
    order = get_order(order_id)
    if order is None:
        return

    # 時間のかかる処理をシミュレート
    time.sleep(0.1)

    total = sum(
        item.price * item.quantity for item in order.items
    )
    print(f"[calculate_total] Calculated total: {total}")

    # さらに時間がかかる
    time.sleep(0.1)

    order.total = total
    order.save()
    print(f"[calculate_total] Done for order {order_id}")


def apply_coupon(order_id: int, coupon_code: str):
    """
    クーポン適用（同期処理）

    問題点:
    - order.totalがまだ計算されていない可能性がある
    - calculate_totalと並行して実行されると結果が上書きされる
    """
    print(f"[apply_coupon] Start for order {order_id}, coupon: {coupon_code}")
    order = get_order(order_id)
    if order is None:
        return

    # クーポンによる割引（簡略化）
    discount = Decimal("500")
    print(f"[apply_coupon] Current total: {order.total}, discount: {discount}")

    order.total = order.total - discount  # ← totalが0の場合、マイナスになる！
    order.save()
    print(f"[apply_coupon] Done for order {order_id}")


def demonstrate_race_condition():
    """レースコンディションを再現する"""
    print("=== レースコンディションのデモ ===\n")

    # 注文を作成
    items = [
        Item(price=Decimal("3000"), quantity=2),
        Item(price=Decimal("4000"), quantity=1),
    ]
    order = create_order(order_id=1, items=items)
    print(f"Order created: id={order.id}, items={len(order.items)}, total={order.total}\n")

    # 期待される結果: 3000*2 + 4000*1 - 500 = 9500

    # 非同期タスクとクーポン適用を並行して実行
    t1 = threading.Thread(target=calculate_total, args=(1,))
    t2 = threading.Thread(target=apply_coupon, args=(1, "SAVE500"))

    print("Starting concurrent operations...\n")
    t1.start()
    time.sleep(0.05)  # 少し遅れてクーポン適用
    t2.start()

    t1.join()
    t2.join()

    print(f"\n=== 結果 ===")
    final_order = get_order(1)
    print(f"Final total: {final_order.total}")
    print(f"Expected total: 9500")
    print(f"Bug occurred: {final_order.total != Decimal('9500')}")

    if final_order.total == Decimal("10000"):
        print("→ クーポン適用が上書きされた（割引が適用されていない）")
    elif final_order.total < 0:
        print("→ totalが0の状態で割引が適用された（マイナスになった）")


if __name__ == "__main__":
    demonstrate_race_condition()
