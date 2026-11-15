// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn folder_grants_are_inherited_and_file_grants_are_not() {
    assert_eq!(
        owner_only_sddl("S-1-5-21-1", FOLDER_ACE_FLAGS),
        "D:P(A;OICI;FA;;;S-1-5-21-1)(A;OICI;FA;;;SY)"
    );
    assert_eq!(
        owner_only_sddl("S-1-5-21-1", FILE_ACE_FLAGS),
        "D:P(A;;FA;;;S-1-5-21-1)(A;;FA;;;SY)"
    );
}

#[test]
fn only_a_full_sid_text_is_accepted() {
    assert!(is_sid_text("S-1-5-21-1004336348-1177238915-682003330-1001"));
    assert!(is_sid_text("S-1-5-18"));
    for text in [
        "S-",
        "S-1-",
        "S-1-5",
        "S-1-5-",
        "S-1-5-x",
        "S-1-5-1)(A;;FA;;;WD",
        "s-1-5-18",
    ] {
        assert!(!is_sid_text(text), "{text}");
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::ffi::OsStr;
    use windows_sys::Win32::Security::Authorization::{
        ConvertSecurityDescriptorToStringSecurityDescriptorW, ConvertStringSidToSidW,
        GetNamedSecurityInfoW,
    };
    use windows_sys::Win32::Security::EqualSid;

    /// The SDDL of the access list of `path`, read back by SID.
    fn read_dacl_sddl(path: &Path) -> String {
        let wide_path = to_wide(path.as_os_str());
        let mut descriptor: *mut c_void = ptr::null_mut();
        // SAFETY: `wide_path` is NUL-terminated; the unused out pointers are null;
        // the system allocates the descriptor and the guard releases it.
        let code = unsafe {
            GetNamedSecurityInfoW(
                wide_path.as_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                &raw mut descriptor,
            )
        };
        assert_eq!(code, ERROR_SUCCESS, "read the access list");
        let _descriptor_guard = LocalAllocation(descriptor);

        let mut text: *mut u16 = ptr::null_mut();
        // SAFETY: `descriptor` is valid, and `text` is a valid out pointer.
        let converted = unsafe {
            ConvertSecurityDescriptorToStringSecurityDescriptorW(
                descriptor,
                SDDL_REVISION_1,
                DACL_SECURITY_INFORMATION,
                &raw mut text,
                ptr::null_mut(),
            )
        };
        assert_ne!(converted, 0, "convert the descriptor to SDDL");
        let _text_guard = LocalAllocation(text.cast());
        // SAFETY: on success `text` points to a NUL-terminated UTF-16 string, so
        // every index below the first NUL is readable.
        let units = unsafe {
            let mut length = 0;
            while *text.add(length) != 0 {
                length += 1;
            }
            std::slice::from_raw_parts(text, length)
        };
        String::from_utf16_lossy(units)
    }

    fn assert_owner_only(path: &Path) {
        let sddl = read_dacl_sddl(path);
        let user_sid = current_user_sid().expect("current user SID");
        assert!(sddl.starts_with("D:P"), "must be protected: {sddl}");
        assert_eq!(sddl.matches('(').count(), 2, "two entries only: {sddl}");
        assert_eq!(sddl.matches("(A;").count(), 2, "allow entries only: {sddl}");
        let trustees = entry_trustees(&sddl);
        assert!(
            trustees
                .iter()
                .any(|trustee| same_account(trustee, &user_sid)),
            "user: {sddl}"
        );
        assert!(trustees.contains(&"SY"), "SYSTEM: {sddl}");
    }

    /// The trustee of each entry, the last field of `(type;flags;rights;;;trustee)`.
    /// Windows may print a well-known account by its SDDL alias (for example
    /// `LA` for the built-in Administrator), so compare trustees as SIDs.
    fn entry_trustees(sddl: &str) -> Vec<&str> {
        sddl.split('(')
            .skip(1)
            .filter_map(|entry| entry.trim_end_matches(')').rsplit(';').next())
            .collect()
    }

    fn same_account(first: &str, second: &str) -> bool {
        let first = sid_from_text(first);
        let second = sid_from_text(second);
        // SAFETY: both pointers are valid SIDs that the guards keep alive.
        unsafe { EqualSid(first.0, second.0) != 0 }
    }

    /// A SID from its text form or its SDDL alias, released by the guard.
    fn sid_from_text(text: &str) -> LocalAllocation {
        let wide_text = to_wide(OsStr::new(text));
        let mut sid: *mut c_void = ptr::null_mut();
        // SAFETY: `wide_text` is NUL-terminated and outlives the call, and
        // `sid` is a valid out pointer that the guard releases.
        let converted = unsafe { ConvertStringSidToSidW(wide_text.as_ptr(), &raw mut sid) };
        assert_ne!(converted, 0, "not a SID: {text}");
        LocalAllocation(sid)
    }

    fn grant_users_group(path: &Path) {
        let status = Command::new("icacls.exe")
            .arg(path)
            .args(["/grant", "*S-1-5-32-545:(OI)(CI)R"])
            .status()
            .expect("run icacls");
        assert!(status.success(), "grant the Users group");
    }

    #[test]
    fn restricted_folder_holds_only_the_current_user_and_system() {
        let folder = tempfile::tempdir().expect("temporary folder");
        let private = folder.path().join("private");
        fs::create_dir(&private).expect("create folder");
        grant_users_group(&private);
        assert!(read_dacl_sddl(&private).contains(";;;BU)"));

        restrict_folder(&private).expect("restrict folder");

        assert_owner_only(&private);
    }

    #[test]
    fn restricted_file_holds_only_the_current_user_and_system() {
        let folder = tempfile::tempdir().expect("temporary folder");
        let private = folder.path().join("token");
        fs::write(&private, b"secret").expect("create file");
        grant_users_group(&private);

        restrict_file(&private).expect("restrict file");

        assert_owner_only(&private);
    }
}
