# Fear of the Unknown：Rust/sqlxでNULLを制する6つのパターン
## はじめに

あるプロジェクトで、電話番号が未登録のユーザーを検索するコードをレビューしていた。`WHERE phone = NULL`——一見正しく見えるこのクエリは、常に0件を返していた。データは確実に存在する。クエリもシンプル。では何が問題なのか。

答えはSQLの3値論理にあった。通常の比較演算はTRUEかFALSEを返すが、SQLにはUNKNOWN（不明）という第3の真偽値がある。NULLは「値が不明」を意味するため、`NULL = NULL`は「不明 = 不明」となり、結果もUNKNOWNになる。`WHERE`句はTRUEの行しか返さないから、UNKNOWNは暗黙にFALSE扱いされ、結果は常に0件になる。

この問題は『SQLアンチパターン』で「Fear of the Unknown」として解説されている。本記事ではRust + sqlxでの実装パターンに焦点を当てる。

こういう妄想の仕様と実際の仕様には違いがある。「おい、類推するな」というブログで書いたので時間がある時に読んでほしい。

[https://syu-m-5151.hatenablog.com/entry/2025/12/06/060208:embed:cite]

[asin:4814400748:detail]

## sqlxの型マッピング

Rustには`Option<T>`という型がある。これは「値があるかもしれないし、ないかもしれない」を表現する型だ。`Some(値)`が「値あり」、`None`が「値なし」を意味する。SQLのNULLに相当するのがこの`None`だ。

```rust
let phone: Option<String> = Some("090-1234-5678".to_string());  // 値あり
let phone: Option<String> = None;                                // 値なし（NULL相当）
```

sqlxはPostgreSQLのNULLをこの`Option<T>`に自動マッピングする。

| PostgreSQL | Rust (NULLable) | Rust (NOT NULL) |
|------------|-----------------|-----------------|
| VARCHAR, TEXT | Option\<String\> | String |
| INTEGER | Option\<i32\> | i32 |
| BIGINT | Option\<i64\> | i64 |
| UUID | Option\<Uuid\> | Uuid |
| DECIMAL | Option\<Decimal\> | Decimal |
| TIMESTAMPTZ | Option\<DateTime\<Utc\>\> | DateTime\<Utc\> |

NULLableカラムを`Option<T>`以外にマッピングすると、NULLが返された時点で実行時エラーになる。私も一度やった。「NULLなんて来ないだろう」と思っていたカラムが、特定の条件でNULLを返し、深夜にSlackが鳴った。

```rust
#[derive(Debug, sqlx::FromRow)]
struct User {
    id: Uuid,
    email: String,              // NOT NULL → 必ず値がある
    name: String,               // NOT NULL → 必ず値がある
    phone: Option<String>,      // NULLable → Option型で「値があるかもしれないし、ないかもしれない」を表現
    bio: Option<String>,        // NULLable → Noneが「値なし」、Some("値")が「値あり」
    created_at: DateTime<Utc>,  // NOT NULL → 必ず値がある
}
```

## パターン1：検索フィルターでのNULL

```rust
// NG: NoneがNULLにバインドされ、phone = NULLは常にUNKNOWN
// query_as::<_, User>の説明:
//   ::<_, User> は戻り値の型を指定するRustの記法（turbofish構文）
//   _ はデータベースの種類をコンパイラに推論させる部分
//   User は「検索結果をUser構造体に変換して」という指定
let users = sqlx::query_as::<_, User>(
    "SELECT * FROM users WHERE phone = $1"  // $1はプレースホルダ（SQLインジェクション対策）
)
.bind(&params.phone)  // bind()で$1に値を埋め込む。NoneはNULLになる
.fetch_all(&pool)     // 全件取得
.await?;              // 非同期処理の完了を待つ。?はエラー時に早期リターン
```

```rust
// OK: 条件分岐でクエリを切り替える
// match式: Option型の中身に応じて処理を分岐（switch文のようなもの）
let users = match &params.phone {
    Some(phone) => {  // Some(値): 値がある場合
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE phone = $1")
            .bind(phone)
            .fetch_all(&pool)
            .await?
    }
    None => {  // None: 値がない場合 → IS NULLを使う
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE phone IS NULL")
            .fetch_all(&pool)
            .await?
    }
};
```

```rust
// OK: IS NOT DISTINCT FROMで1クエリにまとめる（PostgreSQL固有）
// NULLを普通の値として比較できる（NULL同士も「等しい」と判定）
let users = sqlx::query_as::<_, User>(
    "SELECT * FROM users WHERE phone IS NOT DISTINCT FROM $1"
)
.bind(&params.phone)
.fetch_all(&pool)
.await?;
```

## パターン2：COUNTの挙動

```rust
// r#"..."# は生文字列リテラル（raw string literal）
// 複数行のSQLを書きやすく、エスケープも不要な記法
sqlx::query_as(
    r#"
    SELECT
        COUNT(*) as total_users,                           -- 全行数（NULLを含む）
        COUNT(coupon_code) as users_with_coupon,           -- NULLでない行数
        COUNT(*) - COUNT(coupon_code) as users_without_coupon
    FROM users
    "#
)
```

空文字列とNULLが混在している場合は注意が必要。

```rust
// NG: 空文字列のみマッチ、NULLはマッチしない
"SELECT * FROM users WHERE coupon_code = ''"

// OK: 両方を考慮
"SELECT * FROM users WHERE coupon_code IS NULL OR coupon_code = ''"

// OK: NULLIFで正規化
"SELECT * FROM users WHERE NULLIF(coupon_code, '') IS NULL"
```

## パターン3：フォーム送信での空文字列

フロントエンドから`{ "phone": "" }`が送られると、`Option<String>`では`Some("")`になる。データベースには空文字列が保存され、NULLにはならない。

```rust
// Rustレイヤーで正規化
// filter(): 条件を満たさない場合はNoneに変換するメソッド
// |s| !s.is_empty() はクロージャ（無名関数）: sが空でなければtrue
let phone = req.phone.filter(|s| !s.is_empty());  // Some("") → None, Some("090") → Some("090")
let bio = req.bio.filter(|s| !s.is_empty());

sqlx::query("UPDATE users SET phone = $1, bio = $2 WHERE id = $3")
    .bind(&phone)  // NoneはNULLとしてバインドされる
    .bind(&bio)
    .bind(user_id)
    .execute(&pool)  // execute(): SELECT以外のクエリ実行
    .await?;
```

```rust
// SQLレイヤーで正規化
sqlx::query(
    r#"
    UPDATE users
    SET phone = NULLIF(TRIM($1), ''),  -- TRIM: 空白除去, NULLIF: ''ならNULLに
        bio = NULLIF(TRIM($2), '')
    WHERE id = $3
    "#
)
```

## パターン4：LEFT JOINでのOption必須

LEFT JOINは左側のテーブル（例: users）の全行を返す。右側のテーブル（例: orders）に一致する行がない場合、右側のカラムはすべてNULLで埋められる。だから注文がないユーザーの場合、`o.created_at`はNULLになり、`MAX(o.created_at)`の結果もNULLになる。

```rust
// NG: 注文がないユーザーでMAX(o.created_at)がNULLになり、実行時エラー
struct UserWithLastOrder {
    last_order_date: DateTime<Utc>,  // NULLを受け付けない型
}

// OK: Option<T>でNULLを許容する
struct UserWithLastOrder {
    last_order_date: Option<DateTime<Utc>>,  // NULLならNone、値があればSome(値)
}
```

LEFT JOINや集約関数（MAX, AVG, SUM等）の結果は常にNULLになりうる。迷ったら`Option<T>`を使う。

## パターン5：NOT INの罠

```rust
// NG: category_idがNULLの行は削除されない
sqlx::query(
    r#"
    DELETE FROM products
    WHERE category_id NOT IN (
        SELECT id FROM categories WHERE active = true
    )
    "#
)
```

なぜNULLの行が削除されないのか。`NOT IN`は内部で`x <> 1 AND x <> 2 AND ...`に展開される。ここでcategory_idがNULLだとどうなるか。`NULL <> 1`はUNKNOWNを返す。`NULL <> 2`もUNKNOWN。ANDの3値論理では`TRUE AND UNKNOWN = UNKNOWN`だから、条件全体がUNKNOWNになる。WHERE句はTRUEの行しか処理しないため、NULLを含む行は削除対象から外れてしまう。

```rust
// OK: NOT EXISTSを使う
// NULLの行も正しく処理される（サブクエリが0行ならTRUE）
sqlx::query(
    r#"
    DELETE FROM products p
    WHERE NOT EXISTS (
        SELECT 1 FROM categories c
        WHERE c.id = p.category_id AND c.active = true
    )
    "#
)
```

## パターン6：query_as!マクロ

これまでのパターンで使っていた`query_as()`は実行時に型チェックを行う。一方`query_as!()`はマクロで、コンパイル時にデータベースへ接続してスキーマを確認し、型の不整合をビルドエラーとして検出する。NULLになりうるカラムを`Option<T>`以外でマッピングしようとすると、実行前にエラーを発見できる。

```rust
// NG: AVG(rating)はNULLを返す可能性があり、コンパイルエラー
struct ProductSummary {
    average_rating: f64,  // f64はNULLを受け付けない
}

sqlx::query_as!(
    ProductSummary,
    "SELECT name, AVG(rating) as average_rating FROM products GROUP BY name"
)
// コンパイルエラー: AVGの結果がNULLになりうるのにOption<f64>ではない
```

```rust
// OK: Option<T>を使う
struct ProductSummary {
    average_rating: Option<f64>,
}

// OK: COALESCEと"!"サフィックスでNOT NULLを保証
sqlx::query_as!(
    ProductSummary,
    r#"
    SELECT name,
           COALESCE(AVG(rating), 0)::FLOAT8  -- NULLなら0、FLOAT8にキャスト
           as "average_rating!"              -- "!"でNOT NULLを宣言
    FROM products GROUP BY name
    "#
)
```

| サフィックス | 意味 |
|-------------|------|
| `!` | NOT NULLを強制（Option\<T\>ではなくT） |
| `?` | NULLを許容（TではなくOption\<T\>） |

## まとめ

冒頭の`WHERE phone = NULL`は、`WHERE phone IS NULL`に書き換えて5分で解決した。3値論理を知っているかどうか——それだけの差だった。

NULLの問題はバグではなく、SQLの仕様だ。Rust/sqlxでは以下を守れば大半の問題は防げる。

1. NULLableカラムは`Option<T>`にマッピング
2. `= NULL`ではなく`IS NULL`を使う
3. `NOT IN`ではなく`NOT EXISTS`を使う
4. 空文字列とNULLを混在させない

迷ったら`Option<T>`を使う。後から`Option`を外すのは簡単だが、NULLが返ってきたときのパニックを本番で見るのは心臓に悪い。

そもそもNULLableカラムを減らす設計（NOT NULL制約のデフォルト化、別テーブルへの分離）も検討に値する。3値論理の詳細は『SQLアンチパターン』の「Fear of the Unknown」章を参照。

## 参考資料

- [SQL Antipatterns - Fear of the Unknown](https://pragprog.com/titles/bksqla/sql-antipatterns/)
- [PostgreSQL - Comparison Functions](https://www.postgresql.org/docs/current/functions-comparison.html)
- [sqlx - Compile-time checked queries](https://docs.rs/sqlx/latest/sqlx/macro.query.html)
