# Ory Ketoで認可を実装する ― Zanzibarモデル入門

## 前回からの続き

前回の記事では、Ory Kratosを導入して認証システムを構築した。自前実装のRust Login ProviderをKratosに置き換え、パスワード認証、MFA、パスワードリセットが「設定ファイルを書くだけ」で動くようになった。Have I Been Pwnedによる漏洩パスワードチェックという、自分では思いつかなかった機能も手に入った。

> **前提知識**: この記事はOry Stackシリーズの続編です。Ory HydraによるOAuth2認可、Ory Kratosによる認証の基礎知識を前提に進めます。認証（Authentication）と認可（Authorization）の違いを理解している前提です。

でも、ログインしたユーザーが「何をできるか」は、まだ決まっていない。

```rust
// 前回までの実装にはこれがない
if user.can_edit(&document) {
    // 編集処理
}
```

認証（Authentication）と認可（Authorization）は別物だ。認証は「誰であるか」を確認する。認可は「何ができるか」を判断する。

Kratosは認証を担当する。では認可は？

自前で実装するか。データベースにロールテーブルを作り、ミドルウェアで権限チェックを入れる。前回のパターンだ。

でも、前回学んだことを思い出した。

「自前で作ることの非合理性」

認可システムも同じではないか。RBACを実装し、ABACに拡張し、リソース階層を考慮し、監査ログを取り……。

調べていく中で、Googleの「Zanzibar」という論文に出会った。

[https://research.google/pubs/pub48190/:embed:cite]

そして、その実装であるOry Ketoを見つけた。

## Ory Ketoとは

[https://github.com/ory/keto:embed:cite]

[https://www.ory.com/docs/keto:embed:cite]

Ory Ketoは「Zanzibarモデルを実装した認可サーバー」だ。

Zanzibarとは、Googleが社内で使っている認可システム。Drive、YouTube、Calendar、Cloud——数十億のオブジェクトと数百万の権限チェックを毎秒処理している。

その設計思想をオープンソースで実装したのがKetoだ。

```
┌─────────────────────────────────────────────────────────────┐
│                     Ory Stack                               │
├────────────────┬───────────────┬────────────────────────────┤
│   Ory Kratos   │  Ory Hydra    │        Ory Keto            │
│  (認証)        │ (OAuth2/OIDC) │       (認可)               │
├────────────────┼───────────────┼────────────────────────────┤
│ - ログイン     │ - トークン発行 │ - 権限チェック            │
│ - 登録         │ - クライアント │ - 関係性管理              │
│ - MFA          │   管理        │ - RBAC/ABAC/ReBAC         │
└────────────────┴───────────────┴────────────────────────────┘
```

Kratosで「誰か」を確認し、Ketoで「何ができるか」を判断する。これでOry Stackの認証・認可が揃う。

## Zanzibarモデルの基本概念

Zanzibarモデルには4つの基本概念がある。

### Namespace（名前空間）

オブジェクトの種類を定義する。

```
User        # ユーザー
Organization # 組織
Project     # プロジェクト
Document    # ドキュメント
```

RDBMSでいうテーブルに相当する。

### Object（オブジェクト）

Namespace内の具体的なインスタンス。

```
Organization:acme    # acmeという組織
Project:alpha        # alphaというプロジェクト
Document:doc1        # doc1というドキュメント
```

RDBMSでいうレコードに相当する。

### Relation（関係）

オブジェクトに対する関係性の種類。

```
owner   # 所有者
editor  # 編集者
viewer  # 閲覧者
member  # メンバー
admin   # 管理者
```

従来のRBACでいう「ロール」に近いが、決定的な違いがある。

RBACのロールは「ユーザーに付与される」。

```
# RBAC: ユーザー → ロール
alice: [admin, editor]
bob: [viewer]
```

Zanzibarのリレーションは「オブジェクトとユーザーの間に存在する」。

```
# Zanzibar: オブジェクト ← リレーション → ユーザー
Document:doc1#editor@alice
Document:doc2#viewer@alice
Project:alpha#admin@alice
```

この違いは大きい。RBACでは「aliceは管理者だから全てのドキュメントを編集できる」となりがちだ。Zanzibarでは「aliceはdoc1の編集者であり、doc2の閲覧者であり、alphaの管理者である」と、**オブジェクトごとに関係を定義できる**。粒度が細かい。

### Subject（主体）

関係の対象となる存在。単純なユーザーIDか、**Subject Set**（他のオブジェクトの関係）を指定できる。

```
alice                           # aliceというユーザー
Organization:acme#member        # acmeのメンバー全員
```

**Subject Set**が重要だ。これにより「acmeのメンバーは全員、このドキュメントを閲覧できる」といった表現が可能になる。

なぜこれが重要なのか。従来のアプローチと比較する。

```
# 従来のアプローチ：個別に権限を付与
Document:doc1#viewer@alice
Document:doc1#viewer@bob
Document:doc1#viewer@charlie
# → メンバーが増えるたびに追加が必要
# → メンバーが100人いれば100行必要
```

```
# Subject Set：グループ単位で権限を付与
Organization:acme#member@alice
Organization:acme#member@bob
Organization:acme#member@charlie
Document:doc1#viewer@Organization:acme#member
# → メンバーが増えても、Organization:acme#memberへの追加だけでOK
# → Document:doc1の権限設定は1行で済む
```

この設計により、**権限の管理がO(n×m)からO(n+m)に削減される**。nがユーザー数、mがリソース数だとすると、従来は最大n×m個の権限エントリが必要だった。Subject Setを使えば、n個のメンバーシップ + m個のリソース権限で済む。

Google Driveで「フォルダを共有」すると中のファイル全てにアクセスできるのは、この仕組みのおかげだ。

## Relation Tuple（関係タプル）

これら4つを組み合わせた「誰が何に対してどんな関係を持つか」を**Relation Tuple**と呼ぶ。

```
namespace:object#relation@subject
```

具体例。

```
Organization:acme#admin@alice
# aliceはacmeの管理者

Organization:acme#member@bob
# bobはacmeのメンバー

Project:alpha#viewer@Organization:acme#member
# acmeのメンバー全員がalphaを閲覧可能
```

最後の例がSubject Setの威力だ。組織にメンバーを追加すれば、自動的にプロジェクトの閲覧権限も付与される。

## Docker Composeで動かす

理論は十分。実際に動かしてみよう。

```yaml
services:
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: secret
      POSTGRES_DB: keto
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres -d keto"]
      interval: 5s
      timeout: 5s
      retries: 5
    networks:
      - keto

  keto-migrate:
    image: oryd/keto:v0.12.0
    environment:
      DSN: postgres://postgres:secret@postgres:5432/keto?sslmode=disable
    command: migrate up --yes --config /etc/config/keto/keto.yml
    volumes:
      - ./keto:/etc/config/keto:ro
    depends_on:
      postgres:
        condition: service_healthy
    networks:
      - keto

  keto:
    image: oryd/keto:v0.12.0
    environment:
      DSN: postgres://postgres:secret@postgres:5432/keto?sslmode=disable
      LOG_LEVEL: debug
    command: serve --config /etc/config/keto/keto.yml
    volumes:
      - ./keto:/etc/config/keto:ro
    ports:
      - "4466:4466"  # Read API
      - "4467:4467"  # Write API
      - "4468:4468"  # Metrics
    depends_on:
      keto-migrate:
        condition: service_completed_successfully
    healthcheck:
      test: ["CMD", "wget", "-q", "--spider", "http://localhost:4466/health/ready"]
      interval: 10s
      timeout: 5s
      retries: 5
    networks:
      - keto

volumes:
  postgres_data:

networks:
  keto:
```

Ketoは2つのAPIを公開する。

- **Read API（4466）**: 権限チェック、関係の一覧取得
- **Write API（4467）**: 関係の作成・削除

本番環境ではWrite APIを内部ネットワークに限定し、Read APIのみを公開するのが一般的だ。

## Keto設定ファイル

```yaml
version: v0.12.0

log:
  level: debug
  format: text

serve:
  read:
    host: 0.0.0.0
    port: 4466
  write:
    host: 0.0.0.0
    port: 4467
  metrics:
    host: 0.0.0.0
    port: 4468

namespaces:
  - id: 0
    name: User
  - id: 1
    name: Organization
  - id: 2
    name: Project
  - id: 3
    name: Document
```

`namespaces`でオブジェクトの種類を定義する。IDは内部で使用され、名前は人間が読むためのもの。

## 環境の起動と動作確認

```sh
docker compose up -d
```

ヘルスチェック。

```sh
curl http://localhost:4466/health/ready
# {"status":"ok"}
```

## E2Eテスト：権限モデルの構築

実際に権限モデルを構築して、チェックが正しく動作することを確認した。

### 権限構造

```
Organization:acme
├── admin: [alice]
└── member: [alice, bob, charlie]

Project:alpha
├── owner: [alice]
├── editor: [bob]
└── viewer: [Organization:acme#member]  ← 全メンバーが閲覧可能

Document:doc1
├── editor: [alice]
└── viewer: [Project:alpha#editor]  ← プロジェクト編集者が閲覧可能

Document:secret
└── viewer: [alice]  ← aliceのみ閲覧可能
```

### Relation Tupleの作成

Write API（4467）を使って関係を作成する。

```sh
# aliceをacmeの管理者に
curl -X PUT "http://localhost:4467/admin/relation-tuples" \
  -H "Content-Type: application/json" \
  -d '{
    "namespace": "Organization",
    "object": "acme",
    "relation": "admin",
    "subject_id": "alice"
  }'

# bobをacmeのメンバーに
curl -X PUT "http://localhost:4467/admin/relation-tuples" \
  -H "Content-Type: application/json" \
  -d '{
    "namespace": "Organization",
    "object": "acme",
    "relation": "member",
    "subject_id": "bob"
  }'

# acmeのメンバー全員にProject:alphaの閲覧権限を付与
curl -X PUT "http://localhost:4467/admin/relation-tuples" \
  -H "Content-Type: application/json" \
  -d '{
    "namespace": "Project",
    "object": "alpha",
    "relation": "viewer",
    "subject_set": {
      "namespace": "Organization",
      "object": "acme",
      "relation": "member"
    }
  }'
```

最後の例で`subject_id`ではなく`subject_set`を使っている点に注目してほしい。これがZanzibarモデルの核心だ。

### 権限チェック

Read API（4466）を使って権限をチェックする。

```sh
# aliceはacmeの管理者か？
curl -X POST "http://localhost:4466/relation-tuples/check" \
  -H "Content-Type: application/json" \
  -d '{
    "namespace": "Organization",
    "object": "acme",
    "relation": "admin",
    "subject_id": "alice"
  }'
# {"allowed": true}

# bobはacmeの管理者か？
curl -X POST "http://localhost:4466/relation-tuples/check" \
  -H "Content-Type: application/json" \
  -d '{
    "namespace": "Organization",
    "object": "acme",
    "relation": "admin",
    "subject_id": "bob"
  }'
# {"allowed": false}
```

### テスト結果

14個のパーミッションチェックを実行し、全てパスした。

```
--- Organization Checks ---
PASS: alice admin Organization:acme = true
PASS: bob admin Organization:acme = false
PASS: alice member Organization:acme = true
PASS: bob member Organization:acme = true
PASS: charlie member Organization:acme = true
PASS: dave member Organization:acme = false

--- Project Checks ---
PASS: alice owner Project:alpha = true
PASS: bob owner Project:alpha = false
PASS: bob editor Project:alpha = true
PASS: charlie editor Project:alpha = false

--- Document Checks ---
PASS: alice editor Document:doc1 = true
PASS: bob editor Document:doc1 = false
PASS: alice viewer Document:secret = true
PASS: bob viewer Document:secret = false
```

### 関係の一覧取得

作成した関係を確認する。

```sh
curl "http://localhost:4466/relation-tuples?namespace=Organization"
```

```
Organization tuples:
"acme#admin@alice"
"acme#member@charlie"
"acme#member@bob"
"acme#member@alice"

Project tuples:
"alpha#owner@alice"
"alpha#editor@bob"
"alpha#viewer@Organization:acme#member"

Document tuples:
"doc1#viewer@Project:alpha#editor"
"doc1#editor@alice"
"secret#viewer@alice"
```

`alpha#viewer@Organization:acme#member`という表記に注目してほしい。これはSubject Setを使った関係で、「Organization:acmeのmember関係を持つ全員」がalphaのviewerであることを示している。

## Subject Setの威力

従来のRBACでは、プロジェクトにメンバーを追加するとき、各リソースに対して個別に権限を設定する必要があった。

```
# 従来のRBAC（イメージ）
Project:alpha#viewer@bob
Document:doc1#viewer@bob
Document:doc2#viewer@bob
...
```

Zanzibarモデルでは、Subject Setを使って関係を「継承」できる。

```
# Zanzibarモデル
Organization:acme#member@bob              # bobをacmeメンバーに
Project:alpha#viewer@Organization:acme#member  # acmeメンバー全員がviewer
```

bobをacmeのメンバーに追加するだけで、alphaへのアクセス権が自動的に付与される。メンバーを削除すれば、権限も自動的に剥奪される。

これが「数十億のオブジェクト」を管理できる理由だ。権限を1つ1つ設定するのではなく、関係性のグラフとして表現する。

## 従来の認可モデルとの比較

| 観点 | RBAC | ABAC | Zanzibar (Keto) |
|------|------|------|-----------------|
| 基本単位 | ロール | 属性 | 関係 |
| 粒度 | 粗い | 細かい | 細かい |
| 継承 | ロール階層 | なし | Subject Set |
| 動的判断 | 困難 | 可能 | 可能 |
| スケーラビリティ | 中 | 低 | 高 |
| 実装複雑度 | 低 | 高 | 中 |

この表だけでは分かりにくいので、具体的なシナリオで比較する。

### シナリオ：Google Driveのような共有機能

「ユーザーAが作成したドキュメントを、チームBのメンバー全員が閲覧でき、チームBのマネージャーは編集できる」という要件を考える。

**RBACの場合：**
```
ロール: document_123_viewer, document_123_editor
ユーザーとロールの紐付け:
  - チームBのメンバー全員 → document_123_viewer
  - チームBのマネージャー → document_123_editor
```
問題点：ドキュメントが1000個あれば、ロールも1000セット必要。チームにメンバーが追加されるたびに、全ドキュメントのロール割り当てを更新する必要がある。

**ABACの場合：**
```
ポリシー:
  IF user.team == "B" AND resource.type == "document" THEN allow("view")
  IF user.team == "B" AND user.role == "manager" AND resource.type == "document" THEN allow("edit")
```
問題点：ポリシーが複雑化しやすい。「チームBのうち、サブチームCのメンバーは除外」のような条件が増えると、ポリシーが爆発する。また、全ての判断時にポリシーを評価するため、パフォーマンスが低下しやすい。

**Zanzibar（Keto）の場合：**
```
Team:B#member@alice
Team:B#member@bob
Team:B#manager@charlie
Document:123#viewer@Team:B#member
Document:123#editor@Team:B#manager
```
チームにメンバーを追加すれば、自動的にドキュメントへのアクセス権が付与される。ドキュメントが増えても、パターンは同じ。関係のグラフとして表現するため、計算量が抑えられる。

Zanzibarモデルは、**RBACの簡潔さとABACの柔軟性を、関係のグラフという形で両立している**。

## 自前実装との比較

| 観点 | 自前RBAC | Keto |
|------|----------|------|
| 権限チェック | SQLクエリ | API呼び出し |
| 継承 | JOINの嵐 | Subject Set |
| パフォーマンス | DBに依存 | 最適化済み |
| 監査ログ | 自前実装 | 組み込み |
| 一貫性 | 自前保証 | Googleが検証済み |

この比較をもう少し具体的に掘り下げる。

### 「JOINの嵐」とは何か

自前でSubject Set相当の機能を実装すると、こうなる。

```sql
-- 「bobがProject:alphaを閲覧できるか」をSQLで判定
SELECT 1 FROM permissions p
WHERE p.resource = 'Project:alpha'
  AND p.action = 'view'
  AND (
    -- 直接の権限
    p.subject = 'bob'
    OR
    -- チームを経由した権限
    p.subject IN (
      SELECT CONCAT('Team:', t.id, '#member')
      FROM team_members tm
      JOIN teams t ON tm.team_id = t.id
      WHERE tm.user_id = 'bob'
    )
    OR
    -- 組織を経由した権限
    p.subject IN (
      SELECT CONCAT('Organization:', o.id, '#member')
      FROM organization_members om
      JOIN organizations o ON om.org_id = o.id
      WHERE om.user_id = 'bob'
    )
  )
LIMIT 1;
```

階層が深くなるほどJOINが増える。さらに「チームが組織に所属している」のような関係を追加すると、再帰的なクエリが必要になる。パフォーマンスは急激に悪化する。

Ketoは、この関係のグラフ探索を専用のデータ構造とアルゴリズムで最適化している。Googleの論文によれば、Zanzibarは毎秒1000万以上のチェックを処理できる。

### テストの責任範囲

前回の記事で58個のテストを書いた。認証システムの「できないこと」を確認するためだ。

認可システムでも同じアプローチが必要だ。でも、Ketoを使えば**テストの責任範囲が変わる**。

自前実装の場合、以下をすべてテストする必要がある。
- 権限チェックのロジック
- 継承の正しさ
- 同時更新時の一貫性
- パフォーマンス

Ketoを使う場合、以下だけをテストすればいい。
- 関係の作成が正しいか
- チェックAPIの呼び出し結果が期待通りか

内部のグラフ探索アルゴリズムはGoogleが検証済みだ。**テストの責任範囲が「実装の検証」から「設定の検証」に変わる**。同時更新時の一貫性もKetoが保証する。私たちは「データの投入とAPIの呼び出し」だけをテストすればいい。

## アプリケーションへの統合

実際のアプリケーションでは、ミドルウェアでKetoを呼び出す。

```rust
// Rustでの例（擬似コード）
async fn check_permission(
    keto_client: &KetoClient,
    user_id: &str,
    resource: &str,
    action: &str,
) -> Result<bool, Error> {
    let response = keto_client
        .check(CheckRequest {
            namespace: "Document".to_string(),
            object: resource.to_string(),
            relation: action.to_string(),
            subject_id: Some(user_id.to_string()),
            ..Default::default()
        })
        .await?;

    Ok(response.allowed)
}

// ハンドラーで使用
async fn edit_document(
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
    user: AuthenticatedUser,  // Kratosで認証済み
) -> Result<Response, AppError> {
    // Ketoで認可チェック
    if !check_permission(&state.keto, &user.id, &doc_id, "editor").await? {
        return Err(AppError::Forbidden);
    }

    // 編集処理
    // ...
}
```

認証（Kratos）と認可（Keto）が分離されているため、それぞれ独立してスケールできる。

## 次回予告

Ketoを導入したことで、以下が「APIを呼ぶだけ」で実現できるようになった。

- 権限チェック
- 関係の管理
- Subject Setによる継承
- 監査ログ

次回以降で深掘りしたい項目は以下だ。

- **Ory Oathkeeperとの統合** — API Gatewayレベルでの認可
- **Kratos + Ketoの連携** — 認証から認可へのシームレスな流れ
- **OPL（Ory Permission Language）** — より複雑な権限モデルの定義
- **本番環境での考慮事項** — キャッシュ、レイテンシ、可用性

## おわりに

正直に言うと、Zanzibarモデルを理解するのに時間がかかった。「Namespace、Object、Relation、Subject」——最初は抽象的すぎて、何をしているのかわからなかった。

でも、E2Eテストで14個のチェックが全てパスした時、腑に落ちた。

```
Organization:acme#member@bob
Project:alpha#viewer@Organization:acme#member
```

この2行で「bobはacmeのメンバーであり、acmeのメンバー全員がalphaを閲覧できる」という関係が表現されている。SQLで書くとJOINの嵐になる処理が、シンプルなタプルで表現できる。

なぜ腑に落ちたのか。これまで認可システムを「条件分岐」として考えていたからだ。

```
# 条件分岐的な発想（従来）
if user.role == "admin" or user.id in document.editors:
    allow()
```

この発想では、条件が増えるたびにif文が複雑化する。「チームメンバーなら」「プロジェクトオーナーなら」「組織の管理者なら」——条件が入れ子になり、バグが入り込む余地が増える。

Zanzibarは「グラフ探索」として認可を表現する。

```
# グラフ的な発想（Zanzibar）
bob → member → acme
acme#member → viewer → alpha
# bobからalphaへのパスが存在すれば、権限あり
```

条件分岐ではなく、ノード間のパスの有無で権限を判断する。条件を追加するのではなく、エッジを追加する。**論理演算ではなくグラフ探索**。この発想の転換が、複雑な権限モデルをシンプルに保つ鍵だった。

「関係性のグラフ」という発想。これがZanzibarの核心だった。

前回まででKratos（認証）を導入した。今回Keto（認可）を導入した。これでOry Stackの認証・認可基盤が揃った。

「自前で作ることの非合理性」——シリーズを通じて何度も思い出す言葉だ。認可システムも同じだった。仕様は理解できる。実装もできる。でも、グラフ探索のアルゴリズムをプロダクション品質で検証し続けることは、私たちの仕事ではない。

「このユーザーにこのドキュメントの編集権限を付与して」——Ketoで対応します。

この記事が参考になれば、**読者になったり**、**nwiizo**の**X**や**Github**をフォローしてくれると嬉しいです。

## 参考資料

### Ory Keto

- [Ory Keto GitHub](https://github.com/ory/keto)
- [Ory Keto Documentation](https://www.ory.com/docs/keto)
- [Keto Configuration Reference](https://www.ory.com/docs/keto/reference/configuration)

### Zanzibar

- [Zanzibar: Google's Consistent, Global Authorization System](https://research.google/pubs/pub48190/)
- [Zanzibar Academy](https://zanzibar.academy/)

### 関連プロジェクト

- [ory-keto-verification（GitHub）](https://github.com/nwiizo/workspace_2026/tree/main/samples/ory-keto-verification)
- [ory-kratos-verification（GitHub）](https://github.com/nwiizo/workspace_2026/tree/main/samples/ory-kratos-verification)
