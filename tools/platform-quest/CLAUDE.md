# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Platform Quest is a text-based strategic simulation RPG for learning how to build, scale, and evolve an **internal platform** that developers love. Players interact with scenarios through LLMs (Claude, ChatGPT, etc.) by loading `scenario.md` files.

The game focuses on platform engineering as a **product discipline**: cognitive load, self-service, golden paths, control planes, observability, governance, multi-tenancy, federation, FinOps, incident response, deprecation, and AI-augmented platforms.

## Structure

```
platform-quest/
├── scenario.md                    # Main game (4 scenarios + core mechanics)
├── README.md                      # Project documentation
├── CLAUDE.md                      # This file
└── addon-*/                       # 13 addon quests
    ├── README.md                  # Addon overview
    └── scenario.md                # Addon scenario
```

**Addons by phase:**

- Foundation: `addon-product-thinking`
- Building: `addon-golden-path`, `addon-control-plane`, `addon-observability`, `addon-governance`, `addon-developer-portal`
- Operations: `addon-incident`, `addon-onboarding`, `addon-deprecation`
- Scaling: `addon-multitenancy`, `addon-federation`, `addon-cost`
- Future: `addon-ai-augmented`

## Main Scenario Game Systems

| System | Section | Description |
|--------|---------|-------------|
| リソース | 2.1 | 予算 / 開発者信頼度 / 認知負荷 / 経営層支持 / 成熟度 / 技術的負債 |
| 判定 | 2.2 | 2D6 + 修正値、6=失敗 / 7-9=部分成功 / 10+=成功 |
| モメンタム | 2.3 | 成功時に蓄積、消費で判定+2 or 気候イベント1回スキップ |
| 気候イベント | 2.4 | D6で毎ターン外圧（障害、コスト急騰、採用急増、ROI質問、追い風、監査） |
| 難易度スケーリング | 9.1 | Easy/Normal/Hard/Expert |
| グッドエンディング | 8.4 | 7種類のナラティブ（4軸: 技術/組織/経営財務/文化） |
| バッドエンディング | 8.3 | 10種類のナラティブ（4軸: 技術/組織/経営財務/文化） |

## Scenario File Structure

各 `scenario.md` は以下の構造を持つ:

1. **タイトル + 引用** - テーマを示す開幕
2. **中核概念** - フレームワーク・メソドロジー + ASCII図
3. **ゲーム機構** - 判定、リソース、修正値
4. **NPC** - 関係値（-3 to +3）と特殊能力を持つキャラクター
5. **実践シナリオ** - 0（学習モード）+ 1〜3（難易度別）
6. **エンディング** - バッド5+ / グッド3+ ナラティブ
7. **クロスクエスト接続** - 関連アドオンへのリンク

## Content Conventions

- **ASCII図** — 罫線文字（┌─┐│└─┘）で 65 char 幅
- **テーブル** — 構造情報（修正値、イベント、NPC）に使用
- **日本語** — 全文日本語。技術用語はカタカナ + 必要に応じて英語併記
- **NPC** — 名前 / 役割 / セリフ / 関係値 / 特殊能力 を持つ

## Scenario Design Standards

| 要素 | メイン | アドオン |
|---|---|---|
| バッドエンディング | 10個 + 未確定セクション | 7個 + 未確定セクション |
| グッドエンディング | 7個 | 4個 |
| シナリオ | 0（学習）+ 1〜3（難易度別） | 0（学習）+ 1〜3（難易度別） |

### エンディングの4軸

エンディングは以下の4軸で分布させる:

| 軸 | バッドの例 | グッドの例 |
|---|---|---|
| 技術 | ゴールデンケージ、シャドーIT分裂、コントロールプレーン融解 | ゴールデンパス確立、自己進化プラットフォーム |
| 組織 | 燃え尽き分解、プラットフォーム警察化、サイロ強化 | 倍力装置、組織の中の中央銀行 |
| 経営・財務 | 黒字シャットダウン、クラウド破産 | 投資ROI証明、収益化プラットフォーム |
| 文化・人間 | 信仰の崩壊、ナレッジディアスポラ | 「私たちのプラットフォーム」、伝説のチーム |

### NPC ロール（推奨パターン）

| ロール | 視点 | 提供クエスト |
|---|---|---|
| 経営層 / CTO | ビジネス価値・投資判断 | 戦略的意思決定 |
| プラットフォーム PdM | プロダクト思考・ユーザー視点 | 機能優先度・ロードマップ |
| 主要利用チームのテックリード | 利用者代表 | 摩擦の声・要望 |
| SRE リード | 信頼性・運用 | 信頼性のトレードオフ |
| セキュリティ / コンプライアンス | ガバナンス | 統制とスピードの両立 |
| ジュニア開発者 | 学習者の代理 | 認知負荷の体現者 |

## When Adding/Editing Content

- 既存セクション構造を維持する
- ASCII図は 65 char 幅で統一
- 各アドオン: 5+ バッドエンディング + 未確定セクション + 3+ グッドエンディング
- NPC: 名前 / 役割 / セリフ / 関係値 / 特殊能力 を含める
- 関連概念は他アドオンへリンク
- 新アドオン追加時は README.md のアドオン表を更新

## No Build/Test Commands

コンテンツのみのプロジェクト。コンパイル、リント、テストは不要。
