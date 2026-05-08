//! 10章: `std` だけで作る最小 `block_on` の 2024 edition 実装。
//!
//! ここで適用したベストプラクティス:
//! - `std::pin::pin!` マクロで future をスタックに pin。`Box::pin` 不要。
//! - `thread::park` / `unpark` を使うと `Condvar` 自前実装より短く、教科書的に分かりやすい。
//! - `unsafe` ブロックは `RawWakerVTable` 構築箇所のみ。`unsafe_op_in_unsafe_fn = deny`
//!   を満たすため、`unsafe fn` の中でも `unsafe` ブロックを明示する。
//! - `Waker` は `Arc<Thread>` を data ポインタとして持ち、wake で `unpark` する古典構造。
//!
//! ## 補足: 単純なテストなら `Waker::noop` でよい
//!
//! Rust 1.83+ では `std::task::Waker::noop` が安定化されており、
//! 「wake しても何もしない Waker」が必要なテストや単発 poll では下記の `RawWaker` 自前実装は
//! 不要です。本サンプルは `Future` が `Pending` のあいだ `thread::park` で実際に
//! ブロックする本格 `block_on` を構築するので、wake で `unpark` する Waker が必要になります。

use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use std::thread::{self, Thread};

fn waker_for_thread(handle: Arc<Thread>) -> Waker {
    // Arc<Thread> を *const () として運ぶ。
    let raw = Arc::into_raw(handle).cast::<()>();
    // SAFETY: from_raw に渡すポインタは Arc::into_raw 由来。
    unsafe { Waker::from_raw(RawWaker::new(raw, &VTABLE)) }
}

const VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop_waker);

unsafe fn clone(data: *const ()) -> RawWaker {
    // SAFETY: data は Arc<Thread> から into_raw されたもの。
    let arc = unsafe { Arc::from_raw(data.cast::<Thread>()) };
    let cloned = Arc::clone(&arc);
    let _ = Arc::into_raw(arc); // 元 Arc は維持
    RawWaker::new(Arc::into_raw(cloned).cast::<()>(), &VTABLE)
}

unsafe fn wake(data: *const ()) {
    // SAFETY: data は Arc<Thread> 由来。所有権を取り戻して drop で参照を消費する。
    let arc = unsafe { Arc::from_raw(data.cast::<Thread>()) };
    arc.unpark();
}

unsafe fn wake_by_ref(data: *const ()) {
    // SAFETY: data の所有は呼び出し側。借用するだけで into_raw で戻す。
    let arc = unsafe { Arc::from_raw(data.cast::<Thread>()) };
    arc.unpark();
    let _ = Arc::into_raw(arc);
}

unsafe fn drop_waker(data: *const ()) {
    // SAFETY: from_raw で所有権を取り戻し、スコープ終了で drop。
    let _ = unsafe { Arc::from_raw(data.cast::<Thread>()) };
}

fn block_on<F: Future>(future: F) -> F::Output {
    let handle = Arc::new(thread::current());
    let waker = waker_for_thread(handle);
    let mut cx = Context::from_waker(&waker);

    let mut future = pin!(future);
    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(v) => return v,
            Poll::Pending => thread::park(),
        }
    }
}

fn main() {
    // 標準ライブラリだけで future を駆動する。tokio もスレッドプールも不要。
    let answer = block_on(async {
        // ネストした async ブロックも問題なく動く。
        let inner = async { 40u32 };
        inner.await + 2
    });
    println!("answer = {answer}");
}
