//! Hot-reloadable library crate.
//!
//! このクレートは dylib としてビルドされ、hot-lib-reloader によって
//! 実行中のアプリに動的にリロードされる。
//!
//! # 検証ポイント
//! - `#[unsafe(no_mangle)]` 関数の変更 → リロード反映
//! - State 構造体の状態保持（カウンターが維持されるか）
//! - 型変更時の挙動（フィールド追加/削除 → segfault の可能性）
//! - serde_json::Value によるシリアライズ回避パターン

use serde::{Deserialize, Serialize};

/// アプリケーション状態。bin 側で保持し、dylib の関数に渡す。
///
/// # 重要: 型レイアウトの制約
/// この構造体のフィールドを追加・削除すると、リロード時に
/// メモリレイアウトが不一致となり segfault する可能性がある。
/// 安全に変更するには serde_json::Value 経由のシリアライズが必要。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub counter: i64,
    pub message: String,
    // --- 検証: 以下のフィールドを追加してリロードすると何が起きるか試す ---
    // pub extra: f64,
}

impl State {
    pub fn new() -> Self {
        Self {
            counter: 0,
            message: String::from("Hello from hot-lib!"),
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

/// メインのビジネスロジック。この関数の中身を変更して保存すると、
/// hot-lib-reloader が dylib を再ビルド・リロードし、次のループで反映される。
///
/// 試してみること:
/// 1. counter の増分を +1 → +10 に変更
/// 2. message のフォーマットを変更
/// 3. 新しい計算ロジックを追加
#[unsafe(no_mangle)]
pub fn step(state: &mut State) {
    state.counter += 1;
    state.message = format!("counter = {} (try changing this string!)", state.counter);
}

/// 状態を表示する関数。表示フォーマットだけを変えたい場合はここを変更。
#[unsafe(no_mangle)]
pub fn render(state: &State) -> String {
    format!("[hot-lib] {}", state.message)
}

// ---------------------------------------------------------------------------
// serde_json::Value を使った安全なシリアライズパターン
// 型変更に耐性がある（segfault しない）が、パフォーマンスのオーバーヘッドあり
// ---------------------------------------------------------------------------

/// State を JSON 文字列にシリアライズして返す。
/// bin 側でデシリアライズすれば、dylib 側の型変更に安全に追従できる。
#[unsafe(no_mangle)]
pub fn step_serialized(state_json: &str) -> String {
    let result: Result<State, _> = serde_json::from_str(state_json);
    match result {
        Ok(mut state) => {
            step(&mut state);
            serde_json::to_string(&state)
                .unwrap_or_else(|e| format!(r#"{{"error":"serialize failed: {}"}}"#, e))
        }
        Err(e) => {
            // 型が変わってデシリアライズできない場合、デフォルト状態から再開
            eprintln!("[hot-lib] deserialization failed, resetting state: {e}");
            let mut state = State::new();
            step(&mut state);
            serde_json::to_string(&state)
                .unwrap_or_else(|e| format!(r#"{{"error":"serialize failed: {}"}}"#, e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_step_increments_counter() {
        let mut state = State::new();
        assert_eq!(state.counter, 0);
        step(&mut state);
        assert_eq!(state.counter, 1);
        step(&mut state);
        assert_eq!(state.counter, 2);
    }

    #[test]
    fn test_render_contains_message() {
        let state = State {
            counter: 42,
            message: "test".into(),
        };
        let output = render(&state);
        assert!(output.contains("test"));
    }

    #[test]
    fn test_step_serialized_roundtrip() {
        let state = State::new();
        let json = serde_json::to_string(&state).expect("serialize");
        let result = step_serialized(&json);
        let new_state: State = serde_json::from_str(&result).expect("deserialize");
        assert_eq!(new_state.counter, 1);
    }

    #[test]
    fn test_step_serialized_handles_bad_json() {
        let result = step_serialized("invalid json");
        let new_state: State = serde_json::from_str(&result).expect("deserialize");
        // デシリアライズ失敗時はデフォルト状態から再開するので counter=1
        assert_eq!(new_state.counter, 1);
    }
}
