// SPDX-License-Identifier: AGPL-3.0-or-later

mod cli_adapters;
mod login_environment;
#[cfg(unix)]
mod login_shell;
mod program_lookup;
#[cfg(any(windows, test))]
mod registry_environment;

pub use cli_adapters::{AttributionTiming, CliAdapterRegistry};
pub use login_environment::{Environment, LoginEnvironment, ResolvedEnvironment};
pub use program_lookup::find_program;
