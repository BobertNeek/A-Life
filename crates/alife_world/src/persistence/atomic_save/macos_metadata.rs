use std::{ffi::c_void, fs::File, io, os::fd::AsRawFd};

#[link(name = "System")]
extern "C" {
    fn acl_init(count: i32) -> *mut c_void;
    fn acl_set_fd(fd: i32, acl: *mut c_void) -> i32;
    fn acl_free(acl: *mut c_void) -> i32;
    fn fcopyfile(from: i32, to: i32, state: *mut c_void, flags: u32) -> i32;
}

pub(super) fn make_private(staged: &File) -> io::Result<()> {
    // An inherited macOS ACL can grant access independently of mode 0600.
    // Remove it while the newly created file is still empty.
    // SAFETY: acl_init allocates an opaque native ACL; the live descriptor is
    // borrowed only for this call, and the allocation is always freed below.
    let acl = unsafe { acl_init(0) };
    if acl.is_null() {
        return Err(io::Error::last_os_error());
    }
    let result = unsafe { acl_set_fd(staged.as_raw_fd(), acl) };
    let error = (result != 0).then(io::Error::last_os_error);
    unsafe { acl_free(acl) };
    match error {
        Some(error) if error.raw_os_error() == Some(rustix::io::Errno::NOTSUP.raw_os_error()) => {
            Ok(())
        }
        Some(error) => Err(error),
        None => Ok(()),
    }
}

pub(super) fn copy(source: &File, staged: &File) -> io::Result<()> {
    // COPYFILE_ACL | COPYFILE_XATTR. Deliberately omit COPYFILE_DATA and
    // COPYFILE_STAT: the new content and its modification time must stay new.
    const SECURITY_AND_XATTR: u32 = 1 | 4;
    // SAFETY: both descriptors remain open during the call, the null state
    // requests native defaults, and the flags request metadata only.
    if unsafe {
        fcopyfile(
            source.as_raw_fd(),
            staged.as_raw_fd(),
            std::ptr::null_mut(),
            SECURITY_AND_XATTR,
        )
    } != 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
