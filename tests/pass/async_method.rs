//! `async fn` methods are forwarded with sequential awaits.

use std::future::Future;
use std::task::{Context, Poll, Waker};

use auto_tuple::auto_tuple;

/// Minimal executor: the forwarded futures are immediately-ready, so a
/// no-op waker plus a spin loop suffices.
fn block_on<F: Future>(fut: F) -> F::Output {
    let mut fut = Box::pin(fut);
    let waker = Waker::noop();
    let mut cx = Context::from_waker(&waker);
    loop {
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(v) => return v,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

#[auto_tuple]
trait Tr {
    async fn fetch(&self) -> usize;
    async fn add(&self, x: usize) -> usize;
    fn sync(&self) -> usize;
}

struct X(usize);
struct Y(usize);

impl Tr for X {
    async fn fetch(&self) -> usize {
        self.0
    }
    async fn add(&self, x: usize) -> usize {
        self.0 + x
    }
    fn sync(&self) -> usize {
        self.0
    }
}

impl Tr for Y {
    async fn fetch(&self) -> usize {
        self.0
    }
    async fn add(&self, x: usize) -> usize {
        self.0 + x
    }
    fn sync(&self) -> usize {
        self.0
    }
}

fn main() {

    let t = (X(1), Y(2));

    assert_eq!(block_on(t.fetch()), (1, 2));
    assert_eq!(block_on(t.add(5)), (6, 7));
    assert_eq!(t.sync(), (1, 2));
}
