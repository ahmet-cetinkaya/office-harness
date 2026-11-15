// SPDX-License-Identifier: AGPL-3.0-or-later

mod replace {
    use std::fs;

    use crate::replace_private_file;

    #[test]
    fn replaces_the_contents_and_leaves_no_sibling_behind() {
        let root = tempfile::tempdir().expect("temporary folder");
        let path = root.path().join("token");
        fs::write(&path, "old").expect("create file");

        replace_private_file(&path, b"new").expect("replace");

        assert_eq!(fs::read_to_string(&path).expect("read"), "new");
        assert_eq!(fs::read_dir(root.path()).expect("list").count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn the_new_file_is_owner_only_even_over_a_wider_one() {
        use std::os::unix::fs::PermissionsExt;

        let root = tempfile::tempdir().expect("temporary folder");
        let path = root.path().join("token");
        fs::write(&path, "old").expect("create file");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).expect("widen");

        replace_private_file(&path, b"new").expect("replace");

        let mode = fs::metadata(&path).expect("metadata").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}

#[cfg(unix)]
mod unix {
    use std::fs::{self, OpenOptions};
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    use crate::{create_private_dir_all, open_private_file};

    fn mode_of(path: &Path) -> u32 {
        fs::metadata(path).expect("metadata").permissions().mode() & 0o777
    }

    #[test]
    fn creates_folders_with_owner_only_mode_including_parents() {
        let root = tempfile::tempdir().expect("temporary folder");
        let leaf = root.path().join("parent").join("leaf");

        create_private_dir_all(&leaf).expect("create folders");

        assert_eq!(mode_of(&leaf), 0o700);
        assert_eq!(mode_of(leaf.parent().expect("parent")), 0o700);
    }

    #[test]
    fn narrows_a_folder_that_already_exists_with_wider_access() {
        let root = tempfile::tempdir().expect("temporary folder");
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o755)).expect("widen");

        create_private_dir_all(root.path()).expect("create folders");

        assert_eq!(mode_of(root.path()), 0o700);
    }

    #[test]
    fn creates_files_with_owner_only_mode() {
        let root = tempfile::tempdir().expect("temporary folder");
        let path = root.path().join("token");

        open_private_file(&path, OpenOptions::new().write(true)).expect("create file");

        assert_eq!(mode_of(&path), 0o600);
    }

    #[test]
    fn narrows_a_file_that_already_exists_with_wider_access() {
        let root = tempfile::tempdir().expect("temporary folder");
        let path = root.path().join("server.log");
        fs::write(&path, "line").expect("create file");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).expect("widen");

        open_private_file(&path, OpenOptions::new().append(true)).expect("open file");

        assert_eq!(mode_of(&path), 0o600);
    }
}
