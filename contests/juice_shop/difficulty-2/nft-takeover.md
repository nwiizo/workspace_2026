# NFT Takeover ✅

**難易度:** ⭐⭐
**カテゴリ:** Sensitive Data Exposure
**目標:** Soul Bound Token (NFT) を持つウォレットをテイクオーバーする

## 思考プロセス

1. `/juicy-nft` ページでプライベートキーを要求される
2. ユーザーフィードバックにシードフレーズが漏洩している
3. シードフレーズから秘密鍵を導出
4. 秘密鍵を入力してウォレットにアクセス

## 実行手順

### Step 1: NFT ページを発見

```
http://localhost:3000/#/juicy-nft
```

About Us ページのコメントに `/juicy-nft` への言及がある。

### Step 2: シードフレーズを発見

フィードバックコメントに漏洩したシードフレーズ:

```
purpose betray marriage blame crunch monitor spin slide donate sport lift clutch
```

### Step 3: シードフレーズから秘密鍵を導出

[Ian Coleman's BIP39 Tool](https://iancoleman.io/bip39/) を使用:

1. シードフレーズを入力
2. Derivation Path: `m/44'/60'/0'/0` (Ethereum)
3. 最初のアドレスの秘密鍵を取得

**秘密鍵:**
```
0x5bcc3e9d38baa06e7bfaab80ae5957bbe8ef059e640311d7d6d465e6bc948e3e
```

### Step 4: ウォレットにアクセス

```javascript
// /juicy-nft ページで秘密鍵を入力
browser_navigate({ url: "http://localhost:3000/#/juicy-nft" });
browser_type({ ref: "秘密鍵入力欄", text: "0x5bcc3e9d38baa06e7bfaab80ae5957bbe8ef059e640311d7d6d465e6bc948e3e" });
browser_click({ ref: "Authenticate ボタン" });
```

## 解説

### 脆弱性の原因

1. **シードフレーズの漏洩**: ユーザーがフィードバックにシードフレーズを投稿
2. **BIP39/BIP44 の標準化**: シードフレーズから秘密鍵が一意に導出可能
3. **認証の脆弱性**: 秘密鍵だけでウォレットにアクセス可能

### なぜ危険か

- シードフレーズは**絶対に公開してはいけない**
- 12/24単語のシードから全ての秘密鍵が導出される
- 一度漏洩すると資産が全て盗まれる可能性

### 対策

1. **教育**: シードフレーズの重要性をユーザーに周知
2. **検出**: シードフレーズパターンをフィルタリング
3. **ハードウェアウォレット**: オフラインでシードフレーズを保管

## 関連チャレンジ

- [Wallet Depletion](../difficulty-5-6/wallet-depletion.md) - ウォレットから資金を引き出す
- [Blockchain Hype](../difficulty-5-6/blockchain-hype.md) - トークンセールページの発見
- [Web3 Sandbox](../difficulty-1/web3-sandbox.md) - スマートコントラクトサンドボックス

## 参考リンク

- [BIP39 Mnemonic Code](https://github.com/bitcoin/bips/blob/master/bip-0039.mediawiki)
- [BIP44 Derivation Paths](https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki)
- [Ian Coleman BIP39 Tool](https://iancoleman.io/bip39/)
