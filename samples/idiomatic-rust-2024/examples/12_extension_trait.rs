//! ch7 / 7.3 Extension traits。
//!
//! 2024 edition で適用したベストプラクティス:
//! - 自分の crate 外の型に「メソッドを生やす」には extension trait
//! - sealed トレイトパターンで「外部からは impl できない」を明示
//! - blanket impl で `T: AsRef<str>` のように対象を広げる

mod sealed {
    pub trait Sealed {}
}

pub trait StrExt: sealed::Sealed {
    fn truncate_with_ellipsis(&self, max_chars: usize) -> String;
}

impl<T: AsRef<str>> sealed::Sealed for T {}

impl<T: AsRef<str>> StrExt for T {
    fn truncate_with_ellipsis(&self, max_chars: usize) -> String {
        let s = self.as_ref();
        if max_chars == 0 {
            return String::new();
        }
        let count = s.chars().count();
        if count <= max_chars {
            return s.to_owned();
        }
        let mut out: String = s.chars().take(max_chars - 1).collect();
        out.push('…');
        out
    }
}

fn main() {
    let title = "rust 2024 edition で書くidiomaticな話";
    println!("{}", title.truncate_with_ellipsis(10));

    let owned = String::from("hello world");
    println!("{}", owned.truncate_with_ellipsis(20));
}
