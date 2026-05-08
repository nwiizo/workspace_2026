//! ch5 / 5.3 Builder + ch7 / 7.6 Struct tagging + ch8 / 8.1 Trait state machine。
//!
//! 2024 edition で適用したベストプラクティス:
//! - `PhantomData` で「ビルド段階」を型に乗せる (type-state)
//! - `build()` は完成済み状態でしか呼べない。コンパイル時に未設定フィールドを禁止
//! - `#[must_use]` を付けて、builder を捨てると警告

use std::marker::PhantomData;

#[derive(Debug)]
pub struct Missing;

#[derive(Debug)]
pub struct Set;

#[derive(Debug)]
pub struct Request {
    pub url: String,
    pub body: String,
}

#[derive(Debug, Default)]
#[must_use = "Builder は最後に build() を呼んで Request を取り出す"]
pub struct RequestBuilder<UrlState, BodyState> {
    url: Option<String>,
    body: Option<String>,
    _state: PhantomData<(UrlState, BodyState)>,
}

impl RequestBuilder<Missing, Missing> {
    pub const fn new() -> Self {
        Self {
            url: None,
            body: None,
            _state: PhantomData,
        }
    }
}

impl<B> RequestBuilder<Missing, B> {
    pub fn url(self, url: impl Into<String>) -> RequestBuilder<Set, B> {
        RequestBuilder {
            url: Some(url.into()),
            body: self.body,
            _state: PhantomData,
        }
    }
}

impl<U> RequestBuilder<U, Missing> {
    pub fn body(self, body: impl Into<String>) -> RequestBuilder<U, Set> {
        RequestBuilder {
            url: self.url,
            body: Some(body.into()),
            _state: PhantomData,
        }
    }
}

impl RequestBuilder<Set, Set> {
    pub fn build(self) -> Request {
        // url / body が Some なのは型 (Set, Set) で保証されている。
        // 型状態のおかげで unreachable な分岐に落ちない。
        #[expect(
            clippy::expect_used,
            reason = "RequestBuilder<Set, Set> は型レベルで Some を保証する"
        )]
        let url = self.url.expect("type-state guarantees url is Some");
        #[expect(
            clippy::expect_used,
            reason = "RequestBuilder<Set, Set> は型レベルで Some を保証する"
        )]
        let body = self.body.expect("type-state guarantees body is Some");
        Request { url, body }
    }
}

fn main() {
    let req = RequestBuilder::new()
        .url("https://example.com")
        .body("payload")
        .build();
    println!("{req:?}");

    // url を設定する前に build() を呼ぶとコンパイルエラーになる。
    // let _ = RequestBuilder::new().build(); // ← compile error
}
