# WITH RECURSIVE解剖：再帰CTEはどう動くのか

## はじめに

コメントの返信を全部取得したい。親コメントから始めて、その返信、さらにその返信……と辿っていく。単純なSELECTでは1階層しか取れない。

```sql
SELECT * FROM comments WHERE parent_id = 1;  -- 直接の返信だけ
```

この「ツリー全体を取得したい」という要求に対して、PostgreSQLは`WITH RECURSIVE`という構文を用意している。一見すると魔法のように動くが、内部では何が起きているのか。本記事ではWITH RECURSIVEの動作原理を解剖し、なぜこの形式で書く必要があるのかを理解する。

理解すれば、無限ループを避ける方法も、パフォーマンスを改善する方法も見えてくる。

## 隣接リストの限界

階層構造を表現する最もシンプルな方法が「隣接リスト」だ。

```sql
CREATE TABLE comments (
    id SERIAL PRIMARY KEY,
    parent_id INTEGER REFERENCES comments(id),
    content TEXT NOT NULL
);
```

親への参照だけを持つ。直感的でわかりやすい。

```
コメント1 (親なし)
├── コメント2 (parent_id = 1)
│   └── コメント4 (parent_id = 2)
└── コメント3 (parent_id = 1)
```

問題は「ある親の全子孫を取得する」クエリだ。

```rust
// 直接の子だけなら簡単
let children: Vec<Comment> = sqlx::query_as(
    "SELECT id, content FROM comments WHERE parent_id = $1"
)
.bind(parent_id)
.fetch_all(&pool).await?;
```

孫を取るには、子のIDを集めてもう一度クエリを投げる必要がある。曾孫も欲しければ3回。深さが固定でないなら、何回クエリを投げればいいかわからない。

これがN+1問題の変形だ。階層が深くなるほどクエリ数が増える。

## WITH RECURSIVEの基本構造

PostgreSQLのWITH RECURSIVEは、この問題を1つのクエリで解決する。

```sql
WITH RECURSIVE descendants AS (
    -- 1. Base case: 起点となる行
    SELECT id, content, 0 as depth
    FROM comments
    WHERE id = 1

    UNION ALL

    -- 2. Recursive case: 前回の結果を使って次の行を取得
    SELECT c.id, c.content, d.depth + 1
    FROM comments c
    JOIN descendants d ON c.parent_id = d.id
)
SELECT id, content, depth FROM descendants;
```

一見するとSQLとは思えない構文だ。自分自身を参照している。これがどう動くのか。

## 内部動作：ワーキングテーブルとは

PostgreSQLの再帰CTEは、内部的に2つのテーブルを使って動作する。

1. **結果テーブル**: 最終的に返す行を蓄積
2. **ワーキングテーブル**: 次のイテレーションで処理する行

動作の流れを追ってみる。

### イテレーション0（初期化）

```sql
SELECT id, content, 0 as depth FROM comments WHERE id = 1
```

Base caseを実行。結果は`{(1, 'ルートコメント', 0)}`。

- 結果テーブル: `{(1, 'ルートコメント', 0)}`
- ワーキングテーブル: `{(1, 'ルートコメント', 0)}`

### イテレーション1

ワーキングテーブルの行を`descendants`として使い、Recursive caseを実行。

```sql
SELECT c.id, c.content, d.depth + 1
FROM comments c
JOIN descendants d ON c.parent_id = d.id
-- descendants = {(1, 'ルートコメント', 0)}
```

`parent_id = 1`のコメントを取得。結果は`{(2, '返信1', 1), (3, '返信2', 1)}`。

- 結果テーブル: `{(1, ..., 0), (2, ..., 1), (3, ..., 1)}`
- ワーキングテーブル: `{(2, ..., 1), (3, ..., 1)}`（新しく取得した行のみ）

### イテレーション2

```sql
-- descendants = {(2, '返信1', 1), (3, '返信2', 1)}
```

`parent_id = 2`または`parent_id = 3`のコメントを取得。結果は`{(4, '返信1への返信', 2)}`。

- 結果テーブル: `{(1, ..., 0), (2, ..., 1), (3, ..., 1), (4, ..., 2)}`
- ワーキングテーブル: `{(4, ..., 2)}`

### イテレーション3

`parent_id = 4`のコメントを取得。該当なし。ワーキングテーブルが空になったので終了。

この「ワーキングテーブルが空になるまで繰り返す」という動作が、再帰の終了条件だ。

## UNIONとUNION ALLの違い

再帰CTEでは`UNION ALL`を使うのが一般的だが、`UNION`も使える。

```sql
WITH RECURSIVE tree AS (
    SELECT ...
    UNION       -- 重複を除去
    SELECT ...
)
```

`UNION`を使うと、既に結果テーブルにある行は追加されない。これは循環参照がある場合に無限ループを防ぐ効果がある。

ただしパフォーマンスは`UNION ALL`の方が良い。重複チェックのコストがないからだ。ツリー構造で循環がないことが保証されているなら、`UNION ALL`を使う。

## Rustでの実装

sqlxでWITH RECURSIVEを使う例を示す。

```rust
#[derive(Debug, sqlx::FromRow)]
struct CommentWithDepth {
    id: i32,
    content: String,
    depth: i32,
}

async fn get_descendants(pool: &PgPool, root_id: i32) -> Result<Vec<CommentWithDepth>> {
    let descendants: Vec<CommentWithDepth> = sqlx::query_as(
        r#"
        WITH RECURSIVE descendants AS (
            SELECT id, content, 0 as depth
            FROM comments
            WHERE id = $1

            UNION ALL

            SELECT c.id, c.content, d.depth + 1
            FROM comments c
            JOIN descendants d ON c.parent_id = d.id
        )
        SELECT id, content, depth FROM descendants
        "#,
    )
    .bind(root_id)
    .fetch_all(pool)
    .await?;

    Ok(descendants)
}
```

結果はフラットなVecで返る。ツリー構造に変換するにはアプリケーション側で処理が必要だ。

```rust
use std::collections::HashMap;

fn build_tree(comments: Vec<CommentWithDepth>) -> TreeNode {
    // depth順にソート済みと仮定
    // 実装は省略
}
```

## 深さ制限：無限ループを防ぐ

循環参照がある場合、再帰CTEは無限ループに陥る可能性がある。

```sql
-- 意図しない循環: 1 -> 2 -> 3 -> 1
UPDATE comments SET parent_id = 3 WHERE id = 1;  -- 危険！
```

対策として深さ制限を入れる。

```sql
WITH RECURSIVE descendants AS (
    SELECT id, content, 0 as depth
    FROM comments
    WHERE id = $1

    UNION ALL

    SELECT c.id, c.content, d.depth + 1
    FROM comments c
    JOIN descendants d ON c.parent_id = d.id
    WHERE d.depth < 100  -- 深さ制限
)
SELECT * FROM descendants;
```

PostgreSQL 14以降では`CYCLE`句も使える。

```sql
WITH RECURSIVE descendants AS (
    SELECT id, content, ARRAY[id] as path
    FROM comments
    WHERE id = 1

    UNION ALL

    SELECT c.id, c.content, d.path || c.id
    FROM comments c
    JOIN descendants d ON c.parent_id = d.id
    WHERE NOT c.id = ANY(d.path)  -- 既に訪問したノードはスキップ
)
CYCLE id SET is_cycle USING path
SELECT * FROM descendants WHERE NOT is_cycle;
```

## パフォーマンス特性

WITH RECURSIVEのパフォーマンスは、階層の深さとノード数に依存する。

```
イテレーション数 = 最大深さ
各イテレーションのコスト ≈ O(前回の結果行数 × インデックス検索)
```

`parent_id`にインデックスがないと、各イテレーションでSeq Scanが発生する。

```sql
CREATE INDEX idx_comments_parent ON comments(parent_id);
```

これで各イテレーションがIndex Scanになり、大幅に高速化する。

### 実測値の例

10ノード、深さ3の小さなツリーでの比較（参考値）。

| アプローチ | 実行時間 |
|-----------|---------|
| WITH RECURSIVE | ~150μs |
| 閉包テーブル | ~100μs |
| 経路列挙 (LIKE) | ~120μs |

小規模では差が出にくい。1000ノード以上になると閉包テーブルが有利になる場面がある。

## 代替アプローチとの比較

WITH RECURSIVEは万能ではない。用途に応じて使い分ける。

### 隣接リスト + WITH RECURSIVE

```
読み取り: ○（1クエリで全子孫）
書き込み: ◎（1行の更新のみ）
ストレージ: ◎（最小）
参照整合性: ◎（FK制約可能）
```

ツリー構造が頻繁に変更される場合に適している。

### 閉包テーブル

```sql
CREATE TABLE comment_paths (
    ancestor_id INTEGER REFERENCES comments(id),
    descendant_id INTEGER REFERENCES comments(id),
    depth INTEGER NOT NULL,
    PRIMARY KEY (ancestor_id, descendant_id)
);
```

全ての祖先-子孫関係を事前に格納する。

```
読み取り: ◎（JOINのみ、再帰不要）
書き込み: △（ノード追加時にO(深さ)行の挿入）
ストレージ: ×（O(ノード数²)）
参照整合性: ◎（FK制約可能）
```

```rust
// 閉包テーブルでの子孫取得
let descendants: Vec<Comment> = sqlx::query_as(
    r#"
    SELECT c.id, c.content, p.depth
    FROM comments c
    JOIN comment_paths p ON c.id = p.descendant_id
    WHERE p.ancestor_id = $1
    ORDER BY p.depth
    "#,
)
.bind(root_id)
.fetch_all(&pool).await?;
```

読み取りが圧倒的に多い場合に適している。

### 経路列挙

```sql
ALTER TABLE comments ADD COLUMN path TEXT;  -- '1.2.4' のような形式
```

```
読み取り: ○（LIKE検索）
書き込み: ×（サブツリー移動で全パス更新）
ストレージ: ○（中程度）
参照整合性: ×（パス文字列の整合性を保証できない）
```

PostgreSQLなら`ltree`拡張を使う方が良い。

```sql
CREATE EXTENSION ltree;

CREATE TABLE comments (
    id SERIAL PRIMARY KEY,
    path ltree NOT NULL,
    content TEXT NOT NULL
);

CREATE INDEX idx_comments_path ON comments USING GIST(path);

-- 子孫検索
SELECT * FROM comments WHERE path <@ '1.2';
```

GiSTインデックスで高速に検索できる。

## 判断フローチャート

```
ツリー構造が必要？
├─ NO → 通常のテーブル設計
└─ YES → 階層は頻繁に変更される？
          ├─ YES → 隣接リスト + WITH RECURSIVE
          └─ NO → 読み取りが圧倒的に多い？
                   ├─ YES → 閉包テーブル
                   └─ NO → パス検索が主な用途？
                            ├─ YES → ltree拡張
                            └─ NO → 隣接リスト + WITH RECURSIVE
```

迷ったら隣接リスト + WITH RECURSIVEから始める。シンプルで、後から他のアプローチに移行しやすい。

## 落とし穴

WITH RECURSIVEを使う際の注意点をまとめる。

### 1. Base caseが空だと何も返らない

```sql
WITH RECURSIVE descendants AS (
    SELECT * FROM comments WHERE id = 9999  -- 存在しないID
    UNION ALL
    ...
)
SELECT * FROM descendants;  -- 空の結果
```

起点が存在するか事前に確認するか、アプリケーション側でハンドリングする。

### 2. ORDER BYは最後に

```sql
WITH RECURSIVE descendants AS (
    SELECT id, content, 0 as depth ...
    ORDER BY id  -- ここでは効かない
    UNION ALL
    ...
)
SELECT * FROM descendants ORDER BY depth, id;  -- 最後に指定
```

CTEの内部でのORDER BYは無視されることがある。最終SELECTで指定する。

### 3. LIMIT/OFFSETとの組み合わせ

```sql
WITH RECURSIVE descendants AS (...)
SELECT * FROM descendants
ORDER BY depth
LIMIT 10;  -- 上位10件のみ
```

これは動作するが、再帰自体は全ノードを処理する。大規模なツリーでは深さ制限を先に入れた方が効率的。

### 4. 集約関数との組み合わせ

```sql
WITH RECURSIVE descendants AS (...)
SELECT COUNT(*) FROM descendants;  -- 全子孫の数
```

これは問題なく動作する。ただしGROUP BYを再帰CTE内部で使うことはできない。

## まとめ

WITH RECURSIVEは、ワーキングテーブルを使ったイテレーション処理だ。魔法ではなく、「前回の結果を使って次の行を取得し、結果が空になるまで繰り返す」という明確なアルゴリズムで動いている。

冒頭の「全子孫を取得したい」という要求は、隣接リスト + WITH RECURSIVEで1クエリで解決できる。ただし、用途によっては閉包テーブルやltree拡張の方が適している場合もある。

判断基準をまとめる。

- **頻繁に変更される**: 隣接リスト + WITH RECURSIVE
- **読み取り重視**: 閉包テーブル
- **パス検索が多い**: ltree拡張

最初は隣接リストで始め、パフォーマンス要件が明確になってから最適化するのが現実的だ。WITH RECURSIVEの動作原理を理解していれば、どの選択肢が自分のユースケースに合うか判断できる。

## 実行可能なデモコード

本記事のコードは以下で実行できる。

```sh
cd blog_03_complex_data_structures
cargo run
```

## 参考資料

- [PostgreSQL - WITH Queries](https://www.postgresql.org/docs/current/queries-with.html)
- [PostgreSQL - ltree](https://www.postgresql.org/docs/current/ltree.html)
- [sqlx - GitHub](https://github.com/launchbadge/sqlx)
