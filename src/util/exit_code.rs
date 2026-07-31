use std::{
    process::ExitStatus,
    sync::atomic::{AtomicI32, Ordering},
};

static EXTERNAL_PROCESS_EXIT_CODE: AtomicI32 = AtomicI32::new(0);

pub fn clear_external_process_exit_code() {
    EXTERNAL_PROCESS_EXIT_CODE.store(0, Ordering::SeqCst);
}

pub fn record_external_process_exit_code(status: ExitStatus) {
    EXTERNAL_PROCESS_EXIT_CODE.store(status.code().unwrap_or(1), Ordering::SeqCst);
}

pub fn take_external_process_exit_code() -> Option<i32> {
    match EXTERNAL_PROCESS_EXIT_CODE.swap(0, Ordering::SeqCst) {
        0 => None,
        code => Some(code),
    }
}
