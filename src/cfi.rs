use std::mem;
use std::ffi::CString;
use libc;

pub fn unlink(file: &str) {
    unsafe {
        libc::unlink(CString::new(file).unwrap().as_ptr());
    }
}

pub fn mkfifo(fifo: &str) {
    let mode = libc::S_IRUSR | libc::S_IWUSR | libc::S_IRGRP | libc::S_IROTH;
    unsafe {
        let location = CString::new(fifo).unwrap().as_ptr();
        libc::mkfifo(location, mode);
    }
}


pub fn pipe() -> (i32, i32) {
    let mut pipe = [0; 2];
    unsafe {
        match libc::pipe(pipe.as_mut_ptr()) {
            0 => (),
            _ => panic!("no pipe could be created"),
        };
    }

    (pipe[0], pipe[1])
}

pub fn sigaction(signum: i32, call: fn(i32)) {
    unsafe {
        let mut action: libc::sigaction = mem::zeroed();
        action.sa_sigaction = call as usize;
        libc::sigaction(signum, &action, mem::zeroed());
    }
}
