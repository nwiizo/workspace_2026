# 要求ベースラインと追跡表

- Baseline: `candidate-0.1`
- Evidence status: 2026-08-21に自動ゲートを確認。QR-4の15分目標は代表レビュー担当者による評価待ち
- Decision owner: 実案件ではプロダクト責任者の個人名を記録する
- Scope: 単一プロセス内の口座間送金ドメイン
- Source: `DESIGN.md`で整理した20プラクティス

## 前提

- A-1: `ActorId`は上流で認証済みの主体として渡される。
- A-2: 口座所有者はこのサンプルの実行中に変わらない。
- A-3: 1つの`Ledger`への操作はRustの`&mut`により直列化される。
- A-4: プロセス障害をまたぐ再送制御には永続化が必要だが、このサンプルの範囲外とする。

## 機能要求

### FR-1: 未信頼入力を解析する

- Statement: WHEN the system receives transfer bytes THEN the system SHALL reject input over 128 bytes and SHALL return `ParsedTransfer` only for exactly four decimal fields: request ID, source account ID, destination account ID, and amount.
- Business rules: BR-1, BR-2
- Priority: Must
- Acceptance: 0、上限超過、同一口座、非UTF-8、フィールド過不足、巨大入力を拒否する。有効入力は値を保持する。
- Evidence: `ParsedTransfer::parse`, `tests/acceptance.rs`, `tests/adversarial.rs`, `fuzz/fuzz_targets/parse_transfer.rs`

### FR-2: 送金元所有者だけを認可する

- Statement: WHEN an actor requests authorization IF the actor owns the source account and both accounts exist THEN the system SHALL return an `AuthorizedTransfer`; OTHERWISE it SHALL return an error without changing ledger state.
- Business rules: BR-3, BR-8
- Priority: Must
- Acceptance: 所有者は認可され、別主体と存在しない口座は拒否される。
- Evidence: `Ledger::authorize`, `tests/acceptance.rs`, `tests/adversarial.rs`

### FR-3: 残高を原子的に移す

- Statement: WHEN an authorized transfer executes IF the source has enough funds and the destination can receive the amount THEN the system SHALL subtract and add the exact amount atomically; OTHERWISE it SHALL leave all state unchanged.
- Business rules: BR-4, BR-5, BR-8
- Priority: Must
- Acceptance: 成功時の差分と合計保存、残高不足時とオーバーフロー時の無変更を確認する。
- Evidence: `AuthorizedTransfer::execute`, `transition`, `tests/acceptance.rs`, `tests/properties.rs`, Kani harnesses

### FR-4: 再送を一度だけ反映する

- Statement: WHEN a completed request is submitted again IF its request ID and contents match THEN the system SHALL return `Replayed` without changing balances; IF contents differ THEN it SHALL reject the request.
- Business rules: BR-6, BR-7, BR-8
- Priority: Must
- Acceptance: 同一再送で残高が不変となり、要求IDの改ざん再利用が拒否される。
- Evidence: `AuthorizedTransfer::execute`, `tests/acceptance.rs`, `tests/properties.rs`, `tests/adversarial.rs`

## 品質属性要求

### QR-1: 任意入力に対する堅牢性

- Statement: WHEN arbitrary bytes are passed to the parser THEN the system SHALL not panic or invoke unsafe Rust.
- Priority: Must
- Acceptance: `#![forbid(unsafe_code)]`を有効にし、Fuzz targetを60秒以上実行してpanicがない。
- Evidence: crate attribute, Fuzz target, 61秒のFuzzing実行、`scripts/verify.sh`

### QR-2: 残高保存のモデル検査

- Statement: FOR every pair of `u64` balances and every valid transfer amount, a successful transition SHALL preserve their sum when observed as `u128`, and an error SHALL correspond to insufficient source funds or destination overflow.
- Priority: Must
- Acceptance: Kani harnessが反例なしで完了する。
- Evidence: `successful_transition_preserves_total`, `transition_result_matches_preconditions`

### QR-3: 性能

- Statement: WHEN 100,000 valid transfers execute sequentially in a release build on the development machine THEN the workload SHALL finish within 2 seconds.
- Priority: Should
- Acceptance: 無視指定の性能テストとbenchmarkをrelease buildで実行し、環境と結果を記録する。
- Evidence: `tests/performance.rs`, `benches/transfer.rs`

### QR-4: レビュー可能性

- Statement: WHEN a reviewer inspects a change THEN every changed behavior SHALL link a requirement ID to its public type or function, test, and specialized verification command.
- Priority: Must
- Acceptance: 追跡表に空欄がなく、公開項目のドキュメント警告をdenyし、rustdoc testが通る。
- Evidence: この文書、`#![deny(missing_docs)]`, `cargo test --doc`

## 業務ルール

| ID | Rule | Enforced by |
|---|---|---|
| BR-1 | 送金額は1以上1,000,000以下である | `TransferAmount` |
| BR-2 | 送金元と送金先は異なる | `ParsedTransfer` |
| BR-3 | 送金元所有者だけが認可される | `Ledger::authorize` |
| BR-4 | 送金元残高を0未満にしない | `transition` |
| BR-5 | 送金先残高を`u64::MAX`より大きくしない | `transition` |
| BR-6 | 同一要求の再送を二重反映しない | completed request map |
| BR-7 | 要求IDと異なる内容の組み合わせを拒否する | completed request map |
| BR-8 | 拒否時は台帳を変更しない | compute-before-commitとテスト |

## 証拠の追跡表

| Requirement | Type/API | Example | Property | Kani | Fuzz | Performance | Adversarial |
|---|---|---|---|---|---|---|---|
| FR-1 | `ParsedTransfer::parse` | `acceptance` | parser property | n/a | parser target | n/a | malformed corpus |
| FR-2 | `Ledger::authorize` | `acceptance` | authorization immutability | n/a | n/a | n/a | actor/account abuse |
| FR-3 | `AuthorizedTransfer::execute` | `acceptance` | sum and atomicity | 2 harnesses | n/a | transfer workload | overflow/funds |
| FR-4 | `Disposition` | `acceptance` | retry idempotency | n/a | n/a | n/a | request ID reuse |
| QR-1 | parser and crate attributes | n/a | arbitrary byte vectors | n/a | parser target | n/a | oversized/non-UTF-8 |
| QR-2 | `transition` | boundary cases | generated values | 2 harnesses | n/a | n/a | n/a |
| QR-3 | transfer path | n/a | n/a | n/a | n/a | test and benchmark | n/a |
| QR-4 | all public items | rustdoc | n/a | n/a | n/a | n/a | trace review |

`n/a`は手法の漏れではなく、その要求の誤りを見つける用途に選ばなかったことを表す。すべての要求へ
すべての検証技法を当てると、実行時間と保守負担だけが増え、判断しにくくなる。
