//! Async fn in trait (Rust 1.75+, 2024 edition で本格運用) の例。
//!
//! 2024 edition で適用したベストプラクティス:
//! - 静的ディスパッチで足りるならトレイトに直接 `async fn` を書く
//!   (書籍8章は `Box<Pin<dyn Future>>` を手書きしている)
//! - 失敗パスを enum エラーで型付け (panic / unwrap を使わない)
//! - `unused_async = warn` を回避するため、各 `handle` は実際に await を含むか
//!   非async でよいかを意識する
//! - dyn 化したい場合は `#[trait_variant::make]` か手動 `Pin<Box<dyn Future>>` が必要、
//!   という制約は明記しておく (RPITIT は dyn 不可)

#[derive(Debug, thiserror::Error)]
enum ActorError {
    #[error("empty message")]
    Empty,
}

trait Actor {
    async fn handle(&mut self, msg: &str) -> Result<String, ActorError>;
}

struct Echo;

impl Actor for Echo {
    async fn handle(&mut self, msg: &str) -> Result<String, ActorError> {
        if msg.is_empty() {
            return Err(ActorError::Empty);
        }
        // 実際の async I/O を模す: yield_now で他タスクに譲る
        tokio::task::yield_now().await;
        Ok(format!("echo: {msg}"))
    }
}

struct Counter {
    n: u32,
}

impl Actor for Counter {
    async fn handle(&mut self, _msg: &str) -> Result<String, ActorError> {
        self.n += 1;
        Ok(format!("count={}", self.n))
    }
}

async fn drive<A: Actor>(actor: &mut A, msgs: &[&str]) -> Result<(), ActorError> {
    for m in msgs {
        let out = actor.handle(m).await?;
        println!("{out}");
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), ActorError> {
    drive(&mut Echo, &["hello", "world"]).await?;
    drive(&mut Counter { n: 0 }, &["a", "b", "c"]).await?;
    Ok(())
}
