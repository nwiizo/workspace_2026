# chrono

**chrono** は Rust の日時ライブラリです。yasume ではセッション完了時刻の記録、今日/今週のフィルタリング、深夜判定などに使っています。

## なぜ std::time ではなく chrono を使うのか

Rust の標準ライブラリにも `std::time` がありますが、できることが限られています:

| やりたいこと | `std::time` | `chrono` |
|-------------|-------------|----------|
| 経過時間の計測 | `Instant::now()` | 可能 |
| 現在時刻の取得 | `SystemTime::now()`（扱いにくい） | `Local::now()`（タイムゾーン付き） |
| 「今日の日付」を取得 | 不可 | `Local::now().date_naive()` |
| 日付のフォーマット（"2026-02-14"） | 不可 | `.format("%Y-%m-%d")` |
| 曜日の計算 | 不可 | `.weekday()` |
| 日付の加減算（3日前） | 不可 | `date - Duration::days(3)` |
| JSON シリアライズ | 手動実装が必要 | `features = ["serde"]` で自動 |

yasume では「何時に完了したか」「今日のセッション一覧」「今週のレポート」といった**カレンダー的な操作**が頻繁に必要なため、chrono を使っています。

## このプロジェクトでの役割

### ローカル時刻の取得

```rust
use chrono::{Local, Timelike};  // Timelike: .hour() 等のメソッドに必要な trait

let now = Local::now();           // 現在のローカル時刻（DateTime<Local>）
let hour = now.hour();            // 時（0-23）。深夜判定に使用
let date = now.date_naive();      // 日付のみ（NaiveDate）。タイムゾーン情報なし
```

### 日付の比較・フィルタ

```rust
// 今日のタスクだけを抽出する
self.tasks.iter().filter(|t| t.completed_at.date_naive() == today)

// 日付範囲でフィルタ（list コマンド用）
date >= from && date <= to
```

### 曜日の計算（週間レポート用）

`src/history.rs` の `this_week()` メソッドの実際のコード:

```rust
pub fn this_week(&self) -> Vec<&CompletedTask> {
    let today = Local::now().date_naive();
    // 今日の曜日から「月曜からの日数」を計算（月=0, 火=1, ... 日=6）
    let days_from_monday = today.weekday().num_days_from_monday();
    // 今週の月曜日を逆算
    let week_start = today - chrono::Duration::days(i64::from(days_from_monday));
    // 月曜以降のタスクをフィルタ
    self.tasks
        .iter()
        .filter(|t| t.completed_at.date_naive() >= week_start)
        .collect()
}
```

### フォーマットとパース

```rust
// DateTime → 文字列
completed_at.format("%Y-%m-%d")  // "2026-02-14"
completed_at.format("%H:%M")     // "09:15"

// 文字列 → NaiveDate（CLI の --from/--to 引数をパースする）
NaiveDate::parse_from_str("2026-02-14", "%Y-%m-%d")  // Result<NaiveDate, _>
```

## 注意点

- **`Timelike` / `Datelike` trait**: `.hour()` や `.weekday()` を使うにはこれらの trait を `use` する必要がある。忘れるとコンパイルエラーになるので、エラーメッセージの「method not found」が出たらまず `use` を確認
- **`NaiveDate` vs `DateTime<Local>`**: 日付だけの比較には `.date_naive()` で `NaiveDate` に変換してから行う。`DateTime` 同士だと時刻の違いで意図しない結果になる
- **`chrono::Duration` vs `std::time::Duration`**: `chrono::Duration` は負の値を持てるので日付の減算に使える。`std::time::Duration` は正の値のみ
- **`DateTime<Local>` の serde**: `Cargo.toml` で chrono の `features = ["serde"]` を有効にする。ISO 8601 + タイムゾーンオフセット形式でシリアライズされる
