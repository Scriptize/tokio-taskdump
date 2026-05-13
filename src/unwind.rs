// src/unwind.rs — vendored from backtrace-rs

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use core::ffi::c_void;

#[repr(C)]
pub enum _Unwind_Reason_Code {
    _URC_NO_REASON = 0,
    _URC_END_OF_STACK = 5,
    _URC_FAILURE = 9,
}

pub enum _Unwind_Context {}

pub type _Unwind_Trace_Fn =
    extern "C" fn(ctx: *mut _Unwind_Context, arg: *mut c_void) -> _Unwind_Reason_Code;

unsafe extern "C" {
    pub fn _Unwind_Backtrace(
        trace: _Unwind_Trace_Fn,
        trace_argument: *mut c_void,
    ) -> _Unwind_Reason_Code;
    pub fn _Unwind_GetIP(ctx: *mut _Unwind_Context) -> usize;
    pub fn _Unwind_FindEnclosingFunction(pc: *mut c_void) -> *mut c_void;
}
