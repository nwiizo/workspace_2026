# EventStorming Quest

[Strategic Evolution Quest](../) のアドオンシナリオ

ドメイン発見のためのワークショップ手法を学ぶRPG。Big Picture EventStormingからDesign Levelまで、段階的にドメインを解き明かします。

## 前提

**単体でもプレイ可能です。** [Strategic Evolution Quest](../) をプレイ済みだとより深く理解できます。

## 学べる概念

### EventStormingの3つのレベル

Alberto Brandoliniが考案した、ドメイン発見のためのワークショップ手法：

| レベル | 目的 | 参加者 | 所要時間 |
|---|---|---|---|
| **Big Picture** | ビジネス全体の流れを把握 | 全ステークホルダー | 半日〜1日 |
| **Process Level** | 特定プロセスの詳細化 | ドメイン関係者 | 2-4時間 |
| **Design Level** | 実装詳細の設計 | 開発チーム | 1-2時間 |

### EventStormingの要素

色付き付箋を使った視覚的なドメインモデリング：

- **ドメインイベント（オレンジ）**: ビジネス上重要な出来事（過去形）
- **コマンド（青）**: イベントを引き起こすアクション
- **アクター（黄小）**: コマンドを実行する人
- **集約（黄大）**: コマンドを受け取りイベントを発行
- **ポリシー（紫）**: イベントに反応するビジネスルール
- **外部システム（ピンク）**: 連携する外部サービス
- **ホットスポット（赤）**: 疑問点・課題・矛盾

### 境界づけられたコンテキストの発見

EventStormingの重要な成果物：

- **言語の境界**: 用語の意味が変わる場所
- **責任の境界**: 担当者・チームが変わる場所
- **ピボタルイベント**: 重要な転換点となるイベント
- **時間的ギャップ**: プロセス間の自然な区切り

### ファシリテーション技術

ワークショップを成功に導く進行術：

- **サイレントブレインストーミング**: 議論前に各自で付箋を書く
- **タイムライン構築**: イベントを時系列に並べる
- **ウォークスルー**: ストーリーとして読み上げる
- **ホットスポットの活用**: 矛盾を「発見」として扱う

## 参考書籍

| 書籍 | 著者 | 関連章・内容 |
|---|---|---|
| **Architecture Modernization** | Nick Tune, Jean-Georges Perrin | Ch.7: Big Picture EventStorming、モダナイゼーションでの活用 |
| **Introducing EventStorming** | Alberto Brandolini | EventStormingの原典、全レベルの詳細 |
| **Domain-Driven Design Distilled** | Vaughn Vernon | DDDの基礎、EventStormingとの統合 |
| **Domain-Driven Design** | Eric Evans | 戦略的設計、ユビキタス言語 |

## 他のアドオンとの関係

| アドオン | 関係性 |
|---|---|
| [Discovery Quest](../addon-discovery/) | EventStorming前のステークホルダー発見 |
| [Data Modeling Quest](../addon-datamodeling/) | 発見したドメインのデータ設計への適用 |
| [Portfolio Quest](../addon-portfolio/) | 発見したコンテキストの投資優先順位付け |
| [Change Quest](../addon-change/) | ワークショップを通じた組織変革 |

## 想定される学習成果

- ドメインイベントを中心としたビジネス理解手法の習得
- 境界づけられたコンテキストを発見する能力
- ワークショップファシリテーション技術
- 技術者とビジネス関係者の共通理解の構築方法

## 遊び方

生成AIに [scenario.md](./scenario.md) を読み込ませて、以下を伝える：

```
EventStorming Questをプレイしたいです。
シナリオ: 0（学習モード）
```

## シナリオ

| # | 名前 | 難易度 | 学習焦点 |
|---|---|---|---|
| 0 | 学習モード | チュートリアル | EventStormingの基本 |
| 1 | 新規事業ドメイン発見 | ★★☆ | 白紙からのドメイン発見 |
| 2 | レガシー業務フロー解明 | ★★★ | 暗黙知の可視化、例外フロー |
| 3 | 大規模組織横断ワークショップ | ★★★★ | 複数ステークホルダーの調整 |
| 4 | カスタム | 可変 | あなたの組織の状況 |

---

良ければStarをお願いします。

## ライセンス

Friend License (MIT-equivalent)
