use core::ffi::c_void;
use tokio::runtime::dump::TraceMeta;

use crate::unwind::{
    _Unwind_Backtrace, _Unwind_Context, _Unwind_GetIP,
    _Unwind_Reason_Code::{self, _URC_NO_REASON},
};

/// A captured stack trace from a Tokio task.
///
/// Contains the instruction pointer addresses for each frame in the trace,
/// along with the root and leaf addresses that bound the task's execution.
///
/// # Examples
///
/// ```
/// use tokio_taskdump::capture_trace;
/// # use core::ffi::c_void;
///
/// // Create an empty trace to be filled by capture_trace
/// let trace = tokio_taskdump::TaskTrace::new();
/// assert!(trace.frames.is_empty());
/// ```
#[derive(Clone, Debug)]
pub struct TaskTrace {
    /// The instruction pointer addresses captured from the stack.
    pub frames: Vec<usize>,
    /// The root address of the task (entry point).
    pub root_addr: *mut c_void,
    /// The leaf address of the task (yield point).
    pub leaf_addr: *mut c_void,
}

impl TaskTrace {
    /// Creates a new empty `TaskTrace` with no frames and null addresses.
    ///
    /// # Examples
    ///
    /// ```
    /// let trace = tokio_taskdump::TaskTrace::new();
    /// assert!(trace.frames.is_empty());
    /// ```
    pub fn new() -> TaskTrace {
        Self {
            frames: vec![],
            root_addr: std::ptr::null_mut::<c_void>(),
            leaf_addr: std::ptr::null_mut::<c_void>(),
        }
    }
}

impl Default for TaskTrace {
     fn default() -> Self {
        Self::new()
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

/// Captures a stack trace from a Tokio task into the provided [`TaskTrace`].
///
/// This function is meant to be called from within a [`trace_with`] callback.
/// It clears any existing frames in `trace`, sets the root and leaf addresses
/// from `meta`, and performs a stack unwind to collect instruction pointers.
///
/// # Examples
///
/// ```
/// use std::future::Future;
/// use std::task::Poll;
/// use tokio::runtime::dump::{Trace, trace_with};
/// use tokio_taskdump::{capture_trace, TaskTrace};
///
/// # #[tokio::main]
/// # async fn main() {
/// let mut fut = std::pin::pin!(async {
///     tokio::task::yield_now().await;
/// });
///
/// let mut trace = TaskTrace::new();
///
/// Trace::root(std::future::poll_fn(|cx| {
///     trace_with(
///         || { let _ = fut.as_mut().poll(cx); },
///         |meta| { capture_trace(meta, &mut trace); },
///     );
///     Poll::Ready(())
/// }))
/// .await;
/// # }
/// ```
///
/// [`trace_with`]: tokio::runtime::dump::trace_with
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
