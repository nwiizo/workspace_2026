# Extra Language ✅

**難易度:** ⭐⭐⭐⭐⭐
**カテゴリ:** Broken Anti Automation
**目標:** 隠されたクリンゴン語（Klingon）翻訳ファイルにアクセスして言語を適用する

---

## 背景知識

### i18n（国際化）とは

i18n (internationalization の略: i + 18文字 + n) は、アプリケーションを複数の言語に対応させる仕組み。

```
┌─────────────────────────────────────────────────────────────────┐
│                    i18n の基本構造                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  /assets/i18n/                                                  │
│  ├── en.json      → 英語                                       │
│  ├── de_DE.json   → ドイツ語                                   │
│  ├── ja_JP.json   → 日本語                                     │
│  ├── zh_CN.json   → 中国語（簡体字）                           │
│  └── tlh_AA.json  → クリンゴン語 🖖 (隠し)                     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### クリンゴン語とは

- **起源**: スタートレック（Star Trek）に登場する架空の言語
- **言語コード**: `tlh` (ISO 639-2)
- **使用者**: クリンゴン帝国（架空の宇宙帝国）
- **特徴**: 言語学者によって設計された完全な文法体系を持つ

実際に約3,000語の語彙と文法規則が存在し、熱心なファンによって学習されている。

---

## 思考プロセス

### ステップ1: 言語ファイルの構造を理解

まず、通常の言語ファイルのパスを確認:

```javascript
// DevTools の Network タブで観察
GET /assets/i18n/en.json
GET /assets/i18n/de_DE.json
```

### ステップ2: 言語コードのパターンを分析

| パターン | 例 | 説明 |
|----------|-----|------|
| `xx` | `en`, `de` | 言語コードのみ |
| `xx_YY` | `de_DE`, `ja_JP` | 言語コード + 国コード |
| `xxx_YY` | `tlh_AA` | 3文字言語コード + 国コード |

### ステップ3: 隠し言語の推測

開発者がイースターエッグとして追加しそうな言語:
- `tlh` - クリンゴン語 (Star Trek)
- `qya` - クウェンヤ (Lord of the Rings)
- `jbo` - ロジバン (論理言語)
- `eo` - エスペラント

### ステップ4: ブルートフォース or ソースコード検索

```bash
# 言語コードリストでブルートフォース
for lang in tlh qya jbo eo; do
  curl -s "http://localhost:3000/assets/i18n/${lang}_AA.json" | head -c 100
done
```

または main.js で `tlh` を検索。

---

## 実行手順

### 方法1: 直接アクセス

ブラウザのアドレスバーに入力:

```
http://localhost:3000/assets/i18n/tlh_AA.json
```

JSON ファイルがダウンロードされればチャレンジ解決。

### 方法2: JavaScript で取得

```javascript
// Console で実行
fetch('/assets/i18n/tlh_AA.json')
  .then(r => {
    if (r.ok) {
      console.log('✅ Klingon language file found!');
      return r.json();
    }
    throw new Error('Not found');
  })
  .then(data => {
    console.log('Klingon translations:', data);
    console.log('Sample:', {
      LANGUAGE: data.LANGUAGE,
      NAV_SEARCH: data.NAV_SEARCH,
      TITLE_LOGIN: data.TITLE_LOGIN
    });
  });
```

### 方法3: 言語セレクターを改ざん

```javascript
// Console で実行: 言語セレクターにクリンゴン語を追加
(async () => {
  // i18n サービスを取得
  const i18nService = document.querySelector('app-root')
    ?.__ngContext__
    ?.find(x => x?.translate)?.__ngContext__
    ?.find(x => x?.use);

  if (i18nService) {
    await i18nService.use('tlh_AA');
    console.log('Language changed to Klingon!');
  } else {
    // 手動でローカルストレージを変更
    localStorage.setItem('language', 'tlh_AA');
    location.reload();
  }
})();
```

---

## 発見したファイルの内容

### tlh_AA.json の構造

```json
{
  "LANGUAGE": "tlhIngan",
  "NAV_SEARCH": "tu'",
  "NAV_COMPLAIN": "bep",
  "NAV_CONTACT": "ngu'",
  "NAV_ABOUT": "jwIj",
  "TITLE_LOGIN": "yI'el",
  "TITLE_LOGOUT": "bImeH",
  "TITLE_REGISTRATION": "pegh",
  "LABEL_EMAIL": "QIn",
  "LABEL_PASSWORD": "pegh mu'",
  "BTN_LOGIN": "yI'el",
  "BTN_LOGOUT": "bImeH",
  "PLACEHOLDER_SEARCH": "tu'..."
}
```

### 翻訳例

| 英語 | クリンゴン語 | 発音 |
|------|-------------|------|
| Login | yI'el | イーエル |
| Password | pegh mu' | ペフ・ムッ |
| Search | tu' | トゥッ |
| Contact | ngu' | ングッ |

---

## 解説

### なぜこのファイルが隠されているのか

```
┌─────────────────────────────────────────────────────────────────┐
│                    言語セレクターの仕組み                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ユーザーに見せる言語リスト:                                    │
│  ┌─────────────────────────────┐                               │
│  │ 🇺🇸 English                 │                               │
│  │ 🇩🇪 Deutsch                 │                               │
│  │ 🇯🇵 日本語                   │                               │
│  │ 🇨🇳 中文                     │                               │
│  └─────────────────────────────┘                               │
│                                                                 │
│  実際に存在するファイル:                                        │
│  en.json, de_DE.json, ja_JP.json, zh_CN.json, tlh_AA.json      │
│                                                    ↑            │
│                                             UI には非表示       │
│                                                                 │
│  問題: ファイルは誰でもアクセス可能                             │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### このチャレンジが教えること

1. **隠しファイルの発見**: ディレクトリ構造を推測して非公開ファイルを発見
2. **列挙攻撃**: 言語コードをブルートフォースで試行
3. **Security through Obscurity**: UI に表示しないだけでは保護にならない

### 実際の被害シナリオ

| シナリオ | リスク |
|----------|--------|
| **テスト翻訳の発見** | 未公開機能の名前がバレる |
| **開発者コメント** | 翻訳ファイルに機密コメント |
| **API キー漏洩** | 翻訳に外部サービスのキーが含まれる |
| **バージョン情報** | 開発中の機能名からロードマップ推測 |

### 対策

| 対策 | 説明 |
|------|------|
| **アクセス制御** | 使用可能な言語のみ提供 |
| **動的生成** | サーバー側で言語ファイルを生成 |
| **ホワイトリスト** | リクエスト可能な言語コードを制限 |
| **不要ファイル削除** | 本番環境からテストファイルを除外 |

```typescript
// 安全な実装例 (サーバーサイド)
const ALLOWED_LANGUAGES = ['en', 'de_DE', 'ja_JP', 'zh_CN'];

app.get('/assets/i18n/:lang.json', (req, res) => {
  const { lang } = req.params;

  // ホワイトリストチェック
  if (!ALLOWED_LANGUAGES.includes(lang)) {
    return res.status(404).json({ error: 'Language not found' });
  }

  res.sendFile(`i18n/${lang}.json`);
});
```

---

## 完全な攻撃コード

```javascript
// Console で実行: 全言語ファイルを列挙
(async () => {
  const languageCodes = [
    // 一般的な言語コード
    'en', 'de', 'fr', 'es', 'it', 'ja', 'ko', 'zh',
    'de_DE', 'fr_FR', 'es_ES', 'ja_JP', 'zh_CN', 'zh_TW',
    // 架空言語
    'tlh_AA', 'qya_AA', 'jbo_AA', 'eo_AA',
    // テスト用
    'test', 'dev', 'debug', 'xx_XX'
  ];

  const foundLanguages = [];

  for (const lang of languageCodes) {
    try {
      const res = await fetch(`/assets/i18n/${lang}.json`);
      if (res.ok) {
        const data = await res.json();
        foundLanguages.push({
          code: lang,
          name: data.LANGUAGE || lang,
          keys: Object.keys(data).length
        });
      }
    } catch (e) { /* ignore */ }
  }

  console.table(foundLanguages);
  console.log('Hidden languages:',
    foundLanguages.filter(l => !['en', 'de', 'fr', 'es', 'it', 'ja', 'zh'].some(c => l.code.startsWith(c)))
  );

  return foundLanguages;
})();
```

---

## 参考リンク

- [ISO 639-2 Language Codes](https://www.loc.gov/standards/iso639-2/php/code_list.php)
- [Klingon Language Institute](https://www.kli.org/)
- [OWASP - Information Disclosure](https://owasp.org/www-community/Improper_Error_Handling)
- [Angular i18n Guide](https://angular.io/guide/i18n)

## 関連チャレンジ

- [Score Board](../difficulty-1/score-board.md) - 隠しページの発見
- [Easter Egg](../difficulty-4/easter-egg.md) - 隠しコンテンツの発見
- [Blockchain Hype](blockchain-hype.md) - 隠しルートの発見
