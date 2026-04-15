use crate::{
    storage::{ApprovalStatus, StoredApprovalRequest},
    tools::{ToolKind, ToolManifest},
};

use super::phases::invocation_key;

#[test]
fn payload_changes_tool_invocation_key() {
    let manifest = ToolManifest {
        name: "praxis-data-write".to_string(),
        description: "append notes".to_string(),
        kind: ToolKind::Shell,
        required_level: 2,
        requires_approval: true,
        rehearsal_required: true,
        allowed_paths: vec!["JOURNAL.md".to_string()],
        path: None,
        args: Vec::new(),
        timeout_secs: None,
        endpoint: None,
        method: None,
        headers: Vec::new(),
        body: None,
    };
    let first = StoredApprovalRequest {
        id: 1,
        tool_name: manifest.name.clone(),
        summary: "Update JOURNAL.md".to_string(),
        requested_by: "operator".to_string(),
        write_paths: vec!["JOURNAL.md".to_string()],
        payload_json: Some("{\"append_text\":\"first\"}".to_string()),
        status: ApprovalStatus::Approved,
        status_note: None,
        created_at: String::new(),
        updated_at: String::new(),
    };
    let mut second = first.clone();
    second.payload_json = Some("{\"append_text\":\"second\"}".to_string());

    assert_ne!(
        invocation_key(&manifest, &first),
        invocation_key(&manifest, &second)
    );
}
