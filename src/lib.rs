#![no_std]

// todo: set stackoverflow protection
// todo: study bsp_cfg.h in more depth

mod generated {
    #![allow(non_camel_case_types)]
    #![allow(non_upper_case_globals)]
    #![allow(non_snake_case)]

    include!(concat!(env!("OUT_DIR"), "/ra-fsp.rs"));
}

#[unsafe(no_mangle)]
pub extern "C" fn __assert_func(file: *const u8, line: i32, func: *const u8, expr: *const u8) {
    let file = unsafe { core::ffi::CStr::from_ptr(file) };
    let func = unsafe { core::ffi::CStr::from_ptr(func) };
    let expr = unsafe { core::ffi::CStr::from_ptr(expr) };

    let file = file.to_str().unwrap_or("<Invalid UTF-8>");
    let func = func.to_str().unwrap_or("<Invalid UTF-8>");
    let expr = expr.to_str().unwrap_or("<Invalid UTF-8>");

    panic!("Assertion failed in file: {file}, line: {line}, function: {func}, expression: {expr}",);
}

// todo:
// void _log(unsigned level, const char *module, const char *file, int line, const char *fmt, ...)
// 		__attribute__ ((format (printf, 5, 6)));
// #define FSP_LOG_PRINT(X)    _log(1, "bsp", __FILE__, __LINE__, "%s", (X))
//
