use std::mem;
use std::ffi::CString;
use libc;

pub fn unlink(file: &str) {
    unsafe {
        libc::unlink(CString::new(file).unwrap().as_ptr());
    }
}

pub fn mkfifo(fifo: &str, mode: u32) {
    unsafe {
        let location = CString::new(fifo).unwrap().as_ptr();
        libc::mkfifo(location, mode);
    }
}

pub fn sigaction(signum: i32, call: fn(i32)) {
    unsafe {
        let mut action: libc::sigaction = mem::zeroed();
        action.sa_sigaction = call as usize;
        libc::sigaction(signum, &action, mem::zeroed());
    }
}
