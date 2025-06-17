use core::ptr;

type Handle = u32;
const STD_ERROR_HANDLE: Handle = 0xFFFF_FFF4;
const PROCESS_TERMINATE_ACCESS: u32 = 1;

#[link(name = "Kernel32", kind = "dylib")]
unsafe extern "system" {
    fn GetStdHandle(handle: Handle) -> Handle;
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
pub(crate) unsafe fn get_stderr()->StdErr {
    StdErr(unsafe {
        GetStdHandle(STD_ERROR_HANDLE)
    })
}

pub(crate) fn print_error(stderr: &mut StdErr, msg: &str) {
    // SAFETY: Caller correctly acquired stderr so no problem.
    unsafe {
            WriteFile(
                stderr.0,
                msg.as_ptr(),
                msg.len().try_into().unwrap(),
                ptr::null_mut(),
                ptr::null_mut(),
            );
        }
}

pub(crate) fn terminate_current_process(_: StdErr)->! {
    // SAFETY: Caller correctly acquired stderr so no problem.
    unsafe {
                    let current_process_id = GetCurrentProcessId();
            let current_process_handle = OpenProcess(
                PROCESS_TERMINATE_ACCESS,
                false.into(),
                current_process_id,
            );
            TerminateProcess(current_process_handle, 2);
            unreachable!()
    }
}