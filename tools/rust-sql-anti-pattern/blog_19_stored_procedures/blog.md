# ストアドプロシージャは本当に避けるべきか：使い所の見極め方

## 常識への疑問

「ストアドプロシージャはメンテナンス性が悪い。ビジネスロジックはアプリケーション側に書くべき」

これは広く信じられている常識だ。ORMの普及とともに、ストアドプロシージャは時代遅れとみなされるようになった。

しかし本当にそうだろうか。監査ログ、大量データ処理、複雑な制約……アプリケーション側で書くと煩雑になるケースがある。本記事では「ストアドプロシージャを避けるべき」という常識を一度疑い、使い所を見極める基準を示す。

## PostgreSQLの関数とプロシージャ

まず用語を整理する。PostgreSQLには2種類の「ストアドルーチン」がある。

### FUNCTION（関数）

- 値を返す
- SELECT文の中で呼び出せる
- トランザクション制御ができない

```sql
CREATE OR REPLACE FUNCTION add_numbers(a INT, b INT) RETURNS INT AS $$
BEGIN
    RETURN a + b;
END;
$$ LANGUAGE plpgsql;
```

```rust
let result: i32 = sqlx::query_scalar("SELECT add_numbers(10, 20)")
    .fetch_one(&pool).await?;
// result = 30
```

### PROCEDURE（プロシージャ）

- PostgreSQL 11以降
- 値を返さない（OUTパラメータは可能）
- `CALL`で呼び出す
- トランザクション制御ができる（COMMIT/ROLLBACK）

```sql
CREATE OR REPLACE PROCEDURE bulk_update_prices(updates JSONB) AS $$
DECLARE
    item JSONB;
BEGIN
    FOR item IN SELECT * FROM jsonb_array_elements(updates) LOOP
        UPDATE products
        SET price = (item->>'new_price')::DECIMAL
        WHERE id = (item->>'product_id')::UUID;
    END LOOP;
END;
$$ LANGUAGE plpgsql;
```

```rust
sqlx::query("CALL bulk_update_prices($1::jsonb)")
    .bind(updates_json)
    .execute(&pool).await?;
```

## 使うべきケース1：監査ログ

データの変更履歴を自動で記録する。アプリケーション側で実装すると「書き忘れ」が起きる。トリガーなら100%捕捉できる。

```sql
CREATE TABLE audit_log (
    id BIGSERIAL PRIMARY KEY,
    table_name TEXT NOT NULL,
    record_id UUID NOT NULL,
    action TEXT NOT NULL,  -- INSERT, UPDATE, DELETE
    old_values JSONB,
    new_values JSONB,
    changed_by UUID,
    changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE OR REPLACE FUNCTION audit_trigger() RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO audit_log (table_name, record_id, action, old_values, new_values, changed_by)
    VALUES (
        TG_TABLE_NAME,
        COALESCE(NEW.id, OLD.id),
        TG_OP,
        CASE WHEN TG_OP IN ('UPDATE', 'DELETE') THEN to_jsonb(OLD) END,
        CASE WHEN TG_OP IN ('INSERT', 'UPDATE') THEN to_jsonb(NEW) END,
        NULLIF(current_setting('app.current_user_id', TRUE), '')::UUID
    );
    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER orders_audit
    AFTER INSERT OR UPDATE OR DELETE ON orders
    FOR EACH ROW EXECUTE FUNCTION audit_trigger();
```

```rust
// セッション変数でユーザーIDを設定
let mut tx = pool.begin().await?;

sqlx::query(&format!("SET LOCAL app.current_user_id = '{}'", user_id))
    .execute(&mut *tx).await?;

// 注文を更新（トリガーが自動でログを記録）
sqlx::query("UPDATE orders SET total = 200 WHERE id = $1")
    .bind(order_id)
    .execute(&mut *tx).await?;

tx.commit().await?;

// 監査ログを確認
let logs: Vec<AuditLog> = sqlx::query_as(
    "SELECT * FROM audit_log WHERE record_id = $1 ORDER BY id"
)
.bind(order_id)
.fetch_all(&pool).await?;
```

`current_setting('app.current_user_id', TRUE)`でセッション変数を取得する。Webフレームワークのミドルウェアで設定しておけば、全クエリに自動でユーザーIDが記録される。

## 使うべきケース2：複雑な制約

CHECK制約で表現できない複雑なビジネスルールをトリガーで実装する。

```sql
CREATE OR REPLACE FUNCTION enforce_positive_stock() RETURNS TRIGGER AS $$
BEGIN
    IF NEW.stock < 0 THEN
        RAISE EXCEPTION 'Stock cannot be negative: got %', NEW.stock;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER check_stock_trigger
    BEFORE UPDATE ON products
    FOR EACH ROW
    EXECUTE FUNCTION enforce_positive_stock();
```

```rust
// 正常な更新
sqlx::query("UPDATE products SET stock = 5 WHERE id = $1")
    .bind(product_id)
    .execute(&pool).await?;  // OK

// 在庫をマイナスにしようとする
let result = sqlx::query("UPDATE products SET stock = -1 WHERE id = $1")
    .bind(product_id)
    .execute(&pool).await;

match result {
    Ok(_) => unreachable!(),
    Err(e) => {
        assert!(e.to_string().contains("Stock cannot be negative"));
    }
}
```

アプリケーション側でバリデーションしても、直接SQLを実行されると制約を回避される。トリガーならデータベースレベルで強制できる。

## 使うべきケース3：派生カラムの自動更新

注文明細が変わったら、注文の合計を自動更新する。

```sql
CREATE OR REPLACE FUNCTION update_order_total() RETURNS TRIGGER AS $$
BEGIN
    UPDATE orders
    SET total = (
        SELECT COALESCE(SUM(price * quantity), 0)
        FROM order_items
        WHERE order_id = COALESCE(NEW.order_id, OLD.order_id)
    )
    WHERE id = COALESCE(NEW.order_id, OLD.order_id);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER order_items_changed
    AFTER INSERT OR UPDATE OR DELETE ON order_items
    FOR EACH ROW
    EXECUTE FUNCTION update_order_total();
```

```rust
// 注文明細を追加（トリガーが合計を自動更新）
sqlx::query(
    "INSERT INTO order_items (order_id, product_id, quantity, price) VALUES ($1, $2, $3, $4)"
)
.bind(order_id)
.bind(product_id)
.bind(2)
.bind(Decimal::new(9999, 2))
.execute(&pool).await?;

// 合計が自動で更新されている
let total: Decimal = sqlx::query_scalar("SELECT total FROM orders WHERE id = $1")
    .bind(order_id)
    .fetch_one(&pool).await?;
```

## 使うべきケース4：大量データ処理

100万件の価格更新。Rustでループするとネットワークラウンドトリップが100万回発生する。

```rust
// ❌ 遅い：100万回のラウンドトリップ
for update in &price_updates {
    sqlx::query("UPDATE products SET price = $1 WHERE id = $2")
        .bind(update.new_price)
        .bind(update.product_id)
        .execute(&pool).await?;
}
```

ストアドプロシージャならサーバー内で完結する。

```sql
CREATE OR REPLACE PROCEDURE bulk_update_prices(updates JSONB) AS $$
DECLARE
    item JSONB;
BEGIN
    FOR item IN SELECT * FROM jsonb_array_elements(updates) LOOP
        UPDATE products
        SET price = (item->>'new_price')::DECIMAL
        WHERE id = (item->>'product_id')::UUID;
    END LOOP;
END;
$$ LANGUAGE plpgsql;
```

```rust
// ✅ 速い：1回のラウンドトリップ
let updates_json = serde_json::to_value(&price_updates)?;
sqlx::query("CALL bulk_update_prices($1::jsonb)")
    .bind(updates_json)
    .execute(&pool).await?;
```

ただし、このケースはUNNESTを使った一括UPDATEでも対応できる。

```rust
// ✅ これも速い：CTEで一括更新
sqlx::query(r#"
    WITH updates AS (
        SELECT * FROM jsonb_to_recordset($1::jsonb)
        AS t(product_id UUID, new_price DECIMAL)
    )
    UPDATE products p
    SET price = u.new_price
    FROM updates u
    WHERE p.id = u.product_id
"#)
.bind(&updates_json)
.execute(&pool).await?;
```

## 使うべきケース5：レポート生成

複雑な集計ロジックを関数にまとめる。

```sql
CREATE OR REPLACE FUNCTION get_sales_report(
    start_date DATE,
    end_date DATE
) RETURNS TABLE (
    product_id UUID,
    product_name TEXT,
    total_quantity BIGINT,
    total_revenue DECIMAL(12,2),
    avg_price DECIMAL(10,2)
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        p.id,
        p.name,
        COALESCE(SUM(oi.quantity)::BIGINT, 0::BIGINT),
        COALESCE(SUM(oi.price * oi.quantity), 0::DECIMAL(12,2)),
        COALESCE(AVG(oi.price), 0::DECIMAL(10,2))
    FROM products p
    LEFT JOIN order_items oi ON oi.product_id = p.id
    LEFT JOIN orders o ON oi.order_id = o.id
        AND o.created_at::DATE BETWEEN start_date AND end_date
    GROUP BY p.id, p.name
    ORDER BY COALESCE(SUM(oi.price * oi.quantity), 0) DESC;
END;
$$ LANGUAGE plpgsql;
```

```rust
let reports: Vec<SalesReport> = sqlx::query_as(
    "SELECT * FROM get_sales_report($1, $2)"
)
.bind(start_date)
.bind(end_date)
.fetch_all(&pool).await?;
```

`RETURNS TABLE`で複数行を返す関数を定義できる。SELECTの中で呼び出せるので、さらに条件を追加することもできる。

## 避けるべきケース

### ビジネスロジック

- ユーザー登録フロー
- 決済処理
- 外部API呼び出し

これらはアプリケーション側で書くべきだ。理由は以下。

1. **テストが困難**: モックやスタブを使いにくい
2. **デバッグが困難**: ブレークポイントを張れない
3. **バージョン管理**: マイグレーションとの整合性管理が煩雑
4. **外部依存**: HTTPクライアント、キャッシュなどが使えない

### 複雑な分岐

```sql
-- ❌ こういうのはアプリケーション側で
CREATE OR REPLACE FUNCTION process_order(...) AS $$
BEGIN
    IF ... THEN
        IF ... THEN
            CASE WHEN ... THEN
                -- 100行のネストしたロジック
```

複雑な分岐はRustの型システムとテストで管理すべきだ。

## 選択基準

```
ストアドを使うべきか？
├─ データの整合性に関わる → 使う
│   ├─ 監査ログ → トリガー
│   ├─ 複雑な制約 → トリガー
│   └─ 派生カラム → トリガー
├─ パフォーマンスが重要 → 検討する
│   ├─ 大量バッチ処理 → プロシージャまたはCTE
│   └─ 複雑な集計 → RETURNS TABLE関数
├─ ビジネスロジック → 使わない
│   ├─ 認証・認可 → アプリケーション
│   ├─ 外部API連携 → アプリケーション
│   └─ 複雑な分岐 → アプリケーション
└─ 迷ったら → 使わない（アプリケーション優先）
```

## OUTパラメータを持つ関数

複数の値を返したい場合、OUTパラメータを使う。

```sql
CREATE OR REPLACE FUNCTION get_user_stats(
    p_user_id UUID,
    OUT total_orders INT,
    OUT total_spent DECIMAL(12,2),
    OUT last_order_date DATE
) AS $$
BEGIN
    SELECT
        COUNT(*)::INT,
        COALESCE(SUM(total), 0),
        MAX(created_at)::DATE
    INTO total_orders, total_spent, last_order_date
    FROM orders
    WHERE user_id = p_user_id;
END;
$$ LANGUAGE plpgsql;
```

```rust
#[derive(Debug, sqlx::FromRow)]
struct UserStats {
    total_orders: i32,
    total_spent: Decimal,
    last_order_date: Option<NaiveDate>,
}

let stats: UserStats = sqlx::query_as("SELECT * FROM get_user_stats($1)")
    .bind(user_id)
    .fetch_one(&pool).await?;
```

## 「ストアドは避けるべき」の真意

冒頭の「ストアドプロシージャはメンテナンス性が悪い」は、部分的には正しい。ビジネスロジックをストアドに書くと確かにメンテナンスが困難になる。

しかし、データベースレベルで強制すべきルール（監査、制約、派生値）はトリガーで実装する方が確実だ。アプリケーション側で「書き忘れない」という規律に依存するより、仕組みで保証する方が安全だ。

「ストアドを避ける」のではなく「ストアドの適切な使い所を見極める」のが正しいアプローチだ。

## まとめ

ストアドプロシージャは「使わない」のではなく「使い分ける」ものだ。

**使うべきケース**
1. 監査ログ（トリガー）
2. 複雑な制約（トリガー）
3. 派生カラムの自動更新（トリガー）
4. 大量データ処理（プロシージャ）
5. 複雑な集計レポート（RETURNS TABLE関数）

**避けるべきケース**
1. ビジネスロジック
2. 外部API連携
3. 複雑な分岐

迷ったらアプリケーション側に書く。ただし「データベースレベルで強制したい」ルールはトリガーを検討する。

## 実行可能なデモコード

本記事のコードは以下で実行できる。

```sh
cd blog_19_stored_procedures
cargo run
```

## 参考資料

- [PostgreSQL - PL/pgSQL](https://www.postgresql.org/docs/current/plpgsql.html)
- [PostgreSQL - CREATE FUNCTION](https://www.postgresql.org/docs/current/sql-createfunction.html)
- [PostgreSQL - CREATE PROCEDURE](https://www.postgresql.org/docs/current/sql-createprocedure.html)
- [PostgreSQL - Trigger Functions](https://www.postgresql.org/docs/current/plpgsql-trigger.html)
