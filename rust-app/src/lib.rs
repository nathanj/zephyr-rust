#[macro_use]
extern crate cstr;
#[macro_use]
extern crate log;

extern crate zephyr_logger;

use std::time::Duration;
use std::cell::RefCell;

use log::LevelFilter;
use core::ffi::c_void;

use zephyr::mutex::*;
use zephyr::thread::ThreadSyscalls;
use zephyr::device::DeviceSyscalls;

thread_local!(static TLS: RefCell<u8> = RefCell::new(1));

zephyr_macros::k_mutex_define!(MUTEX);

fn mutex_test() {
    let data = MutexData::new(1u32);

    // Bind the static mutex to our local data. This would make more sense if
    // the data were static, but that requires app mem regions for user mode.
    let mutex = unsafe { Mutex::new(&MUTEX, &data) };

    zephyr::any::k_str_out("Locking\n");
    let _val = mutex.lock::<zephyr::context::Any>();
    zephyr::any::k_str_out("Unlocking\n");
}

#[no_mangle]
pub extern "C" fn hello_rust_second_thread(_a: *const c_void, _b: *const c_void, _c: *const c_void) {
    println!("Hello from second thread");

    TLS.with(|f| {
        println!("second thread: f = {}", *f.borrow());
        assert!(*f.borrow() == 1);
        *f.borrow_mut() = 55;
        println!("second thread: now f = {}", *f.borrow());
        assert!(*f.borrow() == 55);
    });
}

macro_rules! zassert {
    ($cond:expr, $($msg_args:tt)+) => {
        if !$cond {
            println!("assertion failed at {}:{}: {}", file!(), line!(), format_args!($($msg_args)+));
            zephyr::kernel::ztest_test_fail();
        }
    }
}

#[no_mangle]
pub extern "C" fn hello_rust_test() {
    zassert!(true, "true should be true");
    //zassert!(false, "false should not be true");
}

#[no_mangle]
pub extern "C" fn hello_rust() {
    use zephyr::context::Kernel as Context;

    println!("Hello Rust println");
    zephyr::kernel::k_str_out("Hello from Rust kernel with direct kernel call\n");
    zephyr::any::k_str_out("Hello from Rust kernel with runtime-detect syscall\n");

    std::thread::sleep(Duration::from_millis(1));
    println!("Time {:?}", zephyr::any::k_uptime_get_ms());
    println!("Time {:?}", std::time::Instant::now());

    Context::k_current_get().k_object_access_grant::<Context, _>(&MUTEX);
    mutex_test();

    if let Some(device) = Context::device_get_binding(cstr!("nonexistent")) {
        println!("Got device");
    } else {
        println!("No device");
    }

    {
        let boxed = Box::new(1u8);
        println!("Boxed value {}", boxed);
    }

    // test std::ops::{Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive}
    {
        let a: [u8; 4] = [1, 2, 3, 4];
        let len = a.iter().len();
        for _ in &a[0..len] {}
        for _ in &a[0..=(len - 1)] {}
        for _ in &a[..] {}
        for _ in &a[0..] {}
        for _ in &a[..len] {}
        for _ in &a[..=(len - 1)] {}
    }

    TLS.with(|f| {
        println!("main thread: f = {}", *f.borrow());
        assert!(*f.borrow() == 1);
        *f.borrow_mut() = 2;
        println!("main thread: now f = {}", *f.borrow());
        assert!(*f.borrow() == 2);
    });

    zephyr::kernel::k_thread_user_mode_enter(|| {
        zephyr::user::k_str_out("Hello from Rust userspace with forced user-mode syscall\n");

        mutex_test();

        zephyr_logger::init(LevelFilter::Info);

        trace!("TEST: trace!()");
        debug!("TEST: debug!()");
        info!("TEST: info!()");
        warn!("TEST: warn!()");
        error!("TEST: error!()");

        TLS.with(|f| {
            println!("main thread: f = {}", *f.borrow());
            assert!(*f.borrow() == 2);
            *f.borrow_mut() = 3;
            println!("main thread: now f = {}", *f.borrow());
            assert!(*f.borrow() == 3);
        });

        zephyr::user::k_str_out("Hello from Rust userspace with forced user-mode syscall\n");

        zephyr::any::k_str_out("Hello from Rust userspace with runtime-detect syscall\nNext call will crash if userspace is working.\n");

        // This will compile, but crash if CONFIG_USERSPACE is working
        zephyr::kernel::k_str_out("Hello from Rust userspace with direct kernel call\n");
    });
}
