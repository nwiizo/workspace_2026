pub use design_gate_core::{BlindSpot, BlindSpotManifest};

pub(crate) fn build(
    parse_failures: usize,
    metadata_failed: bool,
    feature_count: usize,
) -> BlindSpotManifest {
    let combinations = if feature_count >= 128 {
        None
    } else {
        Some(1u128 << feature_count)
    };
    let mut notes = vec![
        "No build is performed; this tool lists static risk candidates before cargo-hack or CI compilation.".to_string(),
        "untested-cfg-path evaluates only two synthetic points: default features and all features. CI workflows, target triples, and custom cargo commands are not parsed.".to_string(),
        "Rust type resolution is not performed; public signature exposure is matched from CST type paths.".to_string(),
        "Issue usage is type-specific: for default-leak it counts risky default entries, while public API diagnostics count matched items. Dependency downstream crate count is not currently part of severity.".to_string(),
        "untested-cfg-path can overlap mutually exclusive feature pairs because the two-point default/all-features model intentionally does not solve full feature constraints.".to_string(),
    ];
    let mut notes_ja = vec![
        "ビルドは実行しません。このツールは cargo-hack や CI コンパイルの前に静的なリスク候補を列挙します。".to_string(),
        "untested-cfg-path は default features と all features の 2 点だけを合成評価します。CI workflow、target triple、独自 cargo command は解析しません。".to_string(),
        "Rust の型解決は行いません。公開 signature 露出は CST の type path から照合します。".to_string(),
        "issue の usage は種別ごとに意味が異なります。default-leak では risky な default entry 数、公開 API 診断では一致 item 数です。依存先を使う下流 crate 数は現時点の severity に含みません。".to_string(),
        "untested-cfg-path は相互排他 feature ペアと重なることがあります。default/all-features の 2 点モデルは完全な feature 制約解決を意図していません。".to_string(),
    ];
    if let Some(value) = combinations {
        if feature_count >= 8 {
            notes.push(format!(
                "Feature powerset size is estimated at 2^{feature_count} = {value} combinations."
            ));
            notes_ja.push(format!(
                "feature 組合せ数は 2^{feature_count} = {value} 通りと推定されます。"
            ));
        }
    } else {
        notes.push(format!(
            "Feature powerset size is larger than this report prints exactly: 2^{feature_count}."
        ));
        notes_ja.push(format!(
            "feature 組合せ数は正確な表示上限を超えています: 2^{feature_count}。"
        ));
    }
    if metadata_failed {
        notes.push(
            "Full cargo dependency metadata failed; dependency default feature diagnostics are incomplete."
                .to_string(),
        );
        notes_ja.push(
            "依存を含む full cargo metadata に失敗したため、依存 crate の default feature 診断は不完全です。"
                .to_string(),
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
                id: "cfg-evaluation-model".to_string(),
                description: "Only feature predicates are evaluated. Non-feature cfg predicates are treated as outside the static model.".to_string(),
                description_ja: "feature 条件だけを評価します。feature 以外の cfg 条件は静的モデル外として扱います。".to_string(),
            },
            BlindSpot {
                id: "cross-file-cfg-mod-propagation".to_string(),
                description: "`#[cfg(feature)] mod x;` on a parent file is not propagated into x.rs during source inspection.".to_string(),
                description_ja: "親ファイルの `#[cfg(feature)] mod x;` は x.rs の解析へ伝播しません。".to_string(),
            },
            BlindSpot {
                id: "macro-generated-code".to_string(),
                description: "Macro expansion is not performed, so generated cfgs, public items, or compile_error! guards may be missed.".to_string(),
                description_ja: "マクロ展開は行わないため、生成された cfg、公開 item、compile_error! guard は漏れる可能性があります。".to_string(),
            },
            BlindSpot {
                id: "workspace-feature-unification".to_string(),
                description: "Cargo feature unification across arbitrary downstream workspaces is approximated from the current package metadata only.".to_string(),
                description_ja: "任意の下流 workspace による Cargo feature unification は、現在の package metadata だけから近似します。".to_string(),
            },
        ],
        notes,
        notes_ja,
    }
}
