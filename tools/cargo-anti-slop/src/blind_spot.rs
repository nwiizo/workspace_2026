pub use design_gate_core::{BlindSpot, BlindSpotManifest};

pub(crate) fn build(
    parse_failures: usize,
    metadata_failed: bool,
    edition_fallback_2024: bool,
    git_volatility_unavailable: bool,
    git_shallow: bool,
    orphan_scan_skipped: bool,
    orphan_scan_unreliable: bool,
) -> BlindSpotManifest {
    let mut notes = vec![
        "No type resolution: erased types are matched from simple use-alias resolution; re-exports, glob imports, and newtype wrappers concealing Value/Any are missed.".to_string(),
        "widen-then-assert tracks single-binding flows inside one function; erasure that crosses function or module boundaries is not followed.".to_string(),
        "default-swallow relies on textual fallible markers in the receiver chain; Results produced by helper functions are not traced.".to_string(),
        "chained-cast lossiness is computed only between the written cast types; the source expression's own type is unknown.".to_string(),
        "anti-slop rules with no Rust equivalent are not mapped: no-module-mocking (module mocking does not exist in Rust) and no-reflect-apply/get (no runtime reflection).".to_string(),
        "Division with rbp-lint: struct-field walls (bool-option-pair, raw-id-field, status-string-field, pub-field-newtype) and unwrap/expect/panic/SAFETY belong to rbp-lint; this tool covers function boundaries and flows.".to_string(),
        "bool-validation matches function names containing `valid` or starting with `verify`; crypto-style verify APIs that legitimately return bool need suppression.".to_string(),
        "raw-id-params and constructor-bypass are single-file syntactic checks: type aliases for raw ids and impls living in other files are not resolved.".to_string(),
    ];
    let mut notes_ja = vec![
        "型解決は行いません: 型消去は単純な use alias 解決による構文照合です。re-export、glob import、Value/Any を隠す newtype は見逃します。".to_string(),
        "widen-then-assert は 1 関数内・単一 binding の流れのみ追跡します。関数やモジュールをまたぐ型消去は追いません。".to_string(),
        "default-swallow は receiver チェーン内の fallible marker への文字列照合です。helper 関数が返す Result は追跡しません。".to_string(),
        "chained-cast の損失判定は書かれた cast 型の間のみで行います。元の式自体の型は不明です。".to_string(),
        "Rust に等価物がない anti-slop ルールは対象外です: no-module-mocking (Rust に module mocking はない) と no-reflect-apply/get (実行時リフレクションがない)。".to_string(),
        "rbp-lint との分業: struct フィールドの壁 (bool-option-pair、raw-id-field、status-string-field、pub-field-newtype) と unwrap/expect/panic/SAFETY は rbp-lint、本ツールは関数境界とフローを担当します。".to_string(),
        "bool-validation は関数名に `valid` を含むか `verify` で始まるものを対象にします。bool を返すのが正当な crypto 系 verify API は suppress してください。".to_string(),
        "raw-id-params と constructor-bypass は単一ファイル内の構文検査です。生 ID の type alias や別ファイルの impl は解決しません。".to_string(),
    ];
    if git_shallow {
        notes.push(
            "shallow git clone detected (CI checkouts default to depth 1); the volatility axis was disabled to avoid uniform severity inflation.".to_string(),
        );
        notes_ja.push(
            "shallow clone を検出しました (CI checkout の既定は depth 1)。severity が一律に水増しされるため volatility 軸を無効化しました。".to_string(),
        );
    } else if git_volatility_unavailable {
        notes.push(
            "git history was unavailable; severity used impact and condition axes only."
                .to_string(),
        );
        notes_ja.push(
            "git 履歴を利用できないため、severity は影響と発生条件の 2 軸で評価しました。"
                .to_string(),
        );
    }
    if orphan_scan_skipped {
        notes.push(
            "orphan-file scan was skipped: it runs only for a single-crate src/ layout, not workspace roots or single files.".to_string(),
        );
        notes_ja.push(
            "orphan-file 走査をスキップしました。単一 crate の src/ 構成でのみ実行し、workspace root や単一ファイルは対象外です。".to_string(),
        );
    }
    if orphan_scan_unreliable {
        notes.push(
            "orphan-file scan was aborted: most files looked unreachable, which indicates a macro-driven module structure this scan cannot resolve.".to_string(),
        );
        notes_ja.push(
            "orphan-file 走査を中止しました。大半のファイルが未到達に見えるのは、この走査が解決できないマクロ駆動のモジュール構成を示すためです。".to_string(),
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
                id: "macro-generated-code".to_string(),
                description: "Macro-generated functions, casts, and bindings are not expanded; json!-style macros are matched by path only.".to_string(),
                description_ja: "マクロ生成された関数、cast、binding は展開しません。json! 系マクロは path のみで照合します。".to_string(),
            },
            BlindSpot {
                id: "type-resolution".to_string(),
                description: "Erased types (serde_json::Value, dyn Any, map value types) are inferred from syntax and simple use aliases rather than rustc type information.".to_string(),
                description_ja: "型消去された型 (serde_json::Value、dyn Any、map の value 型) は rustc の型情報ではなく構文と単純な use alias から推定します。".to_string(),
            },
            BlindSpot {
                id: "custom-any-traits".to_string(),
                description: "A user-defined trait named Any is indistinguishable from std::any::Any; a Value type from an unresolved glob import is not recognized.".to_string(),
                description_ja: "ユーザー定義の Any trait は std::any::Any と区別できません。未解決の glob import 由来の Value 型は認識しません。".to_string(),
            },
            BlindSpot {
                id: "error-boundary-downcast".to_string(),
                description: "runtime-downcast lowers the score for error-looking receivers (err/error/cause/source) instead of proving an error boundary, mirroring anti-slop's `cause` exception.".to_string(),
                description_ja: "runtime-downcast は error 風の receiver (err/error/cause/source) を証明なしに低スコア化します。anti-slop の `cause` 例外に対応する近似です。".to_string(),
            },
            BlindSpot {
                id: "trait-impl-signatures".to_string(),
                description: "erased-signature skips trait impl methods because the trait dictates the signature; the in-repo trait declaration is still checked.".to_string(),
                description_ja: "erased-signature は trait impl のメソッドを対象外にします。signature は trait 側が決めるためで、リポジトリ内の trait 宣言自体は検査します。".to_string(),
            },
            BlindSpot {
                id: "vague-name-lexicon".to_string(),
                description: "vague-symbol-name is a lexical check against a configurable word list; domain-appropriate names on the list need config or suppression.".to_string(),
                description_ja: "vague-symbol-name は設定可能な語彙リストへの字句照合です。ドメイン上妥当な名前は config か suppress で除外してください。".to_string(),
            },
            BlindSpot {
                id: "clippy-overlap".to_string(),
                description: "Single lossy casts and bare `.ok();` overlap with clippy's cast_* and unused_result_ok restriction lints; this tool adds chain detection, fallible-marker context, grading, and baseline ratchets rather than replacing clippy.".to_string(),
                description_ja: "単発の lossy cast と素の `.ok();` は clippy の cast_* / unused_result_ok (restriction) と重複します。本ツールは clippy の代替ではなく、連鎖検出・fallible marker 文脈・Grade・baseline ratchet を足します。".to_string(),
            },
        ],
        notes,
        notes_ja,
    }
}
