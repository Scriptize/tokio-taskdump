//! Basic example showing how to capture a task trace at a yield point.
//!
//! Run with: `cargo run --example basic`

use std::future::Future;
use std::task::Poll;
use tokio::runtime::dump::{Trace, trace_with};
use tokio_taskdump::{TaskTrace, capture_trace};

async fn work_c() {
    tokio::task::yield_now().await;
}

async fn work_b() {
    work_c().await;
}

async fn work_a() {
    work_b().await;
}

#[tokio::main]
async fn main() {
    let mut fut = std::pin::pin!(work_a());
    let mut trace = TaskTrace::new();

    Trace::root(std::future::poll_fn(|cx| {
        trace_with(
            || {
                let _ = fut.as_mut().poll(cx);
            },
            |meta| {
                capture_trace(meta, &mut trace);
            },
        );
        Poll::Ready(())
    }))
    .await;

    println!("Captured {} frames:", trace.frames.len());
    for (i, addr) in trace.frames.iter().enumerate() {
        println!("  frame {i}: {addr:#x}");
    }

    if !trace.root_addr.is_null() {
        println!("\nRoot addr: {:?}", trace.root_addr);
    }
    if !trace.leaf_addr.is_null() {
        println!("Leaf addr: {:?}", trace.leaf_addr);
    }
}
