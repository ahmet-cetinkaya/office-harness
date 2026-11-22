// SPDX-License-Identifier: AGPL-3.0-or-later

mod additions_view;
mod agent_action;
mod file_view;
mod session_command;
mod session_view;
mod start_terminal;

pub use additions_view::AdditionsView;
pub use agent_action::{AgentAction, AgentActionEntry};
pub use file_view::{
    ChangeKind, ChangeList, ChangedFile, CommitDetail, CommitPage, CommitSummary, ContentState,
    DiffState, EntryKind, FileContent, FileDiff, FolderEntry, FolderListing, LinkTarget,
};
pub use session_command::{SessionCommand, SessionPlan, TrackedSession, TrackingRequest};
pub use session_view::SessionView;
pub use start_terminal::{PrepareLaunch, PreparedLaunch, StartTerminal};
