//! Handle-based Windows access-control preservation. Callers serialize changes
//! to the destination and its parent; observations cannot make rename conditional.
use std::{
    ffi::c_void,
    fs::{File, OpenOptions},
    io,
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        fs::{MetadataExt, OpenOptionsExt},
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::Path,
    ptr::null_mut,
};
use windows_sys::Win32::{
    Foundation::{LocalFree, GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE},
    Security::{
        Authorization::*, EqualSid, GetAce, GetSecurityDescriptorControl, GetTokenInformation,
        SetFileSecurityW, TokenUser, ACE_HEADER, ACL, ATTRIBUTE_SECURITY_INFORMATION,
        DACL_SECURITY_INFORMATION, GROUP_SECURITY_INFORMATION, LABEL_SECURITY_INFORMATION,
        OWNER_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
        PSID, SCOPE_SECURITY_INFORMATION, SECURITY_ATTRIBUTES, SE_DACL_PROTECTED, TOKEN_QUERY,
        TOKEN_USER, UNPROTECTED_DACL_SECURITY_INFORMATION,
    },
    Storage::FileSystem::{
        CreateFileW, CREATE_NEW, FILE_ATTRIBUTE_ENCRYPTED, FILE_ATTRIBUTE_NORMAL, READ_CONTROL,
        WRITE_DAC, WRITE_OWNER,
    },
    System::Threading::{GetCurrentProcess, OpenProcessToken},
};
const PARTS: u32 = OWNER_SECURITY_INFORMATION
    | GROUP_SECURITY_INFORMATION
    | DACL_SECURITY_INFORMATION
    | LABEL_SECURITY_INFORMATION;

struct Allocation(*mut c_void);
impl Drop for Allocation {
    fn drop(&mut self) {
        // SAFETY: these allocations come from APIs documented to use LocalAlloc.
        unsafe {
            LocalFree(self.0);
        }
    }
}
pub(super) struct Snapshot {
    _allocation: Allocation,
    owner: PSID,
    group: PSID,
    label: *mut ACL,
    dacl: *mut ACL,
    automatic: bool,
    protected: bool,
    text: Vec<u16>,
    identity: (u64, u64),
}
fn win_result(code: u32) -> io::Result<()> {
    if code == 0 {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(code as i32))
    }
}
fn bool_result(ok: i32) -> io::Result<()> {
    if ok != 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}
fn wide(path: &Path) -> io::Result<Vec<u16>> {
    let value: Vec<_> = path.as_os_str().encode_wide().collect();
    if value.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "NUL in save path",
        ));
    }
    Ok(value.into_iter().chain([0]).collect())
}
fn string(pointer: *const u16) -> Vec<u16> {
    // SAFETY: callers pass a successful native API's NUL-terminated allocation.
    unsafe {
        let mut length = 0;
        while *pointer.add(length) != 0 {
            length += 1;
        }
        std::slice::from_raw_parts(pointer, length).to_vec()
    }
}
fn descriptor_text(descriptor: PSECURITY_DESCRIPTOR, parts: u32) -> io::Result<Vec<u16>> {
    let mut text = null_mut();
    // SAFETY: descriptor is live, outputs have valid storage; API allocates text.
    unsafe {
        bool_result(ConvertSecurityDescriptorToStringSecurityDescriptorW(
            descriptor,
            1,
            parts,
            &mut text,
            null_mut(),
        ))?;
    }
    let _text = Allocation(text.cast());
    Ok(string(text))
}
fn snapshot(file: &File) -> io::Result<Snapshot> {
    let (mut owner, mut group, mut dacl, mut label, mut descriptor) =
        (null_mut(), null_mut(), null_mut(), null_mut(), null_mut());
    // SAFETY: handle is live and every requested output has valid storage.
    unsafe {
        win_result(GetSecurityInfo(
            file.as_raw_handle(),
            SE_FILE_OBJECT,
            PARTS | ATTRIBUTE_SECURITY_INFORMATION | SCOPE_SECURITY_INFORMATION,
            &mut owner,
            &mut group,
            &mut dacl,
            &mut label,
            &mut descriptor,
        ))?;
    }
    let allocation = Allocation(descriptor);
    let (mut control, mut revision) = (0, 0);
    // SAFETY: descriptor remains owned by allocation throughout this function.
    unsafe {
        bool_result(GetSecurityDescriptorControl(
            descriptor,
            &mut control,
            &mut revision,
        ))?;
    }
    if !label.is_null() {
        // The requested SACL subset contains labels and optional resource/CAP
        // policy. Those policies need more than ordinary file DACL copying.
        // SAFETY: label points into the descriptor owned by allocation.
        for index in 0..unsafe { (*label).AceCount } {
            let mut ace = null_mut();
            // SAFETY: valid ACL, in-range index, valid output pointer.
            unsafe {
                bool_result(GetAce(label, index as u32, &mut ace))?;
            }
            // SAFETY: successful GetAce returns an ACE beginning with ACE_HEADER.
            if unsafe { (*ace.cast::<ACE_HEADER>()).AceType } != 0x11 {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "cannot preserve this save's Windows resource or central access policy",
                ));
            }
        }
    }
    let text = descriptor_text(descriptor, PARTS)?;
    let mut info = std::mem::MaybeUninit::uninit();
    // SAFETY: live handle and appropriately sized output buffer.
    unsafe {
        bool_result(
            windows_sys::Win32::Storage::FileSystem::GetFileInformationByHandle(
                file.as_raw_handle(),
                info.as_mut_ptr(),
            ),
        )?;
    }
    // SAFETY: the successful API initialized the output.
    let info = unsafe { info.assume_init() };
    Ok(Snapshot {
        _allocation: allocation,
        owner,
        group,
        label,
        protected: control & SE_DACL_PROTECTED != 0,
        automatic: control & windows_sys::Win32::Security::SE_DACL_AUTO_INHERITED != 0,
        dacl,
        text,
        identity: (
            info.dwVolumeSerialNumber as u64,
            ((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64,
        ),
    })
}
pub(super) fn capture(file: &File, path: &Path) -> io::Result<Snapshot> {
    if file.metadata()?.file_attributes() & FILE_ATTRIBUTE_ENCRYPTED != 0 {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "cannot preserve this save's EFS encryption during atomic replacement",
        ));
    }
    reject_named_streams(path)?;
    snapshot(file)
}
pub(super) fn apply(old: &Snapshot, file: &File, path: &Path) -> io::Result<()> {
    let current = snapshot(file)?;
    let mut parts = 0;
    // Avoid assigning unchanged SIDs or manufacturing a null SACL where the
    // original descriptor has no integrity-label state.
    unsafe {
        if EqualSid(old.owner, current.owner) == 0 {
            parts |= OWNER_SECURITY_INFORMATION;
        }
        if EqualSid(old.group, current.group) == 0 {
            parts |= GROUP_SECURITY_INFORMATION;
        }
    }
    if descriptor_text(old._allocation.0, LABEL_SECURITY_INFORMATION)?
        != descriptor_text(current._allocation.0, LABEL_SECURITY_INFORMATION)?
    {
        parts |= LABEL_SECURITY_INFORMATION;
    }
    if parts != 0 {
        // SAFETY: handle and every supplied SID/ACL allocation remain live.
        unsafe {
            win_result(SetSecurityInfo(
                file.as_raw_handle(),
                SE_FILE_OBJECT,
                parts,
                old.owner,
                old.group,
                null_mut(),
                old.label,
            ))?;
        }
    }
    let protection = if old.protected {
        PROTECTED_DACL_SECURITY_INFORMATION
    } else {
        UNPROTECTED_DACL_SECURITY_INFORMATION
    };
    if old.protected || old.automatic {
        // The modern API preserves protected and auto-inherited descriptors.
        // SAFETY: the handle and source ACL allocation remain live.
        unsafe {
            win_result(SetSecurityInfo(
                file.as_raw_handle(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION | protection,
                null_mut(),
                null_mut(),
                old.dacl,
                null_mut(),
            ))?;
        }
    } else {
        let path = wide(path)?;
        // SetSecurityInfo imposes automatic inheritance on legacy unprotected
        // ACLs, adding parent ACEs absent from the original. The documented
        // obsolete file API preserves that legacy descriptor. It runs only on
        // our empty, exclusively held staging file; verify through its handle.
        // https://learn.microsoft.com/windows/win32/api/securitybaseapi/nf-securitybaseapi-setfilesecurityw
        // SAFETY: valid NUL-terminated path and live security descriptor.
        unsafe {
            bool_result(SetFileSecurityW(
                path.as_ptr(),
                DACL_SECURITY_INFORMATION | protection,
                old._allocation.0,
            ))?;
        }
    }
    if snapshot(file)?.text != old.text {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Windows could not preserve the save's owner, group, DACL and integrity label",
        ));
    }
    Ok(())
}
pub(super) fn recheck(old: &Snapshot, source: &File, path: &Path, staged: &File) -> io::Result<()> {
    // Query by path as well as the original handle to notice replacement.
    let current = OpenOptions::new().access_mode(READ_CONTROL).open(path)?;
    let current = snapshot(&current)?;
    if current.identity != old.identity
        || current.text != old.text
        || snapshot(source)?.text != old.text
        || snapshot(staged)?.text != old.text
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Windows save identity or access controls changed before publication",
        ));
    }
    Ok(())
}
pub(super) fn create_private(path: &Path) -> io::Result<File> {
    // Use the actual process user SID; creator-owner aliases in a protected
    // descriptor are not a substitute for an explicit access grant.
    let mut token = null_mut();
    // SAFETY: valid pseudo-handle and output pointer; successful token is owned.
    unsafe {
        bool_result(OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_QUERY,
            &mut token,
        ))?;
    }
    // SAFETY: successful OpenProcessToken transfers an owned handle.
    let token = unsafe { OwnedHandle::from_raw_handle(token) };
    let mut length = 0;
    // SAFETY: first call only requests the required size.
    unsafe {
        GetTokenInformation(token.as_raw_handle(), TokenUser, null_mut(), 0, &mut length);
    }
    let mut buffer = vec![0usize; (length as usize).div_ceil(std::mem::size_of::<usize>())];
    // SAFETY: aligned, sufficiently sized buffer and live token handle.
    unsafe {
        bool_result(GetTokenInformation(
            token.as_raw_handle(),
            TokenUser,
            buffer.as_mut_ptr().cast(),
            length,
            &mut length,
        ))?;
    }
    // SAFETY: API initialized a TOKEN_USER at the aligned buffer start.
    let user = unsafe { &*buffer.as_ptr().cast::<TOKEN_USER>() };
    let mut sid = null_mut();
    // SAFETY: SID points inside live buffer; output is LocalAlloc-owned.
    unsafe {
        bool_result(ConvertSidToStringSidW(user.User.Sid, &mut sid))?;
    }
    let _sid = Allocation(sid.cast());
    let sddl = format!(
        "D:P(A;;FA;;;{})",
        String::from_utf16(&string(sid)).map_err(io::Error::other)?
    );
    let sddl: Vec<_> = sddl.encode_utf16().chain([0]).collect();
    let mut descriptor = null_mut();
    // SAFETY: NUL terminated input, valid descriptor output.
    unsafe {
        bool_result(ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            1,
            &mut descriptor,
            null_mut(),
        ))?;
    }
    let _descriptor = Allocation(descriptor);
    let attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor,
        bInheritHandle: 0,
    };
    let path = wide(path)?;
    // SAFETY: valid path and descriptor. Exclusive sharing prevents data reads
    // while writing, even after applying an intentionally shared old DACL.
    let handle = unsafe {
        CreateFileW(
            path.as_ptr(),
            GENERIC_READ | GENERIC_WRITE | READ_CONTROL | WRITE_DAC | WRITE_OWNER,
            0,
            &attributes,
            CREATE_NEW,
            FILE_ATTRIBUTE_NORMAL,
            null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: successful CreateFileW transfers the newly created handle.
    let file = unsafe { File::from_raw_handle(handle) };
    let verified = (|| {
        let current = snapshot(&file)?;
        if descriptor_text(current._allocation.0, DACL_SECURITY_INFORMATION)?
            != descriptor_text(descriptor, DACL_SECURITY_INFORMATION)?
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "this save location did not honor the requested private Windows staging DACL",
            ));
        }
        Ok(())
    })();
    if let Err(error) = verified {
        drop(file);
        // Still empty and not entered into the storage ledger. Only remove the
        // file created by this call; a cleanup failure can leave an empty sibling.
        let _ = std::fs::remove_file(std::ffi::OsString::from_wide(&path[..path.len() - 1]));
        return Err(error);
    }
    Ok(file)
}

fn reject_named_streams(path: &Path) -> io::Result<()> {
    use windows_sys::Win32::{
        Foundation::{ERROR_HANDLE_EOF, ERROR_INVALID_PARAMETER, ERROR_NOT_SUPPORTED},
        Storage::FileSystem::{
            FindClose, FindFirstStreamW, FindNextStreamW, FindStreamInfoStandard,
            WIN32_FIND_STREAM_DATA,
        },
    };
    let path = wide(path)?;
    let mut data = std::mem::MaybeUninit::<WIN32_FIND_STREAM_DATA>::uninit();
    // SAFETY: valid path and correctly sized output buffer.
    let handle = unsafe {
        FindFirstStreamW(
            path.as_ptr(),
            FindStreamInfoStandard,
            data.as_mut_ptr().cast(),
            0,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        let error = io::Error::last_os_error();
        // Filesystems without this API retain ordinary saves; do not infer
        // support for streams or a universal metadata-preservation guarantee.
        return match error.raw_os_error().map(|code| code as u32) {
            Some(ERROR_HANDLE_EOF | ERROR_INVALID_PARAMETER | ERROR_NOT_SUPPORTED) => Ok(()),
            _ => Err(error),
        };
    }
    struct Search(windows_sys::Win32::Foundation::HANDLE);
    impl Drop for Search {
        fn drop(&mut self) {
            // SAFETY: successful stream search handle is closed exactly once.
            unsafe {
                FindClose(self.0);
            }
        }
    }
    let _search = Search(handle);
    loop {
        // SAFETY: successful FindFirst/NextStreamW initialized data.
        let value = unsafe { data.assume_init_ref() };
        let end = value
            .cStreamName
            .iter()
            .position(|&unit| unit == 0)
            .unwrap_or(value.cStreamName.len());
        if value.cStreamName[..end] != "::$DATA".encode_utf16().collect::<Vec<_>>() {
            return Err(io::Error::new(io::ErrorKind::Unsupported,
                "save has named Windows data streams; keep the original and use an explicit new save path"));
        }
        // SAFETY: live search handle and correctly sized output buffer.
        if unsafe { FindNextStreamW(handle, data.as_mut_ptr().cast()) } == 0 {
            let error = io::Error::last_os_error();
            return if error.raw_os_error() == Some(ERROR_HANDLE_EOF as i32) {
                Ok(())
            } else {
                Err(error)
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inherited_descriptor_round_trip() {
        let root = std::env::temp_dir().join(format!(
            "alife-win-security-roundtrip-{}",
            std::process::id()
        ));
        std::fs::create_dir(&root).unwrap();
        let root = std::fs::canonicalize(root).unwrap();
        let source = root.join("source.json");
        std::fs::write(&source, b"original").unwrap();
        let source = OpenOptions::new().write(true).open(source).unwrap();
        let old = snapshot(&source).unwrap();
        let stage = root.join("stage.json");
        let stage_file = create_private(&stage).unwrap();
        let result = apply(&old, &stage_file, &stage);
        let copied = snapshot(&stage_file).unwrap();
        assert!(
            result.is_ok(),
            "{result:?}; original={}; staged={}",
            String::from_utf16_lossy(&old.text),
            String::from_utf16_lossy(&copied.text)
        );
        drop(stage_file);
        drop(source);
        std::fs::remove_dir_all(root).unwrap();
    }
}
