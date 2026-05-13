use core::ffi::c_void;
use std::ptr;
use std::future::Future;
use std::task::Poll;
use tokio::runtime::dump::{trace_with, TraceMeta};

use crate::unwind::{
    _Unwind_Backtrace,
    _Unwind_Context,
    _Unwind_FindEnclosingFunction,
    _Unwind_GetIP,
    _Unwind_Reason_Code::{self, _URC_FAILURE, _URC_NO_REASON},
};

#[derive(Clone)]
pub struct TaskTrace {
    pub frames: Vec<usize>,
}
struct CallbackData {
    trace: TaskTrace,
    above_leaf: bool,
    root_addr: *const c_void,
    leaf_addr: *const c_void,
}



extern "C" fn callback(
    ctx: *mut _Unwind_Context,
    arg: *mut c_void,
) -> _Unwind_Reason_Code {
    unsafe {
        let data = &mut *(arg as *mut CallbackData);
        let ip = _Unwind_GetIP(ctx);
        let symbol_addr = _Unwind_FindEnclosingFunction(ip as *mut c_void);
        let below_root = !ptr::eq(symbol_addr, data.root_addr);

        if data.above_leaf && below_root {
            data.trace.frames.push(ip);
        }

        if ptr::eq(symbol_addr, data.leaf_addr) {
            data.above_leaf = true;
        }

        if below_root {
            _URC_NO_REASON
        } else {
            _URC_FAILURE
        }
    }
}


pub fn capture_trace(meta: &TraceMeta) -> TaskTrace {
    let Some(root_addr) = meta.root_addr else {
        return TaskTrace { frames: vec![] };
    };

    let mut data = CallbackData {
        trace: TaskTrace { frames: vec![] },
        above_leaf: false,
        root_addr,
        leaf_addr: meta.trace_leaf_addr,
    };

    unsafe {
        _Unwind_Backtrace(
            callback,
            &mut data as *mut CallbackData as *mut c_void,
        );
    }

    data.trace  // return the trace with collected frames
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::dump::{trace_with, Trace, TraceMeta};
    use std::future::Future;
    use std::task::Poll;

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

        let mut collected = TaskTrace { frames: vec![] };

        Trace::root(std::future::poll_fn(|cx| {
            trace_with(
                || { let _ = fut.as_mut().poll(cx); },
                |meta| {
                    collected = capture_trace(meta);
                },
            );
            Poll::Ready(())
        })).await;

        // Should have multiple frames: deep_a, deep_b, deep_c, etc.
        assert!(collected.frames.len() > 1,
            "expected multiple frames, got {}", collected.frames.len());
    }

    #[tokio::test]
    async fn test_capture_single_yield() {
        let mut fut = std::pin::pin!(async {
            tokio::task::yield_now().await;
        });

        let mut collected = TaskTrace { frames: vec![] };

        Trace::root(std::future::poll_fn(|cx| {
            trace_with(
                || { let _ = fut.as_mut().poll(cx); },
                |meta| {
                    collected = capture_trace(meta);
                },
            );
            Poll::Ready(())
        })).await;

        assert!(!collected.frames.is_empty(),
            "should capture at least one frame");
    }

    #[tokio::test]
    async fn test_no_yield_no_frames() {
        let mut fut = std::pin::pin!(async { 42 });

        let mut collected = TaskTrace { frames: vec![] };

        Trace::root(std::future::poll_fn(|cx| {
            trace_with(
                || { let _ = fut.as_mut().poll(cx); },
                |meta| {
                    collected = capture_trace(meta);
                },
            );
            Poll::Ready(())
        })).await;

        // No yield = trace_leaf never fires = no frames
        assert!(collected.frames.is_empty(),
            "no yield should mean no frames");
    }
}
