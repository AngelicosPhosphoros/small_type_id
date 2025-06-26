use core::ffi::{c_int, c_void};

const STDERR_FILENO: c_int = 2;

unsafe extern "C" {
    fn write(fd: c_int, buffer: *const c_void, len: usize) -> isize;
    // SAFETY: We are intentionally crashing so no problem.
    // If somehow other code had hooked SIGABRT before main,
    // we cannot really do anything about it.
    safe fn abort() -> !;
}

#[repr(transparent)]
pub(crate) struct StdErr(());

/// # Safety
/// Must be called only once.
/// While returned value in use, no other thread should access stderr.
pub(crate) unsafe fn get_stderr() -> StdErr {
    // We don't bother with trying to handle unfinished previous
    // writes made by fprintf, fputs or fwrite because there is
    // no portable way to get their existing file objects and
    // there is no guarantee that fdopen would handle existing
    // buffered writes correctly.
    // Therefore, if previous writers wanted to have their output
    // printed, they should have flushed it themselves.
    // We would just use unbuffered `write` calls.
    StdErr(())
}

pub(crate) fn print_error(_stderr: &mut StdErr, msg: &str) {
    let mut rest = msg.as_bytes();
    while !rest.is_empty() {
        // SAFETY: We are trying to output diagnostic info on best effort basis.
        unsafe {
            // While unbuffered write is slow, this code shouldn't run
            // almost never because it is executed only when TypeIds collide.
            let res = write(STDERR_FILENO, rest.as_ptr().cast(), rest.len());
            if res < 0 {
                // Well, POSIX says that we should handle this and check `errno`.
                // But `errno` is a bad API that is not standardized for linking.
                // It can be:
                //   1. a global variable
                //   2. result of `__errno_location()` (used by musl and glibc)
                //   3. result of `__error()` (macos and some bsds)
                //   4. or literally anything else C macro can expand into.
                // Instead of dealing with this incompatible and undocumented mess,
                // we would just ignore errors. This output are meant to be used
                // by devs during development anyway so they can retry running program.
                return;
            }
            let written: usize = rest.len().min(res.try_into().unwrap());
            rest = &rest[written..];
        }
    }
}

pub(crate) fn terminate_current_process(_stderr: StdErr) -> ! {
    abort()
}
