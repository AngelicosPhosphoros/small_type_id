use core::ptr;

#[repr(transparent)]
#[derive(Clone, Copy)]
struct Handle(u32);

const STD_ERROR_HANDLE: u32 = 0xFFFF_FFF4;
const PROCESS_TERMINATE_ACCESS: u32 = 1;

#[link(name = "Kernel32", kind = "raw-dylib")]
unsafe extern "system" {
    fn GetStdHandle(handle: u32) -> Handle;
    fn WriteFile(
        file_handle: Handle,
        buffer: *const u8,
        len: u32,
        bytes_written: *mut u32,
        overlapping: *mut (),
    ) -> i32;
    safe fn GetCurrentProcessId() -> u32;
    fn OpenProcess(desired_acces: u32, inherit_handle: i32, process_id: u32) -> Handle;
    fn TerminateProcess(handle: Handle, exit_code: u32) -> i32;
}

#[repr(transparent)]
pub(crate) struct StdErr(Handle);

/// # Safety
/// Must be called only once.
/// While returned value in use, no other thread should access stderr.
pub(crate) unsafe fn get_stderr() -> StdErr {
    StdErr(unsafe { GetStdHandle(STD_ERROR_HANDLE) })
}

pub(crate) fn print_error(stderr: &mut StdErr, msg: &str) {
    // SAFETY: Caller correctly acquired stderr so no problem.
    let mut rest = msg.as_bytes();
    while !rest.is_empty() {
        const MIBS_16: usize = usize::pow(2, 24);
        let bytes_to_write: u32 = rest.len().min(MIBS_16).try_into().unwrap();
        // SAFETY: We follow WinAPI requirements.
        let bytes_written = unsafe {
            let mut written = 0;
            let res = WriteFile(
                stderr.0,
                rest.as_ptr(),
                bytes_to_write,
                &raw mut written,
                ptr::null_mut(),
            );
            if res == 0 {
                // Well, we have an error.
                // Since our error reporting is done for diagnostics
                // and on best effort basis during terminating process,
                // we just ignore errors.
                return;
            }
            written
        };
        let bytes_written: usize = bytes_written.try_into().unwrap();
        rest = &rest[bytes_written..];
    }
}

pub(crate) fn terminate_current_process(_: StdErr) -> ! {
    // SAFETY: Caller correctly acquired stderr so no problem.
    // We use TerminateProcess instead of ExitProcess
    // so no other code can corrupt any memory because it wouldn't be run.
    unsafe {
        let current_process_id = GetCurrentProcessId();
        let current_process_handle =
            OpenProcess(PROCESS_TERMINATE_ACCESS, false.into(), current_process_id);
        TerminateProcess(current_process_handle, 2);
        unreachable!()
    }
}
