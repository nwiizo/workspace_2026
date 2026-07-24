use crate::analyzer::Runtime;

pub use design_gate_core::{BlindSpot, BlindSpotManifest};

pub(crate) fn build(
    parse_failures: usize,
    metadata_failed: bool,
    edition_fallback_2024: bool,
    git_volatility_unavailable: bool,
    runtime: Runtime,
) -> BlindSpotManifest {
    let mut notes = vec![
        "No type resolution: async traits, re-exports, wrapper types, and cross-function effects are matched syntactically.".to_string(),
        "Simple use-import aliases are resolved for spawn, timeout, JoinSet/select checks, std::fs-style blocking calls, and drop paths; glob imports and re-exports are not.".to_string(),
        "guard-across-await approximates guard liveness from let/if-let bindings, exact drop(var) calls, and lexical block ranges; try_lock/try_read/try_write cannot distinguish tokio guards from std/parking_lot guards and are scored lower.".to_string(),
        "missing-timeout is intentionally approximate: process-local channel send/recv is filtered heuristically, but client-level default timeout configured through builders is not followed across functions.".to_string(),
        "blocking-in-async does not trace blocking behavior through helper functions.".to_string(),
        "Issue keys use rel_path:Type::method when an enclosing impl type is visible; duplicate functions inside the same identity still fall back to #N suffixes.".to_string(),
    ];
    let mut notes_ja = vec![
        "型解決は行いません: async trait、re-export、wrapper type、関数をまたぐ効果は構文的に照合します。".to_string(),
        "単純な use import alias は spawn、timeout、JoinSet/select 判定、std::fs 形式の blocking call、drop path で解決します。glob import と re-export は対象外です。".to_string(),
        "guard-across-await は let/if-let binding、正確な drop(var)、字句 block 範囲から guard 生存期間を近似します。try_lock/try_read/try_write は tokio guard と std/parking_lot guard を区別できないため低めに採点します。".to_string(),
        "missing-timeout は意図的な近似です。プロセス内 channel の send/recv はヒューリスティックに除外しますが、builder で設定された client-level default timeout は関数をまたいで追跡しません。".to_string(),
        "blocking-in-async は helper 関数の内側に隠れた blocking 動作を追跡しません。".to_string(),
        "issue key は enclosing impl 型が見える場合 rel_path:Type::method を使います。同一 identity 内の重複のみ #N suffix にフォールバックします。".to_string(),
    ];
    if runtime != Runtime::Tokio {
        notes.push(format!(
            "{} runtime was requested; only Tokio patterns are analyzed in Wave 1.",
            runtime.id()
        ));
        notes_ja.push(format!(
            "{} runtime が指定されました。Wave 1 では Tokio pattern のみ解析します。",
            runtime.id()
        ));
    }
    if git_volatility_unavailable {
        notes.push(
            "git history was unavailable; severity used impact and condition axes only."
                .to_string(),
        );
        notes_ja.push(
            "git 履歴を利用できないため、severity は影響と発生条件の 2 軸で評価しました。"
                .to_string(),
        );
    }
    if metadata_failed {
        notes.push(
            "cargo metadata failed; package metadata dependent output may be approximate."
                .to_string(),
        );
        notes_ja.push(
            "cargo metadata に失敗したため、package metadata に依存する出力は近似です。"
                .to_string(),
        );
    }
    if edition_fallback_2024 {
        notes.push(
            "Cargo edition could not be read; sources were parsed as Edition2024.".to_string(),
        );
        notes_ja.push(
            "Cargo edition を読み取れなかったため、Edition2024 としてパースしました。".to_string(),
        );
    }
    if parse_failures > 0 {
        notes.push(format!(
            "{parse_failures} Rust source file(s) could not be parsed cleanly."
        ));
        notes_ja.push(format!(
            "{parse_failures} 件の Rust ソースを完全には解析できませんでした。"
        ));
    }
    BlindSpotManifest {
        blind_spots: vec![
            BlindSpot {
                id: "macro-generated-async".to_string(),
                description: "Macro-generated async functions, select branches, and spawn calls are not expanded.".to_string(),
                description_ja: "マクロ生成された async fn、select 分岐、spawn 呼び出しは展開しません。".to_string(),
            },
            BlindSpot {
                id: "type-resolution".to_string(),
                description: "Guard, runtime, and external I/O types are inferred from syntax rather than rustc type information.".to_string(),
                description_ja: "guard、runtime、外部 I/O 型は rustc の型情報ではなく構文から推定します。".to_string(),
            },
            BlindSpot {
                id: "import-alias-scope".to_string(),
                description: "Only simple use aliases and grouped imports are expanded; glob imports, re-exports, and type aliases remain unresolved.".to_string(),
                description_ja: "単純な use alias と group import のみ展開します。glob import、re-export、type alias は未解決です。".to_string(),
            },
            BlindSpot {
                id: "try-lock-ambiguity".to_string(),
                description: "try_lock/try_read/try_write are synchronous syntax for both tokio and std-like locks, so findings may be conservative.".to_string(),
                description_ja: "try_lock/try_read/try_write は tokio と std 系 lock のどちらでも同期構文のため、保守的な検出になります。".to_string(),
            },
            BlindSpot {
                id: "timeout-heuristic".to_string(),
                description: "missing-timeout reports configured method names without proving whether the receiver is external communication; client-level default timeouts are not propagated.".to_string(),
                description_ja: "missing-timeout は receiver が外部通信かを証明せず、設定されたメソッド名で報告します。client-level default timeout は伝播しません。".to_string(),
            },
            BlindSpot {
                id: "channel-heuristic".to_string(),
                description: "tokio channel send/recv suppression relies on receiver text or nearby binding initializers containing channel markers.".to_string(),
                description_ja: "tokio channel send/recv の抑制は receiver テキストまたは近傍 binding initializer の channel marker に依存します。".to_string(),
            },
            BlindSpot {
                id: "cross-function-blocking".to_string(),
                description: "blocking-in-async does not infer that a locally defined helper function performs blocking work.".to_string(),
                description_ja: "blocking-in-async はローカル helper 関数内の blocking 動作を推論しません。".to_string(),
            },
        ],
        notes,
        notes_ja,
    }
}
