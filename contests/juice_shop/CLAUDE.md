# CLAUDE.md

OWASP Juice Shop CTF - **team2-takenoko**

## Overview

Playwright MCP を使った Juice Shop 自動攻略プロジェクト。

**進捗:** `result.md` | **目標:** 110問中75問解決済み (68%)

## Directory

```
difficulty-1/  (14/14 ✅)  difficulty-2/  (14/15)
difficulty-3/  (24/24 ✅)  difficulty-4/  (20/25)
difficulty-5-6/ (17/32)
```

## Rules

| Rule | Description |
|------|-------------|
| [challenge-docs](/.claude/rules/challenge-docs.md) | チャレンジ解決時のドキュメント作成 |
| [quick-reference](/.claude/rules/quick-reference.md) | 認証情報・ペイロード・エンドポイント |
| [attack-techniques](/.claude/rules/attack-techniques.md) | 学んだ攻撃手法 |
| [unsolved-challenges](/.claude/rules/unsolved-challenges.md) | 未解決チャレンジ一覧 |

## Skills

| Skill | Description |
|-------|-------------|
| [playwright-attack](/.claude/skills/playwright-attack/SKILL.md) | Playwright MCP 攻撃パターン |

## Quick Start

```bash
# Juice Shop 起動
docker run -p 3000:3000 bkimminich/juice-shop

# 管理者ログイン (SQLi)
email: ' OR 1=1--
password: a
```

## References

- https://help.owasp-juice.shop/appendix/solutions.html
- https://pwning.owasp-juice.shop/
- https://github.com/juice-shop/juice-shop
