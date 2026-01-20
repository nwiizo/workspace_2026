# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Strategic Evolution Quest is a text-based strategic simulation RPG for learning Wardley Mapping, DDD, and Team Topologies. Players interact with scenarios through LLMs (ChatGPT, Claude, etc.) by loading scenario.md files.

Based on the book "Architecture Modernization" by Nick Tune & Jean-Georges Perrin.

## Structure

```
wardley-quest/
├── scenario.md              # Main game (2500+ lines)
├── README.md                # Project documentation
├── addon-*/                 # 10 addon quests
│   ├── README.md           # Addon overview
│   └── scenario.md         # Addon scenario
└── addon-observability/     # Stub (empty)
```

**Addons by phase:**
- Discovery: addon-discovery, addon-eventstorming
- Strategy: addon-portfolio
- Design: addon-domainboundary, addon-apidesign
- Implementation: addon-platform, addon-datamodeling
- Operations: addon-techdebt, addon-incident
- All phases: addon-change

## Main Scenario Features

The main scenario.md includes these game systems:

| System | Section | Description |
|--------|---------|-------------|
| 判定システム | 2.2 | 2D6 resolution with modifiers |
| モメンタムシステム | 2.3 | Success momentum resource (earn on success, spend for bonuses) |
| 気候イベント | 2.4 | D6 climate events each turn |
| 難易度スケーリング | 9.1.1 | Easy/Normal/Hard/Expert difficulty options |
| グッドエンディング | 8.4 | 5 narrative good endings |
| バッドエンディング | 8.3 | 6 narrative bad endings |

## Scenario File Structure

Each scenario.md follows a consistent structure:

1. **Title & Quote** - Thematic opening
2. **Core Concepts** - Framework/methodology with ASCII diagrams
3. **Game Mechanics** - Resolution, resources, modifiers
4. **NPCs** - Characters with relationship values and special abilities
5. **Practical Scenarios** - 0 (tutorial) + 1-3 (difficulty progression)
6. **Endings** - Bad (5+) and Good (3+) with narrative vignettes
7. **Cross-quest Connections** - Links to related addons

## Content Conventions

- **ASCII diagrams** using box-drawing characters (┌─┐│└─┘), 65-char width
- **Tables** for structured information (modifiers, events, NPCs)
- **Japanese language** throughout
- **NPCs** have: name, role, quotes, relationship value (-3 to +3), special abilities

## Scenario Design Standards

| Element | Standard |
|---------|----------|
| バッドエンディング | 5個 + 未確定セクション |
| グッドエンディング | 3個 |
| シナリオ | 0（学習モード）+ 1-3（難易度別） |

### NPC Roles (recommended patterns)

| Role | Perspective | Quests Provided |
|------|-------------|-----------------|
| 経営層/スポンサー | ビジネス価値・投資判断 | 戦略的意思決定 |
| アーキテクト/テックリード | 技術設計・実装 | 設計判断・実装パターン |
| ドメインエキスパート | 業務知識・ユーザー視点 | 要件理解・ユビキタス言語 |
| 現場エンジニア | 日常の課題・実務 | 実践的な問題解決 |
| ジュニア/新人 | 学習者の代理・素朴な疑問 | 基礎概念の確認 |

## When Adding/Editing Content

- Maintain the established section structure
- Use consistent ASCII diagram style (65-char width boxes)
- Each addon: 5+ bad endings + undetermined section + 3+ good endings
- NPCs: name, role, quotes, relationship value, special ability
- Link related concepts to other addons
- Update main README.md addon table when adding new addons

## No Build/Test Commands

This is a content-only project. No compilation, linting, or testing required.
