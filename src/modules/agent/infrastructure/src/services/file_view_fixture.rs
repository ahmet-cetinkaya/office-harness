// SPDX-License-Identifier: AGPL-3.0-or-later

//! A real git repository in a temporary folder, for the tests of the git
//! reader and of the local file view.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

use tempfile::TempDir;

use crate::services::git_runner::{GitRunner, LazyGitRunner};
use crate::services::login_environment::Environment;
use crate::services::program_lookup::find_program;

/// A repository `work` on branch `main` with no commit, and a home folder
/// that holds no git configuration.
pub struct Repo {
    _temp: TempDir,
    pub temp_path: PathBuf,
    pub home: PathBuf,
    pub work: PathBuf,
}

impl Repo {
    pub fn new() -> Self {
        let temp = TempDir::new().expect("temp folder");
        let temp_path = temp.path().canonicalize().expect("canonical temp folder");
        let home = temp_path.join("home");
        let work = temp_path.join("work");
        fs::create_dir_all(&home).expect("home");
        fs::create_dir_all(&work).expect("work");
        let repo = Self {
            _temp: temp,
            temp_path,
            home,
            work,
        };
        repo.git(&["init", "--quiet", "--initial-branch=main"]);
        repo
    }

    pub fn path(&self, relative: &str) -> PathBuf {
        self.work.join(relative)
    }

    pub fn write(&self, relative: &str, content: impl AsRef<[u8]>) {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().expect("a parent")).expect("folders");
        fs::write(path, content).expect("file");
    }

    pub fn git(&self, arguments: &[&str]) -> String {
        self.git_input(arguments, b"")
    }

    /// Runs the real git in `work`, with `input` on its standard input.
    pub fn git_input(&self, arguments: &[&str], input: &[u8]) -> String {
        let mut child = Command::new("git")
            .args([
                "-c",
                "commit.gpgsign=false",
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "-C",
            ])
            .arg(&self.work)
            .args(arguments)
            .envs(test_environment(&self.home))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("git starts");
        child
            .stdin
            .take()
            .expect("stdin")
            .write_all(input)
            .expect("input");
        let output = child.wait_with_output().expect("git runs");
        assert!(
            output.status.success(),
            "git {arguments:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    pub fn commit_all(&self, message: &str) {
        self.git(&["add", "--all"]);
        self.git(&["commit", "--quiet", "--allow-empty", "-m", message]);
    }

    pub fn lazy_runner(&self) -> Arc<LazyGitRunner> {
        Arc::new(LazyGitRunner::fixed(Some(runner(&self.home))))
    }
}

pub fn test_environment(home: &Path) -> Environment {
    let mut environment: Environment = std::env::vars().collect();
    for name in ["HOME", "XDG_CONFIG_HOME", "USERPROFILE"] {
        environment.insert(name.to_owned(), home.to_string_lossy().into_owned());
    }
    environment
}

pub fn runner(home: &Path) -> GitRunner {
    let environment = test_environment(home);
    let program = find_program("git", &environment, home).expect("git is installed");
    GitRunner::new(program, &environment)
}
