pub use design_gate_core::{BlindSpot, BlindSpotManifest};

pub(crate) fn build(
    parse_failures: usize,
    metadata_failed: bool,
    edition_fallback_2024: bool,
) -> BlindSpotManifest {
    let mut notes = Vec::new();
    let mut notes_ja = Vec::new();
    if metadata_failed {
        notes.push(
            "cargo metadata failed; project name and edition dependent output may be approximate."
                .to_string(),
        );
        notes_ja.push(
            "cargo metadata に失敗したため、project 名と edition に依存する出力は近似です。"
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
                id: "name-resolution".to_string(),
                description: "Trait, impl, and concrete I/O dependency matching is CST/name based; type aliases and re-exports are not resolved.".to_string(),
                description_ja: "trait、impl、具象 I/O 依存の照合は CST/名前ベースです。type alias や re-export は解決しません。".to_string(),
            },
            BlindSpot {
                id: "unmockable-boundary-heuristic".to_string(),
                description: "Unmockable boundary detection only flags known concrete std I/O, process, and time types in public signatures.".to_string(),
                description_ja: "差し替え不能な境界の検出は、公開シグネチャ内の既知の std I/O・process・time 具象型だけを報告します。".to_string(),
            },
            BlindSpot {
                id: "future-extension-intent".to_string(),
                description: "Zero or one production implementation can still be intentional design; declare known intent in trait-surface.toml.".to_string(),
                description_ja: "production 実装が 0 または 1 つの trait でも意図的な設計の場合があります。trait-surface.toml で intent を宣言してください。".to_string(),
            },
            BlindSpot {
                id: "async-trait-macro".to_string(),
                description: "Traits annotated with async_trait are treated as macro-rewritten for async-method object-safety checks; other proc-macro rewrites are not expanded.".to_string(),
                description_ja: "async_trait 属性付き trait は async method の object-safety 検査でマクロ変換後として扱います。その他の proc macro 変換は展開しません。".to_string(),
            },
        ],
        notes,
        notes_ja,
    }
}
