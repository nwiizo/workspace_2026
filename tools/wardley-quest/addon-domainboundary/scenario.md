# Domain Boundary Quest - 境界を切る者、システムを制す

> *「正しい境界を見つけることは、正しいコードを書くことより難しい。しかし、その価値は100倍ある」*
> — Eric Evans, Domain-Driven Design より

## 1. 概要

**前提クエスト:** メインシナリオ Chapter 4「進化の方向」完了後

このクエストでは、ドメインの境界を「正しく切る」技術を習得します。
境界の引き方ひとつで、チームの自律性、コードの保守性、システムの進化可能性が決まります。

正しい境界がもたらすもの:
- チームの自律性
- 変更の局所化
- 認知負荷の軽減
- 独立したデプロイ

### コンパクトモード

| コマンド | 効果 |
|----------|------|
| 「コンパクトモード」 | 出力を1行要約+選択肢+状態のみに圧縮 |
| 「通常モード」 | 標準出力に戻す |
| 「状態確認」 | 現在のスコアのみ表示 |
| 「サマリー」 | これまでの決定を箇条書きで要約 |

---

## 2. 境界づけられたコンテキスト（Bounded Context）

### 2.1 なぜ境界が必要なのか

**境界なき世界の悲劇 — 「User」って何？**

各部門が同じ「User」を違う意味で使う:
- **認証**: email, password, sessions
- **課金**: payment methods, invoices
- **配送**: address, phone, name
- **サポート**: tickets, history, rating

全部門が同じテーブル（150カラム）を参照 → 変更のたびに全員が影響を受ける。これが「Big Ball of Mud」の始まり。

### 2.2 境界を切ると何が起こるか

各コンテキストが独自のモデルを持ち、「User」という曖昧な概念は存在しなくなる:

| コンテキスト | モデル名 | 主要属性 | 関心事 |
|-------------|---------|---------|-------|
| 認証 | Identity | userId, email, passwordHash, lastLogin | 誰であるか |
| 課金 | Customer | customerId, accountId, billingEmail, paymentMethods | 誰が払うか |
| 配送 | Recipient | recipientId, name, phone, addresses | どこに届けるか |
| サポート | Caller | callerId, displayName, tier, contactHistory | 誰をサポートするか |

コンテキスト間はID連携のみで接続する。

---

## 3. 境界を見つける技術

### 3.1 言語の違いに注目する

同じ言葉、違う意味 ＝ コンテキスト境界のサイン:

**「商品」** — カタログ: 説明・画像・カテゴリ / 在庫: SKU・数量 / 注文: 価格スナップショット / 配送: 重量・サイズ
**「注文」** — 販売: 顧客注文・売上 / 在庫: 引当・ピッキング / 配送: 出荷・追跡 / 経理: 売掛・請求
**「顧客」** — 営業: 見込み客・商談 / サポート: チケット / マーケ: セグメント / 経理: 請求先・与信

ヒント: 「〇〇から見た□□」という言い方が必要なら、そこに境界がある可能性が高い。

### 3.2 変更の軸に注目する

| 領域 | 変更頻度 | 例 |
|------|---------|-----|
| 高頻度 | 日次〜週次 | プロモーション、価格設定、割引ルール |
| 低頻度 | 四半期〜年次 | 商品マスタ基本属性、認証基盤 |

変更頻度が大きく異なる領域は別コンテキストにすべき。高頻度変更領域を低頻度領域から分離 → デプロイ独立性確保。

### 3.3 チーム構造に注目する（逆コンウェイ戦略）

- **コンウェイの法則**: システムの構造は組織の構造を反映する
- **逆コンウェイ戦略**: 望ましいシステム構造に合わせて組織を設計する
- **目安**: 1チーム（5-9人）= 1-3 Bounded Context
- **Anti-pattern**: 1つのBounded Contextを複数チームで担当 → 調整コスト爆発
- **Good pattern**: Stream-aligned team がコンテキストを所有

---

## 4. 実装パターン：境界をコードで表現する

### 4.1 モジュラーモノリス

```
project/
├── src/
│   ├── modules/
│   │   ├── ordering/           # 注文コンテキスト
│   │   │   ├── domain/
│   │   │   ├── application/
│   │   │   ├── infrastructure/
│   │   │   ├── api/
│   │   │   └── mod.rs          # 公開APIのみexport
│   │   ├── inventory/          # 在庫コンテキスト
│   │   └── catalog/            # カタログコンテキスト
│   ├── shared_kernel/          # 共有カーネル（最小限に）
│   └── main.rs
└── Cargo.toml
```

```rust
// ordering/mod.rs - 公開APIの制御
pub mod api;
pub use application::commands::{CreateOrderCommand, SubmitOrderCommand};
pub use application::queries::{GetOrderQuery, OrderDto};
pub use domain::events::{OrderCreated, OrderSubmitted};

// 内部実装は非公開
mod domain;
mod application;
mod infrastructure;
```

```rust
// 境界を超える連携: イベント駆動
#[derive(Clone, Serialize, Deserialize)]
pub struct OrderSubmitted {
    pub order_id: OrderId,
    pub customer_id: CustomerId,
    pub items: Vec<OrderItemSnapshot>,
    pub total: Money,
    pub submitted_at: DateTime<Utc>,
}

// inventory モジュールでの購読
pub async fn handle_order_submitted(event: OrderSubmitted) -> Result<(), Error> {
    for item in event.items {
        inventory_service
            .reserve_stock(item.product_id, item.quantity)
            .await?;
    }
    Ok(())
}
```

### 4.2 Anti-Corruption Layer（腐敗防止層）

ACLの役割: レガシーの複雑さを新システムに漏らさない（命名規則変換、データ構造変換、デフォルト値設定、フラグ解釈）

```rust
pub struct LegacyCustomerAdapter {
    legacy_client: LegacySystemClient,
}

impl CustomerRepository for LegacyCustomerAdapter {
    async fn find_by_id(&self, id: CustomerId) -> Result<Option<Customer>, Error> {
        let legacy = self.legacy_client
            .get_customer_master(&id.to_legacy_format())
            .await?;

        match legacy {
            Some(record) => {
                let customer = Customer {
                    id,
                    name: CustomerName::new(&record.CUST_NM.trim())?,
                    address: self.translate_address(&record)?,
                    status: self.interpret_status_flags(&record)?,
                    preferences: CustomerPreferences::default(),
                };
                Ok(Some(customer))
            }
            None => Ok(None),
        }
    }

    fn interpret_status_flags(&self, record: &LegacyRecord) -> Result<CustomerStatus, Error> {
        match (record.FLG_1.as_str(), record.FLG_2.as_str()) {
            ("1", "0") => Ok(CustomerStatus::Active),
            ("0", _) => Ok(CustomerStatus::Inactive),
            ("1", "1") => Ok(CustomerStatus::Suspended),
            _ => Err(Error::UnknownLegacyStatus),
        }
    }
}
```

### 4.3 Context Map パターン

| パターン | 説明 | 例 |
|---------|------|-----|
| Partnership | 両チームが協力して共通の目標に向かう | 注文チーム ⇔ 配送チーム（共同設計） |
| Customer-Supplier | 上流が下流のニーズに応える | 商品マスタ(U) → 注文サービス(D) |
| Conformist | 下流が上流のモデルにそのまま従う | 外部決済API(U) → 課金サービス(D) |
| Anti-Corruption Layer | 下流が変換層で自己のモデルを守る | レガシー(U) →|ACL|→ 新システム(D) |
| Shared Kernel | 複数コンテキストで共有する小さなモデル | Money, EntityId, Address（最小限に保つ） |

---

## 5. よくある失敗と対処法

### 5.1 技術レイヤーで分割（Anti-Pattern）

**悪い分割**: Frontend Service / API Gateway / Database Service → 1機能変更で3サービス全て変更が必要

**良い分割**: Order Service / Inventory Service / Catalog Service（各サービスがUI→API→DBを縦に持つ）→ 機能変更は1サービス内で完結

### 5.2 分散モノリス（Anti-Pattern）

サービスは分かれているが、同期呼び出しチェーン＋共有DBで実質モノリス。症状:
- 1サービス障害で全体停止
- デプロイは全サービス同時
- テストは結合テストのみ有効

### 5.3 境界を守る実装テクニック

```rust
// コンパイル時に境界違反を検出する
mod domain;          // private - 外部から直接アクセス不可
mod infrastructure;  // private
pub mod api;         // public - HTTP APIのみ公開

// ドメインモデルは直接公開しない
pub use api::dto::{OrderResponse, CreateOrderRequest};

// ❌ use ordering::domain::order::Order;  // Error: module `domain` is private
// ✅ use ordering::api::dto::OrderResponse;
```

```rust
// モジュール境界をテストで守る
#[cfg(test)]
mod architecture_tests {
    #[test]
    fn ordering_should_not_depend_on_inventory_internals() {
        let ordering_deps = get_module_dependencies("ordering");
        assert!(ordering_deps.contains("shared_kernel"));
        assert!(ordering_deps.contains("inventory::api"));
        assert!(!ordering_deps.contains("inventory::domain"));
        assert!(!ordering_deps.contains("inventory::infrastructure"));
    }
}
```

---

## 6. NPCガイド

> **Note**: 以下のNPCは代表的な役割パターンです。GMはシナリオのテーマや複雑さに応じて、NPCの追加・削除・変更を自由に行えます。

### 中村アーキテクト（境界の達人）

**台詞**: 「境界を引くのは簡単だ。正しい場所に引くのが難しい。技術で引くな、ビジネスで引け」「最初から完璧な境界なんてない。学びながら境界を調整していく。それがモジュラーモノリスの良いところだよ」

**クエスト**: "言葉の境界"（ユビキタス言語の違い発見） / "変更の軸"（変更頻度で境界発見） / "モノリス分割"（既存システムの境界発見）

### 松本ドメインエキスパート（業務のプロ）

**台詞**: 「"在庫"って言葉、倉庫チームと販売チームで意味が違うんだよね」「この帳票、3つの部署が微妙に違う言葉で同じこと言ってる」

**クエスト**: "言葉の発掘"（部署ごとの用語の違い発見） / "業務フロー分析"（どこで概念が変わるか）

### 藤井テックリード（実装のプロ）

**台詞**: 「境界は設計図じゃ守れない。コードで強制しないと」「分散システムの境界はAPIスキーマで守る。Contract Testは必須だよ」

**クエスト**: "コードで守る境界"（モジュール設計） / "契約による結合"（API契約テスト） / "ACLの構築"（レガシー連携）

### 斎藤エンジニアリングマネージャー（組織のプロ）

**台詞**: 「チーム構造とシステム構造は連動する。境界を変えるなら、チーム構造も変える覚悟がいる」

**クエスト**: "チームと境界"（Team Topologies適用） / "認知負荷"（チームが担当できる境界の見極め）

### 田中ジュニアエンジニア（学習者視点）

**台詞**: 「なんでこのモジュールは別のパッケージになってるんですか？」「"境界づけられたコンテキスト"って実際のコードではどこに現れるんですか？」

**クエスト**: "初めての境界"（なぜ分けるか理解） / "コードを読む"（既存境界の表現） / "変更の追跡"（PRレビューで境界意識）

---

## 7. 実践シナリオ

基本判定はメインscenario参照。以下はアドオン固有の修飾子のみ記載。

### シナリオ0: 学習モード — 境界設計の基礎を学ぶ

**背景**: あなたは新しいチームに配属されたエンジニア。中村アーキテクトから、ドメイン境界の基本を学ぶことになった。

**学習目標**:
- 境界づけられたコンテキスト（Bounded Context）の概念理解
- 言語の違いから境界を発見する方法
- モジュラーモノリスでの境界表現
- コンテキストマップの読み方・書き方

**レッスン構成（4ターン）**:
1. **なぜ境界が必要か** — God Object問題を体験、境界がないと何が起こるか
2. **言語で境界を見つける** — 同じ言葉・違う意味のパターン発見、ユビキタス言語のマッピング
3. **コードで境界を表現する** — pub/privateで境界を守る、依存関係の方向
4. **コンテキストマップを描く** — 関係性パターン（Partnership, Customer-Supplier等）、自分のシステムのマップ作成

※ 学習モードでは全判定に+2のボーナスがつきます

### シナリオ1: モノリスの分割 — 10年物のモノリスを分割せよ

**現状**: 50万行のモノリス / 「User」テーブルは200カラム / デプロイ4時間・月1回リリース / 全機能が密結合

**ビジネス要件**: 新機能を週次リリース / 特定機能のスケール / 新チームの独立開発

**選択肢**:
- **A) ビッグバン・リライト** — 全てを捨ててマイクロサービスで再構築
- **B) Strangler Fig パターン** — 新機能から徐々に分離、レガシーを絞め殺す
- **C) モジュラーモノリス化** — まず内部で境界を作り、必要に応じて分離

### シナリオ2: 境界の再設計 — 間違った境界を修正せよ

**2年前の設計**: 「商品サービス」「注文サービス」「ユーザーサービス」に分割

**現在の問題**: 新機能追加時に3サービス全変更必要 / 「商品」の定義がサービスごとに違う / チーム間調整会議が週3回 / デプロイは結局全サービス同時

**分析結果**: 本来の境界は「カタログ」「販売」「フルフィルメント」/ 現在の「商品」は3コンテキストに分散すべき

**選択肢**:
- **A) 一気に再構築** — 3ヶ月の開発凍結、全面再設計
- **B) 段階的移行** — 新しい境界に沿って少しずつ移動
- **C) 並行運用** — 新旧両方を動かしながら徐々に切り替え

### シナリオ3: 新規サービスの境界設計 — グリーンフィールドの境界設計

**新規プロジェクト**: BtoB SaaSプラットフォーム

**主要機能**: 顧客企業管理（テナント） / ユーザー認証・認可 / 請求・課金 / コア業務機能（テナントごとにカスタマイズ可能） / 分析・レポート

**チーム構成**: 3つのStream-alignedチーム（各5-7人） / 1つのPlatformチーム

**設計課題**:
- どのような境界で分割するか？
- 各チームにどのコンテキストを担当させるか？
- 共有カーネルには何を入れるか？
- コンテキスト間の連携パターンは？

あなたのContext Mapを描いてください。

---

## 8. バッドエンディング

### 8.1「分散モノリス」
**条件**: 技術レイヤーで境界を引き、サービス間の同期呼び出しチェーンを作ってしまった
**学び**: 境界はビジネスケイパビリティで引く / 同期連鎖は分散モノリスを生む / モノリスの問題を分散させると複雑さは指数的に増加

### 8.2「共有データベースの罠」
**条件**: サービス間でデータベースを共有し、スキーマ変更が全チームに影響する状態にした
**学び**: データの境界はサービスの境界と一致させる / 共有DBはデプロイ独立性を破壊する / 誰がどのカラムを使っているか不明になる

### 8.3「境界なきドメイン」
**条件**: 全社統一ドメインモデルを追求し、50属性のUserエンティティを作ってしまった
**学び**: 統一モデルは幻想、文脈によって意味は変わる / 全部門の要件を1モデルに詰めると複雑怪奇になる / 変更に全部門の承認が必要になる

### 8.4「ナノサービス地獄」
**条件**: 過剰に分割し、150のサービスを作ってしまった
**学び**: 分割には適切な粒度がある / サービス間通信レイテンシが支配的になる / 全体を理解できる人がいなくなる

### 8.5「コンウェイの呪い」
**条件**: アーキテクチャを設計したがチーム構造を変えなかった
**学び**: アーキテクチャとチーム構造は一緒に設計する / 1つのコンテキストを複数チームで触ると責任が分散する / オーナーシップの曖昧さは調整コストを爆発させる

### 8.6 未確定バッドエンディング

プレイヤーの選択次第で以下も到達可能:
- ACLなしのレガシー連携 → レガシーの複雑さが新システムを侵食
- 共有カーネルの肥大化 → 「共通」が巨大な依存関係に
- 境界を超えたトランザクション → 分散トランザクション地獄
- イベントスキーマの破壊的変更 → 非同期連携の崩壊
- ドメインエキスパートなき境界設計 → 技術者の思い込みによる誤った分割
- 境界を変えられない硬直化 → 学びを反映できないアーキテクチャ

---

## 9. グッドエンディング

### 9.1「自律するチーム」
**条件**: 正しい境界設計により各チームが独立してデプロイ可能になった
**学び**: 正しい境界はチームの自律性を最大化する / 認知負荷の低減が生産性向上に直結 / 新機能の90%が単一チームで完結する状態が理想

### 9.2「進化可能なアーキテクチャ」
**条件**: モジュラーモノリスから始め、必要に応じてコンテキストを分離できた
**学び**: 最初からマイクロサービスにしない勇気 / 明確な境界があれば必要時に分離できる / イベント駆動＋Contract Testで安全にリファクタリング

### 9.3「ドメインの可視化」
**条件**: 境界設計によりコンテキストごとのコスト・パフォーマンス・生産性が可視化された
**学び**: 技術とビジネスが同じ言葉で会話できる / 投資判断の根拠が明確になる / ボトルネックの特定と効果測定が容易になる

---

## 10. 関連クエストへの接続

| 関連クエスト | 接続ポイント |
|-------------|-------------|
| EventStorming Quest | 境界発見のファシリテーション |
| DDD Data Modeling | コンテキスト内のモデリング |
| API Design Quest | コンテキスト間の契約設計 |
| Platform Eng Quest | 境界を支えるプラットフォーム |
| Change Leadership Quest | 境界変更時の組織変革 |

---

## 付録

引継書作成は「引継書を作成して」とGMに依頼。メインscenario.md の付録D参照。

---

## 参考文献

- 「ドメイン駆動設計」Eric Evans 著
- 「チームトポロジー」Matthew Skelton, Manuel Pais 著
