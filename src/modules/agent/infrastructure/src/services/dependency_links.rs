// SPDX-License-Identifier: AGPL-3.0-or-later

//! The installed Node.js packages of the base checkout, shared into an
//! agent's own worktree (0021 D1 to D9).
//!
//! Each place gets a real `node_modules` folder with one link per package
//! entry of the base checkout's folder. A link in place of the whole folder would not
//! match the common ignore rule `node_modules/`, so git would list it as
//! untracked; a real folder is ignored by that rule and by `node_modules`.

use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};

use agent_application::common::interfaces::DEPENDENCY_LINKS_SUPPORTED;

use crate::services::git_runner::GitRunner;

const DEPENDENCY_FOLDER: &str = "node_modules";
const PACKAGE_FILES: &str = ":(glob)**/package.json";
const PACKAGE_FILE: &str = "package.json";
const OUTSIDE: &str = "it leads outside the base checkout or to nothing";
const HIDDEN_PREFIX: &str = ".";
/// The entries with a leading dot that package managers make; every other
/// one, such as the tool caches `.vite` and `.cache`, stays unlinked so no
/// two builds write one cache (spec Terms).
const PACKAGE_MANAGER_ENTRIES: [&str; 6] = [
    ".bin",
    ".pnpm",
    ".modules.yaml",
    ".package-lock.json",
    ".yarn-state.yml",
    ".bun",
];

/// Makes the links of every place that qualifies (D3, D4). Returns one
/// reason for each link refused because it leads outside the base checkout
/// and for each place where making the links failed; such a place is left
/// as it was (0021 FR-004, FR-007).
pub fn link_dependencies(runner: &GitRunner, base: &Path, worktree: &Path) -> Vec<String> {
    let mut reasons = Vec::new();
    if !DEPENDENCY_LINKS_SUPPORTED {
        return reasons;
    }
    let Ok(canonical_base) = base.canonicalize() else {
        return reasons;
    };
    for place in dependency_places(runner, worktree) {
        let pair = FolderPair::of(base, worktree, &place);
        if !pair.qualifies(runner, base) {
            continue;
        }
        if !is_inside(&pair.source, &canonical_base) {
            reasons.push(refusal(&pair.relative));
            continue;
        }
        if !pair.source.is_dir() {
            continue;
        }
        if let Err(error) = pair.link(runner, worktree, &canonical_base, &mut reasons) {
            reasons.push(format!(
                "the dependency links of {} were not made: {error}",
                pair.relative
            ));
        }
    }
    reasons
}

/// Removes every link that `link_dependencies` made, without following any
/// of them, and each folder it made that is empty afterwards (D7). Anything
/// else in such a folder is ignored content that the worktree removal
/// deletes.
pub fn unlink_dependencies(runner: &GitRunner, base: &Path, worktree: &Path) {
    for place in dependency_places(runner, worktree) {
        FolderPair::of(base, worktree, &place).remove_links();
    }
}

/// The top and the folder of each tracked `package.json`, `/`-separated and
/// empty for the top; never a folder inside a dependency folder (D2).
fn dependency_places(runner: &GitRunner, worktree: &Path) -> BTreeSet<String> {
    let listed = runner
        .read(worktree, &["ls-files", "-z", "--", PACKAGE_FILES])
        .unwrap_or_default();
    let mut places: BTreeSet<String> = listed
        .split('\0')
        .filter_map(|path| path.strip_suffix(PACKAGE_FILE))
        .map(|folder| folder.trim_end_matches('/').to_owned())
        .filter(|folder| !folder.split('/').any(|part| part == DEPENDENCY_FOLDER))
        .collect();
    places.insert(String::new());
    places
}

/// One place's dependency folder in the base checkout and in the worktree.
struct FolderPair {
    /// `<place>/node_modules`, as git names it relative to the top.
    relative: String,
    source: PathBuf,
    linked: PathBuf,
}

impl FolderPair {
    fn of(base: &Path, worktree: &Path, place: &str) -> Self {
        let relative = if place.is_empty() {
            DEPENDENCY_FOLDER.to_owned()
        } else {
            format!("{place}/{DEPENDENCY_FOLDER}")
        };
        Self {
            source: join_relative(base, &relative),
            linked: join_relative(worktree, &relative),
            relative,
        }
    }

    /// The worktree has nothing at the place, and the base checkout has
    /// something there that git ignores (D3).
    fn qualifies(&self, runner: &GitRunner, base: &Path) -> bool {
        let is_free = matches!(
            std::fs::symlink_metadata(&self.linked),
            Err(error) if error.kind() == io::ErrorKind::NotFound
        );
        is_free
            && std::fs::symlink_metadata(&self.source).is_ok()
            && is_ignored(runner, base, &self.relative)
    }

    /// Makes the folder and its links; on any failure removes what it made.
    fn link(
        &self,
        runner: &GitRunner,
        worktree: &Path,
        canonical_base: &Path,
        refusals: &mut Vec<String>,
    ) -> io::Result<()> {
        std::fs::create_dir(&self.linked)?;
        // The worktree may hold other ignore rules than the base checkout.
        if !is_ignored(runner, worktree, &self.relative) {
            return std::fs::remove_dir(&self.linked);
        }
        let made = self.link_entries(canonical_base, refusals);
        if made.is_err() {
            self.remove_links();
        }
        made
    }

    fn link_entries(&self, canonical_base: &Path, refusals: &mut Vec<String>) -> io::Result<()> {
        for entry in std::fs::read_dir(&self.source)? {
            let name = entry?.file_name();
            if !is_package_entry(&name.to_string_lossy()) {
                continue;
            }
            let target = self.source.join(&name);
            if is_inside(&target, canonical_base) {
                make_link(&target, &self.linked.join(&name))?;
            } else {
                refusals.push(refusal(&format!(
                    "{}/{}",
                    self.relative,
                    name.to_string_lossy()
                )));
            }
        }
        Ok(())
    }

    fn remove_links(&self) {
        if let Ok(entries) = std::fs::read_dir(&self.linked) {
            for entry in entries.flatten() {
                let path = entry.path();
                if self.is_office_link(&path) {
                    let _ = std::fs::remove_file(&path);
                }
            }
        }
        // Fails, as it should, when the folder still holds something.
        let _ = std::fs::remove_dir(&self.linked);
    }

    /// A link whose target is exactly the same entry in the base checkout
    /// (D6); a link the agent made, or a real file, is not one.
    fn is_office_link(&self, path: &Path) -> bool {
        let Some(name) = path.file_name() else {
            return false;
        };
        std::fs::read_link(path).is_ok_and(|target| target == self.source.join(name))
    }
}

/// A name without a leading dot, or one a package manager makes.
fn is_package_entry(name: &str) -> bool {
    !name.starts_with(HIDDEN_PREFIX) || PACKAGE_MANAGER_ENTRIES.contains(&name)
}

/// The log line of a link refused because it leads outside the base
/// checkout (D4).
fn refusal(relative: &str) -> String {
    format!("the dependency link {relative} was not made: {OUTSIDE}")
}

/// `folder` plus a `/`-separated relative path.
fn join_relative(folder: &Path, relative: &str) -> PathBuf {
    relative
        .split('/')
        .fold(folder.to_path_buf(), |joined, part| joined.join(part))
}

/// True when `path`, with every link followed, exists inside `canonical`.
fn is_inside(path: &Path, canonical: &Path) -> bool {
    path.canonicalize()
        .is_ok_and(|real| real.starts_with(canonical))
}

/// True when git, in `folder`, ignores `relative`. Any failure reads as not
/// ignored, so nothing is linked.
fn is_ignored(runner: &GitRunner, folder: &Path, relative: &str) -> bool {
    runner
        .read(folder, &["check-ignore", "--quiet", "--", relative])
        .is_ok()
}

#[cfg(unix)]
fn make_link(target: &Path, link: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(not(unix))]
fn make_link(_target: &Path, _link: &Path) -> io::Result<()> {
    Err(io::ErrorKind::Unsupported.into())
}
