// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use crate::services::file_view_fixture::Repo;

fn view(repo: &Repo) -> LocalFileView {
    LocalFileView::new(repo.lazy_runner())
}

fn path(text: &str) -> RelativePath {
    RelativePath::parse("path", text).expect("path")
}

fn request(path_text: &str, old_path: Option<&str>, commit: Option<&str>) -> DiffRequest {
    DiffRequest {
        path: path(path_text),
        old_path: old_path.map(path),
        commit: commit.map(str::to_owned),
        reveal: false,
    }
}

fn revealed(mut request: DiffRequest) -> DiffRequest {
    request.reveal = true;
    request
}

fn head_commit(repo: &Repo) -> String {
    repo.git(&["rev-parse", "HEAD"]).trim().to_owned()
}

/// A repository whose first commit holds `.env` and `plain.txt`.
fn repo_with_secret_and_plain_file() -> Repo {
    let repo = Repo::new();
    repo.write(".env", "TOKEN=abc\nSECOND=2\nTHIRD=3\nFOURTH=4\n");
    repo.write("plain.txt", "line one\nline two\nline three\nline four\n");
    repo.commit_all("first");
    repo
}

#[test]
fn a_staged_rename_of_a_secret_name_to_a_plain_name_is_secret_until_revealed() {
    let repo = repo_with_secret_and_plain_file();
    repo.git(&["mv", ".env", "notes.txt"]);
    let rename = request("notes.txt", Some(".env"), None);

    let hidden = view(&repo).diff(&repo.work, &rename).expect("diff");
    let shown = view(&repo)
        .diff(&repo.work, &revealed(rename))
        .expect("revealed diff");
    let changes = view(&repo).changes(&repo.work).expect("changes");

    assert_eq!(
        (hidden.state, hidden.bytes, hidden.size),
        (DiffState::Secret, None, None)
    );
    assert_ne!(shown.state, DiffState::Secret);
    let file = &changes.files[0];
    assert_eq!(file.path, "notes.txt");
    assert!(file.secret, "the old name is secret-looking");
}

#[test]
fn a_staged_rename_of_a_plain_name_to_a_secret_name_is_secret_until_revealed() {
    let repo = repo_with_secret_and_plain_file();
    repo.git(&["mv", "plain.txt", "prod.pem"]);
    let rename = request("prod.pem", Some("plain.txt"), None);

    let hidden = view(&repo).diff(&repo.work, &rename).expect("diff");
    let shown = view(&repo)
        .diff(&repo.work, &revealed(rename))
        .expect("revealed diff");

    assert_eq!(hidden.state, DiffState::Secret);
    assert_ne!(shown.state, DiffState::Secret);
}

#[test]
fn the_same_rename_in_a_commit_is_secret_until_revealed() {
    let repo = repo_with_secret_and_plain_file();
    repo.git(&["mv", ".env", "notes.txt"]);
    repo.commit_all("rename");
    let id = head_commit(&repo);
    let rename = request("notes.txt", Some(".env"), Some(&id));

    let hidden = view(&repo).diff(&repo.work, &rename).expect("diff");
    let shown = view(&repo)
        .diff(&repo.work, &revealed(rename))
        .expect("revealed diff");
    let detail = view(&repo).commit(&repo.work, &id).expect("commit");

    assert_eq!((hidden.state, hidden.bytes), (DiffState::Secret, None));
    assert_ne!(shown.state, DiffState::Secret);
    assert!(detail.files[0].secret);
    assert_eq!(detail.files[0].old_path.as_deref(), Some(".env"));
}

#[test]
fn a_secret_untracked_file_is_not_read_without_a_reveal() {
    let repo = repo_with_secret_and_plain_file();
    repo.write("local.key", "-----BEGIN KEY-----\n");

    let hidden = view(&repo)
        .diff(&repo.work, &request("local.key", None, None))
        .expect("diff");
    let shown = view(&repo)
        .diff(&repo.work, &revealed(request("local.key", None, None)))
        .expect("revealed diff");

    assert_eq!((hidden.state, hidden.bytes), (DiffState::Secret, None));
    assert_eq!(shown.state, DiffState::Diff);
}

/// Links, named pipes and shell scripts as hook programs exist on Unix only.
#[cfg(unix)]
mod unix {
    use std::collections::BTreeMap;
    use std::fs;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::path::{Path, PathBuf};
    use std::time::SystemTime;

    use super::*;
    #[test]
    fn the_old_path_of_a_rename_goes_through_the_same_parse_and_containment() {
        let repo = repo_with_secret_and_plain_file();
        let outside = repo.temp_path.join("outside");
        fs::create_dir_all(&outside).expect("outside folder");
        symlink(&outside, repo.path("link-out")).expect("link");
        let revealed_request = |path_text, old_path| revealed(request(path_text, old_path, None));

        for (path_text, old_path) in [
            ("plain.txt", Some("link-out/old.txt")),
            ("link-out/new.txt", Some("plain.txt")),
            ("plain.txt", Some(".git/config")),
            (".git/config", None),
        ] {
            let diff = view(&repo).diff(&repo.work, &revealed_request(path_text, old_path));
            assert!(
                matches!(diff, Err(FileViewError::Refused(_))),
                "{path_text} {old_path:?}: {diff:?}"
            );
        }
        let bad_old_path = RelativePath::parse("old_path", "../outside/old.txt");
        assert!(bad_old_path.is_err());
    }

    #[test]
    fn a_commit_path_is_a_name_of_the_tree_not_a_path_of_the_disk() {
        let repo = repo_with_secret_and_plain_file();
        let outside = repo.temp_path.join("outside");
        fs::create_dir_all(&outside).expect("outside folder");
        symlink(&outside, repo.path("data")).expect("link");
        let id = head_commit(&repo);

        let diff = view(&repo)
            .diff(&repo.work, &request("data/none.txt", None, Some(&id)))
            .expect("a path of the tree, not read from the disk");

        assert_eq!(diff.state, DiffState::None);
    }

    /// A repository whose configuration names a program for every way git can
    /// run one while it only reads, each program writing its marker file.
    struct Markers {
        folder: PathBuf,
    }

    impl Markers {
        fn script(repo: &Repo, name: &str, body: &str) -> String {
            let folder = repo.temp_path.join("scripts");
            fs::create_dir_all(&folder).expect("scripts folder");
            let program = folder.join(name);
            let markers = repo.temp_path.join("markers");
            fs::write(
                &program,
                format!("#!/bin/sh\ntouch '{}/{name}'\n{body}\n", markers.display()),
            )
            .expect("script");
            fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).expect("executable");
            program.to_string_lossy().into_owned()
        }

        fn install(repo: &Repo) -> Self {
            let folder = repo.temp_path.join("markers");
            fs::create_dir_all(&folder).expect("markers folder");
            let configure = |key: &str, value: &str| repo.git(&["config", key, value]);
            configure("diff.external", &Self::script(repo, "external", "exit 0"));
            configure(
                "diff.conv.textconv",
                &Self::script(repo, "textconv", "cat \"$1\""),
            );
            configure("filter.mark.clean", &Self::script(repo, "clean", "cat"));
            configure("filter.mark.smudge", &Self::script(repo, "smudge", "cat"));
            configure("filter.mark.required", "true");
            configure("core.fsmonitor", &Self::script(repo, "fsmonitor", "exit 0"));
            configure("gpg.program", &Self::script(repo, "signature", "exit 1"));
            configure("log.showSignature", "true");
            for hook in [
                "post-index-change",
                "reference-transaction",
                "post-checkout",
                "pre-auto-gc",
            ] {
                let program = Self::script(repo, &format!("hook-{hook}"), "exit 0");
                fs::copy(&program, repo.path(&format!(".git/hooks/{hook}"))).expect("hook");
            }
            Self { folder }
        }

        fn found(&self) -> Vec<String> {
            let mut found: Vec<String> = fs::read_dir(&self.folder)
                .expect("markers folder")
                .map(|entry| {
                    entry
                        .expect("entry")
                        .file_name()
                        .to_string_lossy()
                        .into_owned()
                })
                .collect();
            found.sort();
            found
        }

        fn clear(&self) {
            for name in self.found() {
                fs::remove_file(self.folder.join(name)).expect("marker removed");
            }
        }
    }

    /// Every path below `folder` with its bytes and modification time.
    fn snapshot(folder: &Path) -> BTreeMap<PathBuf, (Option<Vec<u8>>, SystemTime)> {
        let mut entries = BTreeMap::new();
        let mut pending = vec![folder.to_path_buf()];
        while let Some(current) = pending.pop() {
            for entry in fs::read_dir(&current).expect("folder") {
                let path = entry.expect("entry").path();
                let metadata = fs::symlink_metadata(&path).expect("metadata");
                let bytes = metadata.is_file().then(|| fs::read(&path).expect("bytes"));
                if metadata.is_dir() {
                    pending.push(path.clone());
                }
                entries.insert(path, (bytes, metadata.modified().expect("time")));
            }
        }
        entries
    }

    fn marker_repository() -> (Repo, Markers) {
        let repo = Repo::new();
        repo.write("a.txt", "one\n");
        repo.write("conv.dat", "data one\n");
        repo.write("filtered.cfg", "setting one\n");
        repo.commit_all("first");
        repo.write("a.txt", "mid\n");
        repo.commit_all("second");
        repo.write(".gitattributes", "*.dat diff=conv\n*.cfg filter=mark\n");
        let markers = Markers::install(&repo);
        repo.write("a.txt", "two\n");
        repo.write("conv.dat", "data two\n");
        repo.write("filtered.cfg", "setting two, longer than before\n");
        repo.write("untracked.txt", "new\n");
        (repo, markers)
    }

    /// A commit with a signature header, which makes `log --show-signature`
    /// start the configured signature program. It holds the tree of HEAD on the
    /// parent of HEAD, so it changes `a.txt` and the history lists it.
    fn add_signed_commit(repo: &Repo) -> String {
        let tree = repo.git(&["rev-parse", "HEAD^{tree}"]).trim().to_owned();
        let parent = repo.git(&["rev-parse", "HEAD~1"]).trim().to_owned();
        let commit = format!(
            "tree {tree}\nparent {parent}\nauthor T <t@e.com> 1700000000 +0000\ncommitter T <t@e.com> 1700000000 +0000\ngpgsig -----BEGIN PGP SIGNATURE-----\n \n fake\n -----END PGP SIGNATURE-----\n\nsigned commit\n"
        );
        let id = repo
            .git_input(
                &["hash-object", "-t", "commit", "-w", "--stdin"],
                commit.as_bytes(),
            )
            .trim()
            .to_owned();
        repo.git(&["update-ref", "refs/heads/main", &id]);
        id
    }

    #[test]
    fn the_marker_programs_do_run_when_git_is_not_hardened() {
        let (repo, markers) = marker_repository();
        add_signed_commit(&repo);

        repo.git(&["diff"]);
        repo.git(&["diff", "--no-ext-diff"]);
        repo.git(&["status"]);
        repo.git(&["log", "-1"]);

        assert_eq!(
            markers.found(),
            [
                "clean",
                "external",
                "fsmonitor",
                "hook-post-index-change",
                "hook-reference-transaction",
                "signature",
                "smudge",
                "textconv"
            ]
        );
    }

    #[test]
    fn a_whole_session_runs_no_program_and_writes_nothing() {
        let (repo, markers) = marker_repository();
        let signed = add_signed_commit(&repo);
        repo.git(&["diff"]);
        repo.git(&["diff", "--no-ext-diff"]);
        repo.git(&["status"]);
        markers.clear();
        let before = snapshot(&repo.work);
        let view = view(&repo);

        view.list(&repo.work, &RelativePath::root()).expect("list");
        view.read(&repo.work, &path("a.txt"), false).expect("read");
        let changes = view.changes(&repo.work).expect("changes");
        for file in &changes.files {
            let diff = view
                .diff(&repo.work, &request(&file.path, None, None))
                .expect("diff");
            assert_ne!(diff.state, DiffState::None, "{}", file.path);
        }
        let history = view.history(&repo.work, 0).expect("history");
        view.commit(&repo.work, &signed).expect("commit");
        view.diff(&repo.work, &request("a.txt", None, Some(&signed)))
            .expect("commit diff");

        assert_eq!(changes.files.len(), 5, "{:?}", changes.files);
        assert_eq!(history.commits[0].subject, "signed commit");
        assert_eq!(markers.found(), Vec::<String>::new(), "a program ran");
        assert_eq!(snapshot(&repo.work), before, "something was written");
    }
}
