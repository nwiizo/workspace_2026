//! ch4 / 4.1 RAII。
//!
//! 2024 edition で適用したベストプラクティス:
//! - `Drop` で確定的に解放するパターン (RAII) は edition 不変の Rust の核
//! - tail expression drop order の変更 (2024) で、関数末尾式の一時 guard が
//!   ローカル変数より「先」に drop される。これが意味のある違いを生むケースを示す
//! - guard を返す API では `#[must_use]` を付けて、捨てられないようにする

#[derive(Debug)]
struct Span {
    name: &'static str,
}

impl Span {
    #[must_use = "Span を捨てると区間計測の意味がなくなる"]
    fn enter(name: &'static str) -> Self {
        println!("> enter {name}");
        Self { name }
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        println!("< exit  {}", self.name);
    }
}

fn work() {
    // 2024 edition: 末尾式の一時値が先に drop される。
    // ここで `Span::enter("inner")` を let バインドせず単独で書くと、
    // その文が終わった瞬間に drop されてしまう。RAII guard は必ず let で受ける。
    let _outer = Span::enter("outer");
    {
        let _inner = Span::enter("inner");
        println!("  doing work");
    }
    println!("  more outer-only work");
}

fn main() {
    work();
}
