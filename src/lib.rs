#![no_std]
#![feature(ptr_metadata)]

// todo: set stackoverflow protection: splim
// todo: study bsp_cfg.h in more depth
// BSP_FEATURE_ICU_FIXED_IELSR_COUNT

mod generated {
    #![allow(non_camel_case_types)]
    #![allow(non_upper_case_globals)]
    #![allow(non_snake_case)]
    #![allow(dead_code)]
    #![allow(unused_imports)]

    include!(concat!(env!("OUT_DIR"), "/out.rs"));
}

pub mod unsafe_pinned {
    use core::{cell::UnsafeCell, marker::PhantomPinned};

    pub struct UnsafePinned<T: ?Sized>(PhantomPinned, UnsafeCell<T>);

    unsafe impl<T: ?Sized + Sync> Sync for UnsafePinned<T> {}
    unsafe impl<T: ?Sized + Send> Send for UnsafePinned<T> {}

    impl<T> UnsafePinned<T> {
        pub const fn new(value: T) -> Self {
            Self(PhantomPinned, UnsafeCell::new(value))
        }
    }

    impl<T: ?Sized> UnsafePinned<T> {
        pub const fn get(&self) -> *mut T {
            self.1.get()
        }
        pub const fn raw_get(this: *const Self) -> *mut T {
            unsafe { UnsafeCell::raw_get(&raw const (*this).1) }
        }
    }
}

#[cfg(feature = "mod-r_ether")]
pub mod ether;
#[cfg(feature = "mod-r_ether_phy")]
pub mod ether_phy;
#[cfg(feature = "mod-r_ioport")]
pub mod ioport;

mod macros;

pub use generated::{
    e_elc_event, //
    BSP_ICU_VECTOR_MAX_ENTRIES,
    ELC_EVENT_EDMAC0_EINT,
    ELC_EVENT_NONE,
};

#[cfg(feature = "ra6m3")]
use ::ra6m3 as pac;

pub use pac::*;

#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn SysTick_Handler() {
    unsafe extern "C" {
        fn SysTick();
    }
    unsafe { SysTick() }
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

#[cfg(feature = "log")]
#[unsafe(no_mangle)]
pub extern "C" fn __log_func(
    level: u32,
    module: *const u8,
    file: *const u8,
    _line: i32,
    fmt: *const u8,
) {
    let module = unsafe { core::ffi::CStr::from_ptr(module) };
    let file = unsafe { core::ffi::CStr::from_ptr(file) };
    let fmt = unsafe { core::ffi::CStr::from_ptr(fmt) };
    let module = module.to_str().unwrap_or("<Invalid UTF-8>");
    let file = file.to_str().unwrap_or("<Invalid UTF-8>");
    let fmt = fmt.to_str().unwrap_or("<Invalid UTF-8>");

    let lvl = match level {
        0 => log::Level::Error,
        1 => log::Level::Warn,
        2 => log::Level::Info,
        3 => log::Level::Debug,
        4 => log::Level::Trace,
        _ => log::Level::Info,
    };

    log::__private_api::log(
        log::__log_logger!(__log_global_logger),
        format_args!("{fmt}"),
        lvl,
        &(module, file, log::__private_api::loc()),
        (),
    );
}

// todo:
// void _log(unsigned level, const char *module, const char *file, int line, const char *fmt, ...)
// 		__attribute__ ((format (printf, 5, 6)));
// #define FSP_LOG_PRINT(X)    _log(1, "bsp", __FILE__, __LINE__, "%s", (X))
//
