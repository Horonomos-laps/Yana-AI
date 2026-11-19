//! Canonical Yana turn runtime.
//!
//! Every client eventually submits turns here. Providers supply inference,
//! Yana's control plane authorizes capabilities, and Giám Thị remains the
//! fail-closed root authority above both.

mod away;
mod away_cmd;
mod away_parked;
mod authority;
mod check;
mod controller;
mod events;
mod human_grant;
#[allow(dead_code)]
mod level;
mod level_cmd;
#[allow(dead_code)]
mod level_rules;
#[allow(dead_code)]
mod level_shadow;
#[allow(dead_code)]
mod level_store;
mod origin;
mod outcome;
mod pending_approval;
mod receipt;
mod request;
mod verify;

pub(crate) use authority::{
    AuthorityDecision, AuthorityLayer, IntentDeclaration, RuntimeAuthority, YanaAuthorityChain,
};
pub(crate) use controller::{
    execute_approved_tool, push_tool_result, ApprovedTool, CancellationToken, ToolExecutor,
    TurnEngine, TurnError,
};
pub(crate) use check::cmd_authority_check;
pub(crate) use events::RuntimeEvent;
pub(crate) use human_grant::{grant_for_decision, project_code_warning, ApprovalScope, GrantOutcome, GrantRequest};
pub(crate) use away_cmd::{
    cmd_away_report, cmd_away_start, cmd_away_status, cmd_away_stop, DEFAULT_AWAY_BUDGET, DEFAULT_AWAY_MINUTES,
};
pub(crate) use level_cmd::{cmd_level_get, cmd_level_set};
pub(crate) use origin::{TurnContext, TurnOrigin};
pub(crate) use outcome::TurnOutcome;
pub(crate) use pending_approval::{
    cmd_pending_approvals, disclosure_summary, reason_with_disclosure, resume_turn, PendingApprovalStore,
};
pub(crate) use receipt::cmd_authority_executions;
pub(crate) use receipt::cmd_authority_receipts;
pub(crate) use request::TurnRequest;
pub(crate) use verify::cmd_authority_verify;

#[cfg(test)]
mod tests;
