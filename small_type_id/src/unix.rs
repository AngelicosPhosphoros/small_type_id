#[repr(C)]
struct File(core::ffi::c_void);
const STDERR_FILENO: i32 = 2;

unsafe extern "C" {
    fn fdopen(fd: i32, mode: *const u8) -> *mut File;
    fn fwrite(buffer: *const u8, elem_size: usize, len: usize, file: *mut File) -> usize;
    fn fflush(file: *mut File) -> i32;
    safe fn abort() -> !;
}

#[repr(transparent)]
pub(crate) struct StdErr(*mut File);

/// # Safety
/// Must be called only once.
/// While returned value in use, no other thread should access stderr.
pub(crate) unsafe fn get_stderr() -> StdErr {
    unsafe {
        let stderr: *mut File = fdopen(STDERR_FILENO, c"a".as_ptr().cast());
        StdErr(stderr)
    }
}

pub(crate) fn print_error(stderr: &mut StdErr, msg: &str) {
    // SAFETY: Caller correctly acquired stderr so no problem.
    unsafe {
        fwrite(msg.as_ptr(), 1, msg.len(), stderr.0);
    }
}

pub(crate) fn terminate_current_process(stderr: StdErr) -> ! {
    // SAFETY: Caller correctly acquired stderr so no problem.
    unsafe {
        // Need to flush on unix because it doesn't flush after process terminates.
        fflush(stderr.0);
        // Leaking file object is not a problem because process terminates anyway.
        abort()
    }
}
