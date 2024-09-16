use std::sync::{Arc, Mutex};

#[derive(Clone, PartialEq, Debug)]
pub(crate) enum Stage {
    Idle,
    PrePrepare,
    Prepare,
    Commit,
}
#[derive(Clone)]
pub(crate) struct State {
    pub(crate) view_id: u32,
    pub(crate) current_stage: Arc<Mutex<Stage>>,
}