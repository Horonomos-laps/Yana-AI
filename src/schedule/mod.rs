//! Scheduled runs. This first slice is only the pure part: what a schedule
//! says and when it fires next. The job record, the tick loop and the
//! commands come in later slices (see `.planning/scheduler/PLAN.md`).

// Nothing calls this yet; the next slice (the job store and the commands) does.
#![allow(dead_code)]

mod spec;

#[allow(unused_imports)]
pub(crate) use spec::{parse, Schedule};
