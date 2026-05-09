// src/routes/conversations/workspace_session.rs

#[derive(Debug, Clone, PartialEq)]
pub enum WorkspaceSessionRef {
    Artifact(i64),
    WorkItem(i64),
}

pub fn workspace_session_key(session: &WorkspaceSessionRef) -> String {
    match session {
        WorkspaceSessionRef::Artifact(id) => format!("artifact:{id}"),
        WorkspaceSessionRef::WorkItem(id) => format!("work-item:{id}"),
    }
}

pub fn upsert_workspace_session(
    sessions: &mut Vec<WorkspaceSessionRef>,
    session: WorkspaceSessionRef,
) -> String {
    let key = workspace_session_key(&session);
    if !sessions
        .iter()
        .any(|existing| workspace_session_key(existing) == key)
    {
        sessions.push(session);
    }
    key
}
