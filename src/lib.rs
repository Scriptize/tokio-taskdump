//! tokio-taskdump
// src/lib.rs

mod unwind;
mod capture;
pub use capture::capture_trace;

// /// A collected task dump — raw instruction pointers, symbolize later
// pub struct TaskTrace {
//     pub frames: Vec<usize>,
// }


// /// A dump of multiple tasks
// pub struct TaskDump {
//     pub tasks: Vec<TaskTrace>,
// }



// pub fn trace_leaf(meta: &TraceMeta) -> TaskTrace {
//     let mut frames: Vec<usize> = vec![];
//     let mut above_leaf = false;

//     let Some(root_addr) = meta.root_addr else {
//         return TaskTrace { frames: frames }
//     };

//     //pack everything into struct and recast
//     struct CallbackData {
//         frames: Vec<usize>,
//         above_leaf: bool,
//         root_addr: *const c_void,
//         leaf_addr: *const c_void,
//     }

//     let mut data = CallbackData {
//         frames: vec![],
//         above_leaf: false,
//         root_addr: root_addr,
//         leaf_addr: meta.trace_leaf_addr,
//     };

//     // The callback must be `extern "C"` to match _Unwind_Trace_Fn signature
//     extern "C" fn callback(
//         ctx: *mut _Unwind_Context,
//         arg: *mut c_void,
//     ) -> _Unwind_Reason_Code {
//         unsafe {
//             let data = &mut *(arg as *mut CallbackData);

//             let ip = _Unwind_GetIP(ctx);
//             let symbol_addr = _Unwind_FindEnclosingFunction(ip as *mut c_void);

//             let below_root = !ptr::eq(symbol_addr, data.root_addr);

//             if data.above_leaf && below_root {
//                 data.frames.push(ip);
//             }

//             if ptr::eq(symbol_addr, data.leaf_addr) {
//                 data.above_leaf = true;
//             }

//             if below_root {
//                 _URC_NO_REASON  // keep walking
//             } else {
//                 _URC_FAILURE    // stop, we hit root
//             }
//         }
//     }

//     // Actually call _Unwind_Backtrace
//     unsafe {
//         _Unwind_Backtrace(
//             callback,
//             &mut data as *mut CallbackData as *mut c_void,
//         );
//     }

//     TaskTrace { frames: data.frames }
// }






