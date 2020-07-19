//! It's multitask's work-stealing executor, but just running on its own single thread
//!
//! Taken from the smol lib, which hasn't it's new architecture yet.
//!
//! Not sure how I can just use the LocalExecutor? Maybe can't if I want to keep the executor on a
//! background thread.

use once_cell::sync::Lazy;
use multitask::Executor;
use std::panic::catch_unwind;
use std::future::Future;
use std::task::{Context, Poll};
use std::pin::Pin;

pub struct Task<T>(multitask::Task<T>);

impl<T> Task<T> {
    pub(crate) fn spawn<F>(future: F) -> Task<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        static EXECUTOR: Lazy<Executor> = Lazy::new(|| {
            // TODO can I use the LocalExecutor? maybe not, it looks like it's meant to be used
            // "inline" in a thread.
            std::thread::spawn(|| {
                let (p, u) = async_io::parking::pair();
                let ticker = EXECUTOR.ticker(move || u.unpark());

                loop {
                    if let Ok(false) = catch_unwind(|| ticker.tick()) {
                        p.park();
                    }
                }
            });

            Executor::new()
        });

        Task(EXECUTOR.spawn(future))
    }

}

impl<T> Future for Task<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.0).poll(cx)
    }
}
