use std::{collections::BTreeMap, ffi::OsString, fs::File, io, os::unix::ffi::OsStringExt};

pub(super) type Attributes = BTreeMap<OsString, Vec<u8>>;
const ACL: &str = "system.posix_acl_access";

fn value(file: &File, name: &std::ffi::OsStr) -> io::Result<Option<Vec<u8>>> {
    let size = match rustix::fs::fgetxattr(file, name, &mut [0_u8; 0]) {
        Ok(size) => size,
        Err(rustix::io::Errno::NODATA | rustix::io::Errno::OPNOTSUPP) => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let mut bytes = vec![0; size];
    let size = rustix::fs::fgetxattr(file, name, &mut bytes)?;
    bytes.truncate(size);
    Ok(Some(bytes))
}

pub(super) fn read(file: &File) -> io::Result<Attributes> {
    let size = match rustix::fs::flistxattr(file, &mut [0_u8; 0]) {
        Ok(size) => size,
        // Filesystems without Linux xattrs still support ordinary mode saves.
        Err(rustix::io::Errno::OPNOTSUPP) => 0,
        Err(error) => return Err(error.into()),
    };
    let mut names = vec![0_u8; size];
    if size != 0 {
        let size = rustix::fs::flistxattr(file, &mut names)?;
        names.truncate(size);
    }
    let mut attributes = Attributes::new();
    for name in names
        .split(|byte| *byte == 0)
        .filter(|name| !name.is_empty())
    {
        let name = OsString::from_vec(name.to_vec());
        // These describe privileged execution or signatures of the old data.
        // Copying them onto changed content is not a metadata-preserving save.
        if ["security.capability", "security.ima", "security.evm"]
            .iter()
            .any(|key| name == *key)
        {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!("cannot retain content-dependent privileged attribute {name:?}"),
            ));
        }
        let bytes = value(file, &name)?.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::Interrupted,
                "save attribute disappeared during inspection",
            )
        })?;
        attributes.insert(name, bytes);
    }
    // Query the POSIX ACL directly as well: enumeration can filter names.
    if !attributes.contains_key(std::ffi::OsStr::new(ACL)) {
        if let Some(acl) = value(file, std::ffi::OsStr::new(ACL))? {
            attributes.insert(OsString::from(ACL), acl);
        }
    }
    Ok(attributes)
}

pub(super) fn apply(attributes: &Attributes, staged: &File) -> io::Result<()> {
    let inherited = read(staged)?;
    // Remove inherited policy absent from the existing destination while
    // staging still has mode 0600. In particular, never unmask a different ACL
    // by first restoring the destination's shared mode bits.
    for name in inherited
        .keys()
        .filter(|name| !attributes.contains_key(*name))
    {
        rustix::fs::fremovexattr(staged, name.as_os_str())?;
    }
    for (name, bytes) in attributes
        .iter()
        .filter(|(name, _)| name.as_os_str() != ACL)
    {
        if inherited.get(name) != Some(bytes) {
            rustix::fs::fsetxattr(
                staged,
                name.as_os_str(),
                bytes,
                rustix::fs::XattrFlags::empty(),
            )?;
        }
    }
    // Apply the old ACL after the other visible metadata. Its owner/group
    // principals already match, and the complete new content has been written.
    if let Some(acl) = attributes.get(std::ffi::OsStr::new(ACL)) {
        if inherited.get(std::ffi::OsStr::new(ACL)) != Some(acl) {
            rustix::fs::fsetxattr(staged, ACL, acl, rustix::fs::XattrFlags::empty())?;
        }
    }
    Ok(())
}
