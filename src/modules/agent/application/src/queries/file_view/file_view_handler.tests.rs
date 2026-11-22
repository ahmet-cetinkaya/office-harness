// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::Path;
use std::sync::Mutex;

use agent_domain::enums::AgentStatus;
use kernel::Timestamp;

use super::*;
use crate::common::interfaces::FileViewError;
use crate::test_support::{Office, agent_in_project, project_named};

const FULL_COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

/// Records the root of every call and answers with empty results.
#[derive(Default)]
struct RecordingView {
    roots: Mutex<Vec<PathBuf>>,
}

impl RecordingView {
    fn record(&self, root: &Path) {
        self.roots.lock().expect("lock").push(root.to_path_buf());
    }

    fn roots(&self) -> Vec<PathBuf> {
        self.roots.lock().expect("lock").clone()
    }
}

impl FileView for RecordingView {
    fn list(&self, root: &Path, path: &RelativePath) -> Result<FolderListing, FileViewError> {
        self.record(root);
        Ok(FolderListing {
            root: root.display().to_string(),
            path: path.to_text(),
            entries: Vec::new(),
            more: 0,
            read_at: Timestamp::from_unix_millis(0),
        })
    }

    fn read(&self, _: &Path, _: &RelativePath, _: bool) -> Result<FileContent, FileViewError> {
        Err(FileViewError::Failed("unused".to_owned()))
    }

    fn changes(&self, _: &Path) -> Result<ChangeList, FileViewError> {
        Err(FileViewError::Failed("unused".to_owned()))
    }

    fn diff(&self, _: &Path, _: &DiffRequest) -> Result<FileDiff, FileViewError> {
        Err(FileViewError::Failed("unused".to_owned()))
    }

    fn history(&self, root: &Path, _: u32) -> Result<CommitPage, FileViewError> {
        self.record(root);
        Ok(CommitPage {
            repository: true,
            commits: Vec::new(),
            more: false,
        })
    }

    fn commit(&self, _: &Path, _: &str) -> Result<CommitDetail, FileViewError> {
        Err(FileViewError::Failed("unused".to_owned()))
    }
}

fn handler(office: &Office) -> (FileViewHandler, Arc<RecordingView>) {
    let view = Arc::new(RecordingView::default());
    let handler =
        FileViewHandler::new(office.agents.clone(), office.projects.clone(), view.clone());
    (handler, view)
}

#[test]
fn an_agent_with_a_working_folder_is_viewed_at_that_folder() {
    let office = Office::new();
    let folder = Path::new("/work/here");
    let agent = office.add_agent("Ana", "claude", folder, AgentStatus::Stopped);
    let (handler, view) = handler(&office);

    handler.list(agent.id, "").expect("list");

    assert_eq!(view.roots(), [folder.to_path_buf()]);
}

#[test]
fn an_agent_without_a_working_folder_is_viewed_at_its_project_folder() {
    let office = Office::new();
    let project = project_named("Site");
    office.projects.insert(&project).expect("project");
    let agent = agent_in_project("Ana", project.id);
    office.agents.insert(&agent).expect("agent");
    let (handler, view) = handler(&office);

    handler.history(agent.id, 0).expect("history");

    assert_eq!(view.roots(), [project.folder]);
}

#[test]
fn an_unknown_agent_is_not_found() {
    let office = Office::new();
    let (handler, view) = handler(&office);

    let error = handler.list(AgentId::generate(), "").expect_err("unknown");

    assert!(matches!(error, ApplicationError::NotFound(_)), "{error:?}");
    assert_eq!(view.roots(), Vec::<PathBuf>::new());
}

#[test]
fn a_malformed_input_is_refused_before_the_view_is_asked() {
    let office = Office::new();
    let agent = office.add_agent("Ana", "claude", Path::new("/w"), AgentStatus::Stopped);
    let (handler, view) = handler(&office);
    let question = |commit| DiffQuestion {
        path: "a.txt",
        old_path: None,
        commit,
        reveal: false,
    };

    let errors = [
        handler.list(agent.id, "../x").expect_err("parent"),
        handler.read(agent.id, "", false).expect_err("root file"),
        handler
            .diff(
                agent.id,
                &DiffQuestion {
                    old_path: Some("/abs"),
                    ..question(None)
                },
            )
            .expect_err("old path"),
        handler
            .diff(agent.id, &question(Some("HEAD")))
            .expect_err("revision"),
        handler
            .diff(agent.id, &question(Some("--output=x")))
            .expect_err("option"),
        handler
            .diff(agent.id, &question(Some(&FULL_COMMIT.to_uppercase())))
            .expect_err("upper case"),
        handler.commit(agent.id, "abc1234").expect_err("short id"),
        handler.history(agent.id, 1_000_001).expect_err("skip"),
    ];

    for error in errors {
        assert!(matches!(error, ApplicationError::Invalid(_)), "{error:?}");
    }
    assert_eq!(view.roots(), Vec::<PathBuf>::new());
}

#[test]
fn a_view_refusal_keeps_its_code() {
    let error: ApplicationError = FileViewError::Refused("outside".to_owned()).into();
    assert_eq!(error, ApplicationError::Refused("outside".to_owned()));
}
