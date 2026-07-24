use design_gate_core::{BlindSpot, BlindSpotManifest};

pub fn build(parse_failures: usize) -> BlindSpotManifest {
    let mut manifest = BlindSpotManifest {
        blind_spots: vec![
            BlindSpot {
                id: "not-strict-semver-audit".to_string(),
                description: "cargo-api-drift is a fast CST and git-diff classifier. Use cargo-semver-checks for strict, rustdoc-JSON based semver audits.".to_string(),
                description_ja: "cargo-api-drift は CST と git diff による高速な分類器です。厳密な rustdoc JSON ベースの semver audit には cargo-semver-checks を使ってください。".to_string(),
            },
            BlindSpot {
                id: "macro-public-api".to_string(),
                description: "macro_rules! exports and proc-macro public signatures are not tracked.".to_string(),
                description_ja: "macro_rules! export と proc-macro の公開シグネチャは追跡しません。".to_string(),
            },
            BlindSpot {
                id: "name-resolution".to_string(),
                description: "Re-export and module visibility tracking is approximate and does not perform full Rust name resolution.".to_string(),
                description_ja: "再エクスポートとモジュール可視性の追跡は近似であり、完全な Rust 名前解決は行いません。".to_string(),
            },
            BlindSpot {
                id: "cfg-feature-matrix".to_string(),
                description: "Feature-specific cfg API surfaces are parsed as source text, not as a full cargo feature matrix.".to_string(),
                description_ja: "feature ごとの cfg API surface はソーステキストとして解析し、完全な cargo feature matrix としては評価しません。".to_string(),
            },
            BlindSpot {
                id: "const-static-values".to_string(),
                description: "pub const and pub static item existence and type signatures are tracked, but initializer expression changes are not classified.".to_string(),
                description_ja: "pub const / pub static の存在と型シグネチャは追跡しますが、初期化式だけの変更は分類しません。".to_string(),
            },
            BlindSpot {
                id: "type-alias-exposure".to_string(),
                description: "Public exposure through type aliases, such as pub type Alias = pub(crate) Inner, is not resolved transitively.".to_string(),
                description_ja: "pub type Alias = pub(crate) Inner のような型エイリアス経由の公開露出は推移的に解決しません。".to_string(),
            },
        ],
        notes: Vec::new(),
        notes_ja: Vec::new(),
    };
    if parse_failures > 0 {
        manifest.notes.push(format!(
            "{parse_failures} file(s) had parse errors; classification may be incomplete."
        ));
        manifest.notes_ja.push(format!(
            "{parse_failures} ファイルに parse error があり、分類が不完全な可能性があります。"
        ));
    }
    manifest
}
