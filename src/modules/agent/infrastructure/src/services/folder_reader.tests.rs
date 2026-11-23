// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::{Duration, Instant};

use tempfile::TempDir;

use super::*;

fn relative(text: &str) -> RelativePath {
    RelativePath::parse("path", text).expect("a valid path")
}

/// A root folder `work` next to a folder `outside` that holds a file with
/// a recognizable content.
struct Fixture {
    _temp: TempDir,
    work: PathBuf,
    outside: PathBuf,
}

fn fixture() -> Fixture {
    let temp = TempDir::new().expect("temp folder");
    let work = temp.path().join("work");
    let outside = temp.path().join("outside");
    fs::create_dir_all(&work).expect("work");
    fs::create_dir_all(outside.join("inner")).expect("outside");
    fs::write(outside.join("key.txt"), "OUTSIDE-BYTES").expect("outside file");
    Fixture {
        _temp: temp,
        work,
        outside,
    }
}

fn names(listing: &FolderListing) -> Vec<&str> {
    listing
        .entries
        .iter()
        .map(|entry| entry.name.as_str())
        .collect()
}

fn text_of(content: &FileContent) -> String {
    String::from_utf8(content.bytes.clone().expect("text bytes")).expect("utf-8")
}

#[test]
fn a_listing_has_folders_first_then_names_by_code_point() {
    let fixture = fixture();
    fs::create_dir(fixture.work.join("zeta")).expect("folder");
    fs::create_dir(fixture.work.join("alpha")).expect("folder");
    fs::write(fixture.work.join("Beta.txt"), "b").expect("file");
    fs::write(fixture.work.join("apple.txt"), "a").expect("file");
    fs::write(fixture.work.join("é.txt"), "e").expect("file");

    let listing = list_folder(&fixture.work, &RelativePath::root()).expect("list");

    assert_eq!(
        names(&listing),
        ["alpha", "zeta", "Beta.txt", "apple.txt", "é.txt"]
    );
    assert_eq!(listing.more, 0);
    let file = &listing.entries[2];
    assert_eq!((file.kind, file.size), (EntryKind::File, Some(1)));
    assert_eq!(listing.entries[0].size, None);
}

#[test]
fn the_git_folder_is_never_listed_in_any_case() {
    let fixture = fixture();
    // In two folders, because a disk that ignores case cannot hold both.
    fs::create_dir(fixture.work.join(".git")).expect("folder");
    fs::create_dir_all(fixture.work.join("upper/.GIT")).expect("folder");
    fs::write(fixture.work.join(".gitignore"), "x").expect("file");

    let root = list_folder(&fixture.work, &RelativePath::root()).expect("list");
    let upper_path = RelativePath::parse("path", "upper").expect("path");
    let upper = list_folder(&fixture.work, &upper_path).expect("list");

    assert_eq!(names(&root), ["upper", ".gitignore"]);
    assert_eq!(names(&upper), Vec::<&str>::new());
}

#[test]
fn more_than_a_thousand_entries_are_counted_in_more() {
    let fixture = fixture();
    for index in 0..=MAX_LIST_ENTRIES {
        fs::write(fixture.work.join(format!("f{index:04}")), "").expect("file");
    }

    let listing = list_folder(&fixture.work, &RelativePath::root()).expect("list");

    assert_eq!(listing.entries.len(), MAX_LIST_ENTRIES);
    assert_eq!(listing.more, 1);
    assert_eq!(listing.entries[0].name, "f0000");
}

#[test]
fn secret_looking_names_are_flagged_in_a_listing() {
    let fixture = fixture();
    fs::write(fixture.work.join(".env"), "A=1").expect("file");
    fs::write(fixture.work.join("notes.txt"), "n").expect("file");

    let listing = list_folder(&fixture.work, &RelativePath::root()).expect("list");

    let flags: Vec<(&str, bool)> = listing
        .entries
        .iter()
        .map(|entry| (entry.name.as_str(), entry.secret))
        .collect();
    assert_eq!(flags, [(".env", true), ("notes.txt", false)]);
}

#[test]
fn a_missing_folder_and_a_file_are_told_apart() {
    let fixture = fixture();
    fs::write(fixture.work.join("a.txt"), "a").expect("file");

    let missing = list_folder(&fixture.work, &relative("nope")).expect_err("missing");
    let file = list_folder(&fixture.work, &relative("a.txt")).expect_err("a file");

    assert!(matches!(&missing, FileViewError::Failed(text) if text.contains("does not exist")));
    assert!(matches!(file, FileViewError::Invalid(_)));
}

#[test]
fn a_missing_root_is_a_failure_that_says_why() {
    let fixture = fixture();

    let error =
        list_folder(&fixture.work.join("gone"), &RelativePath::root()).expect_err("no root");

    assert!(matches!(&error, FileViewError::Failed(text) if text.contains("does not exist")));
}

#[test]
fn a_file_reads_as_text_with_its_size() {
    let fixture = fixture();
    fs::create_dir(fixture.work.join("sub")).expect("folder");
    fs::write(fixture.work.join("sub/a.txt"), "hello\n").expect("file");

    let content = read_file(&fixture.work, &relative("sub/a.txt"), false).expect("read");

    assert_eq!(content.state, ContentState::Text);
    assert_eq!(content.size, 6);
    assert_eq!(text_of(&content), "hello\n");
    assert_eq!(content.path, "sub/a.txt");
}

#[test]
fn an_empty_file_is_text_with_size_zero() {
    let fixture = fixture();
    fs::write(fixture.work.join("empty"), "").expect("file");

    let content = read_file(&fixture.work, &relative("empty"), false).expect("read");

    assert_eq!((content.state, content.size), (ContentState::Text, 0));
    assert_eq!(content.bytes, Some(Vec::new()));
}

#[test]
fn the_size_cap_is_exact() {
    let fixture = fixture();
    fs::write(fixture.work.join("at"), vec![b'a'; MAX_CONTENT_BYTES]).expect("file");
    fs::write(fixture.work.join("over"), vec![b'a'; MAX_CONTENT_BYTES + 1]).expect("file");

    let at = read_file(&fixture.work, &relative("at"), false).expect("read");
    let over = read_file(&fixture.work, &relative("over"), false).expect("read");

    assert_eq!(at.state, ContentState::Text);
    assert_eq!(at.bytes.map(|bytes| bytes.len()), Some(524_288));
    assert_eq!(
        (over.state, over.size, over.bytes),
        (ContentState::TooLarge, 524_289, None)
    );
}

#[test]
fn a_zero_byte_in_the_prefix_makes_a_file_binary() {
    let fixture = fixture();
    fs::write(fixture.work.join("blob"), [b'a', 0, b'b']).expect("file");

    let content = read_file(&fixture.work, &relative("blob"), false).expect("read");

    assert_eq!((content.state, content.bytes), (ContentState::Binary, None));
}

#[test]
fn a_secret_looking_file_gives_no_bytes_without_a_reveal() {
    let fixture = fixture();
    fs::write(fixture.work.join(".env"), "TOKEN=abc").expect("file");

    let hidden = read_file(&fixture.work, &relative(".env"), false).expect("read");
    let revealed = read_file(&fixture.work, &relative(".env"), true).expect("reveal");

    assert_eq!(
        (hidden.state, hidden.size, hidden.bytes),
        (ContentState::Secret, 9, None)
    );
    assert_eq!(text_of(&revealed), "TOKEN=abc");
}

#[cfg(unix)]
mod unix {
    use std::os::unix::fs::symlink;

    use nix::sys::stat::Mode;
    use nix::unistd::mkfifo;

    use super::*;

    #[test]
    fn a_path_that_leaves_the_root_is_refused_in_every_way() {
        let fixture = fixture();
        symlink(
            fixture.outside.join("key.txt"),
            fixture.work.join("file-link"),
        )
        .expect("link");
        symlink(&fixture.outside, fixture.work.join("folder-link")).expect("link");
        fs::create_dir(fixture.work.join("inside")).expect("folder");
        symlink("../../outside", fixture.work.join("inside/up")).expect("link");
        symlink("folder-link", fixture.work.join("chain-a")).expect("link");
        symlink("chain-a/key.txt", fixture.work.join("chain-b")).expect("link");
        let absolute = fixture.outside.join("key.txt");

        for path in [
            "file-link",
            "folder-link/key.txt",
            "folder-link/inner",
            "inside/up/key.txt",
            "chain-b",
            "folder-link/missing",
        ] {
            let read = read_file(&fixture.work, &relative(path), true);
            assert!(
                matches!(read, Err(FileViewError::Refused(_))),
                "{path}: {read:?}"
            );
        }
        for path in ["folder-link", "folder-link/inner", "inside/up"] {
            let listed = list_folder(&fixture.work, &relative(path));
            assert!(matches!(listed, Err(FileViewError::Refused(_))), "{path}");
        }
        for text in ["../outside/key.txt", "inside/../../outside/key.txt"] {
            assert!(RelativePath::parse("path", text).is_err(), "{text}");
        }
        assert!(RelativePath::parse("path", absolute.to_str().expect("text")).is_err());
    }

    #[test]
    fn a_link_that_stays_inside_is_followed_through_two_links() {
        let fixture = fixture();
        fs::create_dir_all(fixture.work.join("real/deep")).expect("folders");
        fs::write(fixture.work.join("real/deep/a.txt"), "inside").expect("file");
        symlink("real", fixture.work.join("alias")).expect("link");
        symlink("deep", fixture.work.join("real/shortcut")).expect("link");

        let content = read_file(&fixture.work, &relative("alias/shortcut/a.txt"), false)
            .expect("read through two links");

        assert_eq!(text_of(&content), "inside");
    }

    #[test]
    fn the_git_data_is_refused_even_through_a_link() {
        let fixture = fixture();
        fs::create_dir(fixture.work.join(".git")).expect("git folder");
        fs::write(fixture.work.join(".git/config"), "[remote]").expect("file");
        symlink(".git", fixture.work.join("data")).expect("link");

        for path in [".git/config", ".GIT/config", "data/config"] {
            let read = read_file(&fixture.work, &relative(path), true);
            assert!(
                matches!(read, Err(FileViewError::Refused(_))),
                "{path}: {read:?}"
            );
        }
        assert!(matches!(
            list_folder(&fixture.work, &relative("data")),
            Err(FileViewError::Refused(_))
        ));
    }

    #[test]
    fn link_targets_are_classified_from_the_root() {
        let fixture = fixture();
        fs::create_dir(fixture.work.join("folder")).expect("folder");
        fs::write(fixture.work.join("file"), "f").expect("file");
        symlink("folder", fixture.work.join("to-folder")).expect("link");
        symlink("file", fixture.work.join("to-file")).expect("link");
        symlink(&fixture.outside, fixture.work.join("to-outside")).expect("link");
        symlink("missing", fixture.work.join("to-missing")).expect("link");
        symlink("loop-b", fixture.work.join("loop-a")).expect("link");
        symlink("loop-a", fixture.work.join("loop-b")).expect("link");
        mkfifo(&fixture.work.join("pipe"), Mode::S_IRWXU).expect("fifo");
        symlink("pipe", fixture.work.join("to-pipe")).expect("link");

        let listing = list_folder(&fixture.work, &RelativePath::root()).expect("list");

        let target_of = |name: &str| {
            listing
                .entries
                .iter()
                .find(|entry| entry.name == name)
                .and_then(|entry| entry.target)
        };
        assert_eq!(target_of("to-folder"), Some(LinkTarget::Folder));
        assert_eq!(target_of("to-file"), Some(LinkTarget::File));
        assert_eq!(target_of("to-outside"), Some(LinkTarget::Outside));
        assert_eq!(target_of("to-missing"), Some(LinkTarget::Broken));
        assert_eq!(target_of("loop-a"), Some(LinkTarget::Broken));
        assert_eq!(target_of("to-pipe"), Some(LinkTarget::Other));
        assert_eq!(target_of("file"), None);
    }

    #[test]
    fn a_pipe_is_listed_as_other_and_refused_without_blocking() {
        let fixture = fixture();
        mkfifo(&fixture.work.join("pipe"), Mode::S_IRWXU).expect("fifo");
        let started = Instant::now();

        let listing = list_folder(&fixture.work, &RelativePath::root()).expect("list");
        let read = read_file(&fixture.work, &relative("pipe"), false);

        assert_eq!(listing.entries[0].kind, EntryKind::Other);
        assert!(matches!(read, Err(FileViewError::Invalid(_))), "{read:?}");
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn a_secret_looking_pipe_answers_secret_without_being_opened() {
        let fixture = fixture();
        mkfifo(&fixture.work.join(".env"), Mode::S_IRWXU).expect("fifo");

        let hidden = read_file(&fixture.work, &relative(".env"), false).expect("secret first");

        assert_eq!(hidden.state, ContentState::Secret);
    }

    #[test]
    fn a_plainly_named_link_to_a_secret_looking_file_is_secret() {
        let fixture = fixture();
        fs::write(fixture.work.join(".env"), "TOKEN=abc").expect("file");
        symlink(".env", fixture.work.join("notes")).expect("link");

        let hidden = read_file(&fixture.work, &relative("notes"), false).expect("read");
        let revealed = read_file(&fixture.work, &relative("notes"), true).expect("reveal");

        assert_eq!((hidden.state, hidden.bytes), (ContentState::Secret, None));
        assert_eq!(text_of(&revealed), "TOKEN=abc");
    }

    #[test]
    fn a_folder_is_not_a_regular_file() {
        let fixture = fixture();
        fs::create_dir(fixture.work.join("sub")).expect("folder");

        let read = read_file(&fixture.work, &relative("sub"), false);

        assert!(matches!(read, Err(FileViewError::Invalid(_))), "{read:?}");
    }
}
