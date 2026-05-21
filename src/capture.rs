use core::ffi::c_void;
use tokio::runtime::dump::TraceMeta;

use crate::unwind::{
    _Unwind_Backtrace, _Unwind_Context, _Unwind_GetIP,
    _Unwind_Reason_Code::{self, _URC_NO_REASON},
};

#[derive(Clone)]
pub struct TaskTrace {
    pub frames: Vec<usize>,
    pub root_addr: *mut c_void,
    pub leaf_addr: *mut c_void,
}

impl TaskTrace {
    pub fn new() -> TaskTrace {
        Self {
            frames: vec![],
            root_addr: std::ptr::null_mut::<c_void>(),
            leaf_addr: std::ptr::null_mut::<c_void>(),
        }
    }
}

extern "C" fn callback(ctx: *mut _Unwind_Context, arg: *mut c_void) -> _Unwind_Reason_Code {
    unsafe {
        let data = &mut *(arg as *mut Vec<usize>);
        let ip = _Unwind_GetIP(ctx);

        // do this later
        // let symbol_addr = _Unwind_FindEnclosingFunction(ip as *mut c_void);
        // let below_root = !ptr::eq(symbol_addr, data.root_addr);

        data.push(ip);
        _URC_NO_REASON
    }
}

pub fn capture_trace(meta: &TraceMeta, trace: &mut TaskTrace) {
    trace.frames.clear();

    trace.leaf_addr = meta.trace_leaf_addr as *mut c_void;
    trace.root_addr = meta.root_addr.unwrap_or(std::ptr::null()) as *mut c_void;

    unsafe {
        _Unwind_Backtrace(
            callback,
            &mut trace.frames as *mut Vec<usize> as *mut c_void,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    use std::task::Poll;
    use tokio::runtime::dump::{Trace, trace_with};

    // Create a chain of async functions to produce multiple frames
    async fn deep_c() {
        tokio::task::yield_now().await;
    }

    async fn deep_b() {
        deep_c().await;
    }

    async fn deep_a() {
        deep_b().await;
    }

    #[tokio::test]
    async fn test_capture_multiple_frames() {
        let mut fut = std::pin::pin!(async {
            deep_a().await;
        });

        let mut collected = TaskTrace::new();

        Trace::root(std::future::poll_fn(|cx| {
            trace_with(
                || {
                    let _ = fut.as_mut().poll(cx);
                },
                |meta| {
                    capture_trace(meta, &mut collected);
                },
            );
            Poll::Ready(())
        }))
        .await;

        // Should have multiple frames: deep_a, deep_b, deep_c, etc.
        assert!(
            collected.frames.len() > 1,
            "expected multiple frames, got {}",
            collected.frames.len()
        );
    }

    #[tokio::test]
    async fn test_capture_single_yield() {
        let mut fut = std::pin::pin!(async {
            tokio::task::yield_now().await;
        });

        let mut collected = TaskTrace::new();

        Trace::root(std::future::poll_fn(|cx| {
            trace_with(
                || {
                    let _ = fut.as_mut().poll(cx);
                },
                |meta| {
                    capture_trace(meta, &mut collected);
                },
            );
            Poll::Ready(())
        }))
        .await;

        assert!(
            !collected.frames.is_empty(),
            "should capture at least one frame"
        );
    }

    #[tokio::test]
    async fn test_no_yield_no_frames() {
        let mut fut = std::pin::pin!(async { 42 });

        let mut collected = TaskTrace::new();

        Trace::root(std::future::poll_fn(|cx| {
            trace_with(
                || {
                    let _ = fut.as_mut().poll(cx);
                },
                |meta| {
                    capture_trace(meta, &mut collected);
                },
            );
            Poll::Ready(())
        }))
        .await;

        // No yield = trace_leaf never fires = no frames
        assert!(
            collected.frames.is_empty(),
            "no yield should mean no frames"
        );
    }
}
