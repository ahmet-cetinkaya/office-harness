// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn folder_grants_are_inherited_and_file_grants_are_not() {
    let folder = restrict_arguments("C:\\data", "S-1-5-21-1", FOLDER_INHERITANCE);
    let file = restrict_arguments("C:\\data\\token", "S-1-5-21-1", FILE_INHERITANCE);

    assert_eq!(
        folder,
        [
            "C:\\data",
            "/inheritance:r",
            "/grant:r",
            "*S-1-5-21-1:(OI)(CI)F",
            "/grant:r",
            "*S-1-5-18:(OI)(CI)F",
        ]
    );
    assert_eq!(file[3], "*S-1-5-21-1:F");
}

#[test]
fn reads_principals_from_icacls_output() {
    let output = "C:\\Users\\a b\\data NT AUTHORITY\\SYSTEM:(OI)(CI)(F)\n\
                  \x20                  HOST\\ana:(OI)(CI)(F)\n\
                  \n\
                  Successfully processed 1 files; Failed processing 0 files\n";

    assert_eq!(
        access_list_principals(output, "C:\\Users\\a b\\data"),
        ["NT AUTHORITY\\SYSTEM", "HOST\\ana"]
    );
}

#[cfg(windows)]
#[test]
fn restricted_folder_grants_only_the_current_user_and_system() {
    let folder = tempfile::tempdir().expect("temporary folder");
    let private = folder.path().join("private");
    fs::create_dir(&private).expect("create folder");

    restrict_folder(&private).expect("restrict folder");

    let path = private.to_string_lossy().into_owned();
    let listing = run_system_tool("icacls.exe", &[path.as_str()]).expect("read access list");
    let mut principals = access_list_principals(&listing, &path);
    principals.sort();
    assert_eq!(principals.len(), 2, "{listing}");
    assert!(
        principals.contains(&"NT AUTHORITY\\SYSTEM".to_owned()),
        "{listing}"
    );
    assert!(
        !listing.contains("(I)"),
        "inheritance must be broken: {listing}"
    );
}
