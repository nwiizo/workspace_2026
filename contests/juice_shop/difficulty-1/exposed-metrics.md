# Exposed Metrics ✅

**難易度:** ⭐
**カテゴリ:** 情報漏洩 / Security Misconfiguration
**目標:** サーバーの監視データ（メトリクス）を見つける

---

## 背景知識

### Prometheus メトリクスとは

Prometheus は、クラウドネイティブ環境で広く使われている**監視・アラートシステム**。サーバーは `/metrics` エンドポイントで自身の状態を公開し、Prometheus がそれを定期的に収集する。

```
┌─────────────────────────────────────────────────────────────────┐
│                     Prometheus アーキテクチャ                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  【正常な構成】                                                  │
│                                                                 │
│  ┌─────────────┐     収集      ┌─────────────────┐             │
│  │ Application │ ←──────────── │ Prometheus      │             │
│  │ /metrics    │               │ Server          │             │
│  └─────────────┘               └─────────────────┘             │
│         ↑                              ↓                        │
│    内部ネットワークからのみ      ダッシュボード（Grafana等）      │
│    アクセス可能                                                 │
│                                                                 │
│  【脆弱な構成（このチャレンジ）】                                 │
│                                                                 │
│  ┌─────────────┐                                               │
│  │ Application │                                               │
│  │ /metrics    │ ←── インターネットから誰でもアクセス可能！      │
│  └─────────────┘                                               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### メトリクスに含まれる情報

```
┌─────────────────────────────────────────────────────────────────┐
│                     漏洩する可能性のある情報                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  【システム情報】                                                │
│  - CPU 使用率、メモリ使用量                                     │
│  - プロセス数、スレッド数                                       │
│  - ファイルディスクリプタ数                                     │
│                                                                 │
│  【アプリケーション情報】                                        │
│  - リクエスト数、レスポンス時間                                  │
│  - エラー率、エラーの種類                                       │
│  - データベース接続プールの状態                                  │
│                                                                 │
│  【ビジネス情報】                                                │
│  - ユーザー数、アクティブセッション数                            │
│  - トランザクション量                                           │
│  - キュー内のジョブ数                                           │
│                                                                 │
│  【セキュリティ上の問題】                                        │
│  - 攻撃者がシステムの弱点を特定できる                           │
│  - 負荷の高い時間帯を狙った攻撃が可能                           │
│  - 内部構造（使用技術、バージョン等）が判明                      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 日常的な例え

会社のビルの警備員が、ビルの詳細な見取り図と警備ローテーション表を入口に貼っているようなもの。正当な訪問者には不要だし、泥棒には大助かり。

---

## 思考プロセス

### ステップ1: 一般的なエンドポイントを推測

```
「監視ツールが使うエンドポイントは？」
    ↓
「/metrics - Prometheus の標準」
「/health - ヘルスチェック」
「/debug - デバッグ情報」
「/status - ステータス情報」
```

### ステップ2: アクセスを試行

```
「http://localhost:3000/metrics にアクセス」
    ↓
「大量のテキストが表示された！」
    ↓
「認証なしでメトリクスが公開されている」
```

---

## 実行手順

1. ブラウザのアドレスバーに以下を入力:
   ```
   http://localhost:3000/metrics
   ```
2. 大量のテキストデータが表示されれば成功

### 表示される内容の例

```
# HELP process_cpu_user_seconds_total Total user CPU time spent in seconds.
# TYPE process_cpu_user_seconds_total counter
process_cpu_user_seconds_total 12.62

# HELP process_resident_memory_bytes Resident memory size in bytes.
# TYPE process_resident_memory_bytes gauge
process_resident_memory_bytes 89010176

# HELP http_requests_total Total number of HTTP requests
# TYPE http_requests_total counter
http_requests_total{method="GET",path="/api/Products",status="200"} 1523
http_requests_total{method="POST",path="/rest/user/login",status="401"} 89
```

---

## Juice Shop の脆弱なコードパターン

### 脆弱なコード（推定）

```typescript
// ❌ 脆弱なコード
// server.ts
import { collectDefaultMetrics, register } from 'prom-client'

// デフォルトメトリクスを有効化
collectDefaultMetrics()

// 認証なしで /metrics を公開
app.get('/metrics', (req, res) => {
  res.set('Content-Type', register.contentType)
  res.end(register.metrics())
})
```

### 問題点

1. **認証なし**: 誰でもアクセス可能
2. **ネットワーク制限なし**: 外部からもアクセス可能
3. **情報量が多すぎる**: デフォルトで多くの情報を公開

---

## 安全な実装

```typescript
// ✅ 安全なコード
// server.ts
import { collectDefaultMetrics, register } from 'prom-client'

collectDefaultMetrics()

// 1. 認証ミドルウェアを追加
app.get('/metrics',
  authenticateApiKey,  // API キー認証
  (req, res) => {
    res.set('Content-Type', register.contentType)
    res.end(register.metrics())
  }
)

// または IP ホワイトリスト
function restrictToInternalNetwork(req, res, next) {
  const clientIp = req.ip
  const allowedRanges = ['127.0.0.1', '10.0.0.0/8', '172.16.0.0/12', '192.168.0.0/16']

  if (!isInAllowedRange(clientIp, allowedRanges)) {
    return res.status(403).json({ error: 'Forbidden' })
  }
  next()
}

app.get('/metrics', restrictToInternalNetwork, (req, res) => {
  res.set('Content-Type', register.contentType)
  res.end(register.metrics())
})
```

### Kubernetes での対策

```yaml
# Ingress で /metrics を外部に公開しない
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: app-ingress
  annotations:
    nginx.ingress.kubernetes.io/server-snippet: |
      location /metrics {
        deny all;
        return 403;
      }
spec:
  rules:
    - host: app.example.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: app-service
                port:
                  number: 80
```

### 対策のチェックリスト

| チェック項目 | 説明 |
|-------------|------|
| ✅ **認証** | /metrics へのアクセスに認証を要求 |
| ✅ **ネットワーク制限** | 内部ネットワークからのみアクセス許可 |
| ✅ **Ingress 設定** | 外部公開時に /metrics を除外 |
| ✅ **情報量制限** | 本当に必要なメトリクスのみ公開 |

---

## 他の探索すべきエンドポイント

開発・運用系のエンドポイントは公開されていることが多い:

| エンドポイント | ツール/目的 | リスク |
|---------------|-------------|--------|
| `/metrics` | Prometheus | システム情報漏洩 |
| `/health` | ヘルスチェック | 稼働状況の漏洩 |
| `/debug/pprof` | Go プロファイラ | パフォーマンス情報 |
| `/actuator` | Spring Boot | 設定情報漏洩 |
| `/.well-known/*` | 各種メタデータ | 設定情報 |
| `/server-status` | Apache | サーバー統計 |
| `/nginx_status` | Nginx | 接続情報 |

---

## 解説

- これは Prometheus という監視ツール用のエンドポイント
- サーバーのCPU使用率やメモリ使用量などの内部情報が見える
- 本来は管理者だけがアクセスできるべき

**なぜこれが危険？**
- 攻撃者にシステムの内部状態を教えてしまう
- 負荷状況やエラー率などの情報が攻撃計画に悪用される可能性がある
- 使用技術やバージョンが判明し、既知の脆弱性を狙われる

---

## OWASP との関連

- **A05:2021 - Security Misconfiguration**: デフォルト設定のまま、または不適切な設定

---

## 関連チャレンジ

- [Error Handling](error-handling.md) - エラーメッセージからの情報漏洩
- [Security Policy](security-policy.md) - セキュリティポリシーの探索

## 参考リンク

- [Prometheus Best Practices - Security](https://prometheus.io/docs/operating/security/)
- [OWASP Security Misconfiguration](https://owasp.org/Top10/A05_2021-Security_Misconfiguration/)
- [CWE-200: Exposure of Sensitive Information](https://cwe.mitre.org/data/definitions/200.html)
