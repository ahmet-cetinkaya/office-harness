// SPDX-License-Identifier: AGPL-3.0-or-later

mod bounded_child;
mod cli_adapters;
mod dependency_links;
#[cfg(test)]
mod file_view_fixture;
mod folder_reader;
mod git_parsing;
mod git_reader;
mod git_runner;
mod git_worktrees;
mod local_file_view;
mod login_environment;
#[cfg(unix)]
mod login_shell;
mod program_lookup;
#[cfg(any(windows, test))]
mod registry_environment;
mod status_command;

pub use cli_adapters::{AttributionTiming, CliAdapterRegistry};
pub use git_runner::{GitRunner, LazyGitRunner};
pub use git_worktrees::GitWorktrees;
pub use local_file_view::LocalFileView;
pub use login_environment::{Environment, LoginEnvironment, ResolvedEnvironment};
pub use program_lookup::find_program;
pub use status_command::run_status_command;
