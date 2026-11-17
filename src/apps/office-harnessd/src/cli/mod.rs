// SPDX-License-Identifier: AGPL-3.0-or-later

//! The login-start subcommands `register`, `unregister`, and `status`
//! (plan D13). They run instead of the server and exit.

pub mod agent_call;
mod cli_error;
mod command;
mod login_item;
mod login_start;
mod run;
mod system_login_item;

pub use cli_error::CliError;
pub use command::{Command, split_command};
pub use login_item::{LoginItem, LoginItemError};
pub use login_start::LoginStart;
pub use run::run;
pub use system_login_item::SystemLoginItem;
