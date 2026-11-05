use std::{fs, path::PathBuf};

use alife_core::{BrainScaleTier, OrganismId, Vec3f};
use alife_world::{
    persistence::{AssetManifest, PortableSaveFile, RuntimeConfig},
    HeadlessScenarioBuilder,
};

fn fixture(name: &str) -> PortableSaveFile {
    let world = HeadlessScenarioBuilder::new(4242)
        .agent("creature", OrganismId(1), Vec3f::ZERO)
        .food("berry", Vec3f::new(1.0, 0.0, 0.0), 0.75)
        .build()
        .unwrap();
    PortableSaveFile::from_headless_world(
        name,
        &world,
        RuntimeConfig::deterministic_default(4242, BrainScaleTier::Nano512),
        AssetManifest::empty(),
        vec![],
    )
    .unwrap()
}

fn temp_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "alife-atomic-portable-save-{name}-{}",
        std::process::id()
    ));
    fs::create_dir(&root).unwrap();
    // macOS temp_dir commonly traverses /var -> /private/var. Budget fixtures
    // must use their real owned path; do not weaken the output alias guard.
    fs::canonicalize(root).unwrap()
}

#[cfg(windows)]
fn windows_acl_script(path: &std::path::Path, script: &str) -> String {
    let output = std::process::Command::new("pwsh.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .env("ALIFE_DACL_PATH", path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

#[cfg(windows)]
fn windows_sddl(path: &std::path::Path) -> String {
    windows_acl_script(path, "(Get-Acl -LiteralPath $env:ALIFE_DACL_PATH).Sddl")
}

#[cfg(windows)]
#[test]
fn windows_ordinary_inherited_save_retains_security_and_new_save_is_private() {
    let root = temp_root("inherited-dacl");
    let path = root.join("inherited.json");
    let original = fixture("inherited");
    fs::write(&path, original.to_json_string_pretty().unwrap()).unwrap();
    let before = windows_sddl(&path);
    original.to_json_file(&path).unwrap();
    assert_eq!(windows_sddl(&path), before);
    assert_eq!(PortableSaveFile::from_json_file(&path).unwrap(), original);
    let new = root.join("new.json");
    original.to_json_file(&new).unwrap();
    let acl = windows_sddl(&new);
    assert!(
        acl.contains("D:P"),
        "new save DACL must be protected: {acl}"
    );
    assert!(
        !acl.contains(";;;WD)"),
        "new save grants Everyone access: {acl}"
    );
    fs::remove_dir_all(root).unwrap();
}

#[cfg(windows)]
#[test]
fn windows_native_metadata_supports_relative_and_unprefixed_long_save_paths() {
    let relative = PathBuf::from(format!("alife-relative-save-{}", std::process::id()));
    fs::create_dir(&relative).unwrap();
    let path = relative.join("creature.json");
    let original = fixture("relative");
    original.to_json_file(&path).unwrap();
    fixture("relative replacement").to_json_file(&path).unwrap();
    assert_eq!(
        PortableSaveFile::from_json_file(&path).unwrap().save_id,
        "relative replacement"
    );
    fs::remove_dir_all(relative).unwrap();

    let root = temp_root("long-native-path");
    let mut nested = root.clone();
    for index in 0..7 {
        nested.push(format!("long-save-directory-{index}-abcdefghijklmnop"));
    }
    fs::create_dir_all(&nested).unwrap();
    let path = nested.join("creature.json");
    // The API must also accept ordinary absolute spellings, as std::fs does.
    let normal = path.to_str().unwrap().strip_prefix(r"\\?\").unwrap();
    assert!(normal.len() > 260);
    original.to_json_file(normal).unwrap();
    let replacement = fixture("long replacement");
    replacement.to_json_file(normal).unwrap();
    assert_eq!(
        PortableSaveFile::from_json_file(&path).unwrap(),
        replacement
    );
    fs::remove_dir_all(root).unwrap();
}

#[cfg(windows)]
#[test]
fn windows_modern_inherited_acl_survives_replacement() {
    let root = temp_root("modern-inherited-dacl");
    let path = root.join("inherited.json");
    let original = fixture("modern inherited");
    fs::write(&path, original.to_json_string_pretty().unwrap()).unwrap();
    windows_acl_script(
        &root,
        r#"
        $ErrorActionPreference = 'Stop'
        $acl = Get-Acl -LiteralPath $env:ALIFE_DACL_PATH
        $everyone = [System.Security.Principal.SecurityIdentifier]::new('S-1-1-0')
        $acl.AddAccessRule([System.Security.AccessControl.FileSystemAccessRule]::new(
            $everyone, 'ReadAndExecute', 'ContainerInherit,ObjectInherit', 'None', 'Allow'))
        Set-Acl -LiteralPath $env:ALIFE_DACL_PATH -AclObject $acl
    "#,
    );
    let before = windows_sddl(&path);
    assert!(
        before.contains("D:AI") && before.contains(";ID;"),
        "fixture did not produce an automatically inherited ACL: {before}"
    );
    let replacement = fixture("replacement");
    replacement.to_json_file(&path).unwrap();
    assert_eq!(windows_sddl(&path), before);
    assert_eq!(
        PortableSaveFile::from_json_file(&path).unwrap(),
        replacement
    );
    fs::remove_dir_all(root).unwrap();
}

#[cfg(windows)]
#[test]
fn windows_write_denied_dacl_is_not_bypassed_by_directory_rename() {
    let root = temp_root("write-denied-dacl");
    let path = root.join("denied.json");
    let original = fixture("write denied");
    fs::write(&path, original.to_json_string_pretty().unwrap()).unwrap();
    windows_acl_script(
        &path,
        r#"
        $ErrorActionPreference = 'Stop'
        $acl = Get-Acl -LiteralPath $env:ALIFE_DACL_PATH
        $me = [System.Security.Principal.WindowsIdentity]::GetCurrent().User
        $acl.AddAccessRule([System.Security.AccessControl.FileSystemAccessRule]::new($me, 'WriteData', 'Deny'))
        Set-Acl -LiteralPath $env:ALIFE_DACL_PATH -AclObject $acl
    "#,
    );
    let before = windows_sddl(&path);
    assert!(fs::OpenOptions::new().write(true).open(&path).is_err());
    let error = fixture("replacement").to_json_file(&path).unwrap_err();
    eprintln!("native DACL write refusal: {error}");
    assert_eq!(PortableSaveFile::from_json_file(&path).unwrap(), original);
    assert_eq!(windows_sddl(&path), before);
    assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
    fs::remove_dir_all(root).unwrap();
}

#[cfg(windows)]
#[test]
fn windows_named_stream_refusal_preserves_original_and_recovery() {
    let root = temp_root("named-stream");
    let path = root.join("stream.json");
    let original = fixture("stream owner");
    fs::write(&path, original.to_json_string_pretty().unwrap()).unwrap();
    let stream = path.with_file_name("stream.json:recovery");
    fs::write(&stream, b"recoverable stream").unwrap();
    let before = windows_sddl(&path);
    let error = fixture("replacement").to_json_file(&path).unwrap_err();
    assert!(
        error.to_string().contains("named Windows data streams"),
        "{error}"
    );
    assert_eq!(PortableSaveFile::from_json_file(&path).unwrap(), original);
    assert_eq!(fs::read(&stream).unwrap(), b"recoverable stream");
    assert_eq!(windows_sddl(&path), before);
    assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
    // Removing only this disposable extra stream permits an ordinary retry.
    fs::remove_file(stream).unwrap();
    let replacement = fixture("replacement");
    replacement.to_json_file(&path).unwrap();
    assert_eq!(
        PortableSaveFile::from_json_file(&path).unwrap(),
        replacement
    );
    fs::remove_dir_all(root).unwrap();
}

#[cfg(windows)]
#[test]
#[allow(clippy::permissions_set_readonly_false)] // Windows-only attribute reset.
fn windows_explicit_restricted_dacl_survives_broader_parent() {
    let root = temp_root("explicit-dacl");
    let path = root.join("creature.alife.json");
    let original = fixture("previous");
    fs::write(&path, original.to_json_string_pretty().unwrap()).unwrap();
    let setup = std::process::Command::new("pwsh.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", r#"
            $ErrorActionPreference = 'Stop'
            $path = $env:ALIFE_DACL_PATH
            $parent = $env:ALIFE_DACL_PARENT
            $everyone = [System.Security.Principal.SecurityIdentifier]::new('S-1-1-0')
            $parentAcl = Get-Acl -LiteralPath $parent
            $parentAcl.AddAccessRule([System.Security.AccessControl.FileSystemAccessRule]::new(
                $everyone, 'ReadAndExecute', 'ContainerInherit,ObjectInherit', 'None', 'Allow'))
            Set-Acl -LiteralPath $parent -AclObject $parentAcl
            $acl = Get-Acl -LiteralPath $path
            $acl.SetAccessRuleProtection($true, $false)
            foreach ($rule in @($acl.Access)) { $acl.RemoveAccessRuleSpecific($rule) }
            $me = [System.Security.Principal.WindowsIdentity]::GetCurrent().User
            $acl.AddAccessRule([System.Security.AccessControl.FileSystemAccessRule]::new($me, 'FullControl', 'Allow'))
            Set-Acl -LiteralPath $path -AclObject $acl
            (Get-Acl -LiteralPath $path).Sddl
        "#])
        .env("ALIFE_DACL_PATH", &path).env("ALIFE_DACL_PARENT", &root)
        .output().unwrap();
    assert!(
        setup.status.success(),
        "{}",
        String::from_utf8_lossy(&setup.stderr)
    );
    let before = String::from_utf8(setup.stdout).unwrap();
    assert!(
        before.contains("D:P"),
        "fixture DACL is not protected: {before}"
    );
    let mut replacement = original.clone();
    replacement.save_id = "replacement".to_string();
    // Both a read-only flag and a denied delete-sharing handle must preserve
    // the restricted descriptor and old content. Closing the blocker allows retry.
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&path, permissions).unwrap();
    assert!(replacement.to_json_file(&path).is_err());
    assert_eq!(PortableSaveFile::from_json_file(&path).unwrap(), original);
    assert_eq!(windows_sddl(&path), before.trim());
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_readonly(false);
    fs::set_permissions(&path, permissions).unwrap();
    use std::os::windows::fs::OpenOptionsExt;
    let blocker = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(3)
        .open(&path)
        .unwrap();
    assert!(replacement.to_json_file(&path).is_err());
    assert_eq!(PortableSaveFile::from_json_file(&path).unwrap(), original);
    assert_eq!(windows_sddl(&path), before.trim());
    assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
    drop(blocker);
    replacement.to_json_file(&path).unwrap();
    assert_eq!(
        PortableSaveFile::from_json_file(&path).unwrap(),
        replacement
    );
    let after = std::process::Command::new("pwsh.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "(Get-Acl -LiteralPath $env:ALIFE_DACL_PATH).Sddl",
        ])
        .env("ALIFE_DACL_PATH", &path)
        .output()
        .unwrap();
    assert!(after.status.success());
    assert_eq!(
        String::from_utf8(after.stdout).unwrap().trim(),
        before.trim(),
        "replacement changed the explicit DACL"
    );
    fs::remove_dir_all(root).unwrap();
}

#[cfg(windows)]
#[test]
#[allow(clippy::permissions_set_readonly_false)] // Windows-only attribute reset.
fn windows_read_only_save_is_not_replaced() {
    let root = temp_root("read-only");
    let path = root.join("creature.alife.json");
    let original = fixture("protected").to_json_string_pretty().unwrap();
    fs::write(&path, &original).unwrap();
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&path, permissions).unwrap();
    let error = fixture("replacement").to_json_file(&path).unwrap_err();
    assert!(error.to_string().contains("unchanged"), "{error}");
    assert_eq!(fs::read(&path).unwrap(), original.as_bytes());
    assert!(fs::metadata(&path).unwrap().permissions().readonly());
    assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_readonly(false);
    fs::set_permissions(&path, permissions).unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[cfg(target_os = "macos")]
fn macos_acl(path: &std::path::Path) -> String {
    let output = std::process::Command::new("ls")
        .arg("-le")
        .arg(path)
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .skip(1)
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(target_os = "macos")]
#[test]
fn macos_native_acl_and_xattr_survive_replacement() {
    use std::{os::unix::fs::PermissionsExt, process::Command};
    let root = temp_root("macos-metadata");
    let path = root.join("creature.alife.json");
    fs::write(&path, fixture("protected").to_json_string_pretty().unwrap()).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(Command::new("chmod")
        .args(["+a", "group:everyone allow read"])
        .arg(&path)
        .status()
        .unwrap()
        .success());
    assert!(Command::new("xattr")
        .args(["-w", "org.alife.save-test", "retained"])
        .arg(&path)
        .status()
        .unwrap()
        .success());
    let before = macos_acl(&path);
    assert!(!before.is_empty(), "ACL fixture was not created");
    fixture("replacement").to_json_file(&path).unwrap();
    assert_eq!(
        PortableSaveFile::from_json_file(&path).unwrap().save_id,
        "replacement"
    );
    assert_eq!(macos_acl(&path), before);
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let value = Command::new("xattr")
        .args(["-p", "org.alife.save-test"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(value.status.success());
    assert_eq!(String::from_utf8(value.stdout).unwrap().trim(), "retained");
    fs::remove_dir_all(root).unwrap();
}

#[cfg(target_os = "macos")]
#[test]
fn macos_inherited_acl_does_not_broaden_save_access() {
    use std::{os::unix::fs::PermissionsExt, process::Command};
    let root = temp_root("macos-inherited");
    let path = root.join("creature.alife.json");
    fs::write(&path, fixture("protected").to_json_string_pretty().unwrap()).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let before = macos_acl(&path);
    assert!(Command::new("chmod")
        .args([
            "+a",
            "group:everyone allow read,file_inherit,directory_inherit"
        ])
        .arg(&root)
        .status()
        .unwrap()
        .success());
    fixture("replacement").to_json_file(&path).unwrap();
    assert_eq!(macos_acl(&path), before);
    let new = root.join("new.alife.json");
    fixture("new").to_json_file(&new).unwrap();
    assert!(
        macos_acl(&new).is_empty(),
        "new save inherited a readable ACL"
    );
    assert_eq!(
        fs::metadata(&new).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(fs::read_dir(&root).unwrap().count(), 2);
    fs::remove_dir_all(root).unwrap();
}

#[cfg(target_os = "macos")]
#[test]
fn macos_post_write_metadata_failure_keeps_save_and_cleans_staging() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let root = temp_root("macos-post-write-metadata");
    let original = fixture("previous");
    let bytes = original.to_json_string_pretty().unwrap().into_bytes();
    let mut replacement = original.clone();
    replacement.save_id = "replacement with different bytes".to_string();
    let replacement_bytes = replacement.to_json_string_pretty().unwrap().into_bytes();
    let peak = (bytes.len() + replacement_bytes.len()) as u64;
    let budget = alife_storage::OutputBudget::register(&root, peak).unwrap();
    let path = root.join("save.json");
    alife_storage::write(&path, &bytes).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o200)).unwrap();
    let before = fs::metadata(&path).unwrap();
    let error = replacement.to_json_file(&path).unwrap_err();
    assert!(error.to_string().contains("ACLs/xattrs"), "{error}");
    let after = fs::metadata(&path).unwrap();
    assert_eq!((after.ino(), after.mode()), (before.ino(), before.mode()));
    assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
    let usage = budget.usage().unwrap();
    assert_eq!(
        (usage.high_water, usage.used, usage.reserved),
        (peak, bytes.len() as u64, 0)
    );
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(fs::read(&path).unwrap(), bytes);
    replacement.to_json_file(&path).unwrap();
    assert_eq!(
        PortableSaveFile::from_json_file(&path).unwrap(),
        replacement
    );
    alife_storage::remove_dir_all(&root).unwrap();
}

#[cfg(unix)]
#[test]
fn unix_read_only_save_is_not_replaced() {
    use std::os::unix::fs::PermissionsExt;
    let root = temp_root("read-only");
    let path = root.join("creature.alife.json");
    let original = fixture("protected").to_json_string_pretty().unwrap();
    let replacement = fixture("replacement");
    let mut overwritten = vec![];
    for mode in [0o000, 0o400] {
        fs::write(&path, &original).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
        // The pre-atomic implementation used this exact direct-write API.
        let direct = alife_storage::write(&path, b"direct replacement");
        if direct.is_ok() {
            eprintln!("read-only protection test requires an unprivileged process");
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
            fs::remove_dir_all(root).unwrap();
            return;
        }
        let atomic = replacement.to_json_file(&path);
        eprintln!("mode {mode:03o}: direct={direct:?}, atomic={atomic:?}");
        let retained_mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        if atomic.is_ok()
            || fs::read(&path).unwrap() != original.as_bytes()
            || retained_mode != mode
        {
            overwritten.push(mode);
        }
    }
    fs::remove_dir_all(root).unwrap();
    assert!(
        overwritten.is_empty(),
        "replaced read-only modes: {overwritten:?}"
    );
}

#[cfg(unix)]
#[test]
fn unix_save_permissions_under_controlled_umasks() {
    for mask in ["000", "022", "077"] {
        let status = std::process::Command::new("sh")
            .args([
                "-c",
                "umask \"$1\"; shift; exec \"$@\"",
                "alife-save-mode",
                mask,
            ])
            .arg(std::env::current_exe().unwrap())
            .args(["--exact", "unix_save_mode_child", "--nocapture"])
            .env("ALIFE_SAVE_MODE_CHILD", "1")
            .status()
            .unwrap();
        assert!(status.success(), "save permissions under umask {mask}");
    }
}

#[cfg(unix)]
#[test]
fn unix_save_mode_child() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    if std::env::var_os("ALIFE_SAVE_MODE_CHILD").is_none() {
        return;
    }
    let root = temp_root("unix-modes");
    let path = root.join("creature.alife.json");
    let mut save = fixture("private");
    // Exercise the public API against an existing private save first.
    fs::write(&path, save.to_json_string_pretty().unwrap()).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    save.save_id = "replacement".to_string();
    save.to_json_file(&path).unwrap();
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        PortableSaveFile::from_json_file(&path).unwrap().save_id,
        "replacement"
    );
    // Ordinary private and shared modes are retained exactly. Read-only modes
    // are covered separately against the pre-atomic direct-write behavior.
    for mode in [0o200, 0o600, 0o640, 0o644] {
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
        let before = fs::metadata(&path).unwrap();
        let result = save.to_json_file(&path);
        #[cfg(target_os = "macos")]
        if mode == 0o200 {
            let error = result.unwrap_err();
            assert!(error.to_string().contains("ACLs/xattrs"), "{error}");
            assert!(error.to_string().contains("unchanged"), "{error}");
            let after = fs::metadata(&path).unwrap();
            assert_eq!((after.ino(), after.mode()), (before.ino(), before.mode()));
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
            assert_eq!(
                fs::read(&path).unwrap(),
                save.to_json_string_pretty().unwrap().as_bytes()
            );
            fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
            continue;
        }
        result.unwrap();
        let after = fs::metadata(&path).unwrap();
        assert_eq!((after.uid(), after.gid()), (before.uid(), before.gid()));
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            mode
        );
    }
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    fs::remove_file(&path).unwrap();
    save.to_json_file(&path).unwrap();
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    fs::remove_dir_all(root).unwrap();
}

#[cfg(target_os = "linux")]
fn linux_acl(owner: u16, named: u16, mask: u16, uid: u32) -> Vec<u8> {
    let mut bytes = 2_u32.to_le_bytes().to_vec();
    for (tag, permissions, id) in [
        (1_u16, owner, u32::MAX),
        (2, named, uid),
        (4, 0, u32::MAX),
        (16, mask, u32::MAX),
        (32, 0, u32::MAX),
    ] {
        bytes.extend_from_slice(&tag.to_le_bytes());
        bytes.extend_from_slice(&permissions.to_le_bytes());
        bytes.extend_from_slice(&id.to_le_bytes());
    }
    bytes
}

#[cfg(target_os = "linux")]
fn set_fixture_attribute(file: &fs::File, name: &str, value: &[u8]) -> bool {
    match rustix::fs::fsetxattr(file, name, value, rustix::fs::XattrFlags::empty()) {
        Ok(()) => true,
        Err(rustix::io::Errno::OPNOTSUPP) => {
            eprintln!("filesystem does not support {name}; fixture skipped");
            false
        }
        Err(error) => panic!("could not create {name} fixture: {error}"),
    }
}

#[cfg(target_os = "linux")]
#[test]
fn linux_special_permission_bits_are_refused_unchanged() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let root = temp_root("special-mode");
    let path = root.join("creature.alife.json");
    let original = fixture("protected").to_json_string_pretty().unwrap();
    fs::write(&path, &original).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o4600)).unwrap();
    let before = fs::metadata(&path).unwrap();
    let error = fixture("replacement").to_json_file(&path).unwrap_err();
    assert!(
        error.to_string().contains("special permission bits"),
        "{error}"
    );
    assert_eq!(fs::read(&path).unwrap(), original.as_bytes());
    let after = fs::metadata(&path).unwrap();
    assert_eq!((after.ino(), after.mode()), (before.ino(), before.mode()));
    assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
    fs::remove_dir_all(root).unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn linux_attributes_and_acl_survive_replacement() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    for name in ["user.alife-save-test", "system.posix_acl_access"] {
        let root = temp_root(if name.starts_with("user") {
            "xattr"
        } else {
            "acl"
        });
        let path = root.join("creature.alife.json");
        let original = fixture("protected").to_json_string_pretty().unwrap();
        fs::write(&path, &original).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        let file = fs::File::open(&path).unwrap();
        let value = if name.starts_with("user") {
            b"retained attribute".to_vec()
        } else {
            linux_acl(6, 4, 0, file.metadata().unwrap().uid())
        };
        if set_fixture_attribute(&file, name, &value) {
            let before = file.metadata().unwrap();
            fixture("replacement").to_json_file(&path).unwrap();
            assert_eq!(
                PortableSaveFile::from_json_file(&path).unwrap().save_id,
                "replacement"
            );
            let after = fs::metadata(&path).unwrap();
            assert_eq!(
                (after.mode(), after.uid(), after.gid()),
                (before.mode(), before.uid(), before.gid())
            );
            let replacement = fs::File::open(&path).unwrap();
            let mut buffer = [0_u8; 128];
            let size = rustix::fs::fgetxattr(&replacement, name, &mut buffer).unwrap();
            assert_eq!(&buffer[..size], value);
            assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        }
        fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(target_os = "linux")]
#[test]
fn linux_directory_acl_does_not_replace_existing_acl_policy() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let root = temp_root("inherited-acl");
    let path = root.join("creature.alife.json");
    fs::write(&path, fixture("protected").to_json_string_pretty().unwrap()).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    let directory = fs::File::open(&root).unwrap();
    let acl = linux_acl(7, 4, 4, directory.metadata().unwrap().uid());
    if set_fixture_attribute(&directory, "system.posix_acl_default", &acl) {
        fixture("replacement").to_json_file(&path).unwrap();
        assert_eq!(
            PortableSaveFile::from_json_file(&path).unwrap().save_id,
            "replacement"
        );
        assert_eq!(fs::metadata(&path).unwrap().mode() & 0o777, 0o640);
        let replaced = fs::File::open(&path).unwrap();
        assert_eq!(
            rustix::fs::fgetxattr(&replaced, "system.posix_acl_access", &mut [0_u8; 0]),
            Err(rustix::io::Errno::NODATA)
        );
        let new = root.join("new.alife.json");
        fixture("new").to_json_file(&new).unwrap();
        assert_eq!(fs::metadata(&new).unwrap().mode() & 0o777, 0o600);
        assert_eq!(fs::read_dir(&root).unwrap().count(), 2);
        let mut actual = [0_u8; 128];
        let size =
            rustix::fs::fgetxattr(&directory, "system.posix_acl_default", &mut actual).unwrap();
        assert_eq!(&actual[..size], acl);
    }
    fs::remove_dir_all(root).unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn linux_unreadable_metadata_refusal_keeps_previous_save() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let root = temp_root("unreadable-metadata");
    let path = root.join("creature.alife.json");
    let original = fixture("protected").to_json_string_pretty().unwrap();
    fs::write(&path, &original).unwrap();
    let file = fs::File::open(&path).unwrap();
    if set_fixture_attribute(&file, "user.alife-save-test", b"retained") {
        fs::set_permissions(&path, fs::Permissions::from_mode(0o200)).unwrap();
        let before = fs::metadata(&path).unwrap();
        let can_read = rustix::fs::fgetxattr(&file, "user.alife-save-test", &mut [0_u8; 0]).is_ok();
        if !can_read {
            let error = fixture("replacement").to_json_file(&path).unwrap_err();
            assert!(error.to_string().contains("unchanged"), "{error}");
            assert!(
                error.to_string().contains(&path.display().to_string()),
                "{error}"
            );
            let after = fs::metadata(&path).unwrap();
            assert_eq!((after.ino(), after.mode()), (before.ino(), before.mode()));
            assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        } else {
            eprintln!("metadata refusal fixture requires an unprivileged user");
        }
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(fs::read(&path).unwrap(), original.as_bytes());
        let mut actual = [0_u8; 128];
        let size = rustix::fs::fgetxattr(&file, "user.alife-save-test", &mut actual).unwrap();
        assert_eq!(&actual[..size], b"retained");
    }
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn unix_save_rejects_symlink_without_changing_target_or_link() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let root = temp_root("symlink");
    let target = root.join("private-original");
    fs::write(&target, b"recoverable original").unwrap();
    fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).unwrap();
    let link = root.join("creature.alife.json");
    symlink(&target, &link).unwrap();
    assert!(fixture("replacement").to_json_file(&link).is_err());
    assert!(fs::symlink_metadata(&link)
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(fs::read(&target).unwrap(), b"recoverable original");
    assert_eq!(
        fs::metadata(&target).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(fs::read_dir(&root).unwrap().count(), 2);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn save_creation_and_replacement_preserve_exact_world_state_and_budget() {
    let root = temp_root("replace");
    let budget = alife_storage::OutputBudget::register(&root, 8 * 1024 * 1024).unwrap();
    let path = root.join("creature.alife.json");
    let mut save = fixture("first");
    save.to_json_file(&path).unwrap();
    save.save_id = "replacement".to_string();
    save.to_json_file(&path).unwrap();
    let bytes = fs::read(&path).unwrap();
    assert_eq!(bytes, save.to_json_string_pretty().unwrap().as_bytes());
    let restored = PortableSaveFile::from_json_file(&path).unwrap();
    assert_eq!(restored.save_id, save.save_id);
    assert_eq!(restored.world, save.world);
    assert_eq!(
        restored
            .restore_headless_world()
            .unwrap()
            .canonical_signature_digest()
            .unwrap(),
        save.restore_headless_world()
            .unwrap()
            .canonical_signature_digest()
            .unwrap()
    );
    assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
    assert_eq!(budget.usage().unwrap().used, bytes.len() as u64);
    alife_storage::remove_dir_all(&root).unwrap();
    assert_eq!(budget.usage().unwrap().used, 0);
}

#[test]
fn failed_save_write_keeps_previous_save_readable_and_releases_staging_budget() {
    let root = temp_root("write-failure");
    let original = fixture("previous");
    let bytes = original.to_json_string_pretty().unwrap().into_bytes();
    let budget = alife_storage::OutputBudget::register(&root, bytes.len() as u64 + 1).unwrap();
    let path = root.join("creature.alife.json");
    alife_storage::write(&path, &bytes).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    }
    let mut replacement = original.clone();
    replacement.save_id = "larger replacement".repeat(1024);
    let error = replacement.to_json_file(&path).unwrap_err();
    assert!(error.to_string().contains("budget exhausted"), "{error}");
    assert!(fs::read(&path).unwrap() == bytes, "previous save changed");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o640
        );
    }
    let restored = PortableSaveFile::from_json_file(&path).unwrap();
    assert_eq!(restored.save_id, original.save_id);
    assert_eq!(restored.world, original.world);
    assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
    let usage = budget.usage().unwrap();
    assert_eq!(usage.used, bytes.len() as u64);
    assert_eq!(usage.reserved, 0);
    assert!(usage.high_water <= usage.limit);
    alife_storage::remove_dir_all(&root).unwrap();
}

#[test]
fn non_regular_save_destination_is_rejected_before_staging() {
    let root = temp_root("publication-failure");
    let budget = alife_storage::OutputBudget::register(&root, 8 * 1024 * 1024).unwrap();
    let path = root.join("creature.alife.json");
    fs::create_dir(&path).unwrap();
    let sentinel = path.join("recoverable-state");
    alife_storage::write(&sentinel, b"preserve this").unwrap();
    let error = fixture("replacement").to_json_file(&path).unwrap_err();
    assert!(error.to_string().contains("regular file"), "{error}");
    assert_eq!(fs::read(&sentinel).unwrap(), b"preserve this");
    assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
    assert_eq!(budget.usage().unwrap().used, 13);
    assert_eq!(budget.usage().unwrap().high_water, 13);
    alife_storage::remove_dir_all(&root).unwrap();
}

#[test]
fn publication_failure_after_staging_keeps_previous_save_and_releases_budget() {
    let root = temp_root("publication-active-writer");
    let original = fixture("previous");
    let bytes = original.to_json_string_pretty().unwrap().into_bytes();
    let mut replacement = original.clone();
    replacement.save_id = "replacement with a different length".to_string();
    let replacement_bytes = replacement.to_json_string_pretty().unwrap().into_bytes();
    let recovery_bytes = b"recoverable staging from an earlier attempt";
    let baseline = (bytes.len() + recovery_bytes.len()) as u64;
    let peak = baseline + replacement_bytes.len() as u64;
    let budget = alife_storage::OutputBudget::register(&root, peak).unwrap();
    let path = root.join("creature.alife.json");
    let recovery = root.join(".alife-portable-save-previous.tmp");
    alife_storage::write(&path, &bytes).unwrap();
    alife_storage::write(&recovery, recovery_bytes).unwrap();

    // This non-truncating handle is registered as an active destination writer.
    // Staging may proceed, but replace_with refuses its publication at rename.
    let handle = fs::OpenOptions::new().write(true).open(&path).unwrap();
    let active_writer = alife_storage::writer(&path, handle).unwrap();
    let error = replacement.to_json_file(&path).unwrap_err();
    assert!(
        error.to_string().contains("active destination writer"),
        "{error}"
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(PortableSaveFile::from_json_file(&path).unwrap(), original);
    assert_eq!(fs::read(&recovery).unwrap(), recovery_bytes);
    assert_eq!(fs::read_dir(&root).unwrap().count(), 2);
    let usage = budget.usage().unwrap();
    // The full replacement was charged before publication failed. Its cleanup
    // must release that charge without removing the previous recovery sibling.
    assert_eq!(usage.high_water, peak);
    assert_eq!(usage.used, baseline);
    assert_eq!(usage.reserved, 0);

    drop(active_writer);
    replacement.to_json_file(&path).unwrap();
    assert_eq!(
        PortableSaveFile::from_json_file(&path).unwrap(),
        replacement
    );
    assert_eq!(fs::read(&recovery).unwrap(), recovery_bytes);
    assert_eq!(fs::read_dir(&root).unwrap().count(), 2);
    assert_eq!(
        budget.usage().unwrap().used,
        (replacement_bytes.len() + recovery_bytes.len()) as u64
    );
    alife_storage::remove_dir_all(&root).unwrap();
}

#[cfg(windows)]
#[test]
fn windows_rename_failure_after_staging_preserves_save_and_recovery() {
    use alife_world::persistence::PersistenceError;
    use std::os::windows::fs::OpenOptionsExt;
    let root = temp_root("publication-sharing-violation");
    let original = fixture("previous");
    let bytes = original.to_json_string_pretty().unwrap().into_bytes();
    let mut replacement = original.clone();
    replacement.save_id = "replacement".to_string();
    let replacement_bytes = replacement.to_json_string_pretty().unwrap().into_bytes();
    let recovery_bytes = b"keep prior recoverable work";
    let baseline = (bytes.len() + recovery_bytes.len()) as u64;
    let peak = baseline + replacement_bytes.len() as u64;
    let budget = alife_storage::OutputBudget::register(&root, peak).unwrap();
    let path = root.join("creature.alife.json");
    let recovery = root.join(".alife-portable-save-previous.tmp");
    alife_storage::write(&path, &bytes).unwrap();
    alife_storage::write(&recovery, recovery_bytes).unwrap();
    // Permit the write-access probe, but deny deletion/replacement of the
    // destination until this handle closes (FILE_SHARE_READ | FILE_SHARE_WRITE).
    let held = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0x1 | 0x2)
        .open(&path)
        .unwrap();
    let error = replacement.to_json_file(&path).unwrap_err();
    eprintln!("native publication refusal: {error:?}");
    assert!(
        matches!(&error, PersistenceError::Io(source) if matches!(source.raw_os_error(), Some(5 | 32))),
        "expected native access/sharing denial at rename, got {error}"
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(fs::read(&recovery).unwrap(), recovery_bytes);
    assert_eq!(fs::read_dir(&root).unwrap().count(), 2);
    let usage = budget.usage().unwrap();
    assert_eq!(
        (usage.high_water, usage.used, usage.reserved),
        (peak, baseline, 0)
    );
    drop(held);
    replacement.to_json_file(&path).unwrap();
    assert_eq!(
        PortableSaveFile::from_json_file(&path).unwrap(),
        replacement
    );
    assert_eq!(fs::read(&recovery).unwrap(), recovery_bytes);
    assert_eq!(fs::read_dir(&root).unwrap().count(), 2);
    alife_storage::remove_dir_all(&root).unwrap();
}
