use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::consensus;
use crate::consensus::message::RequestMsg;

#[derive(Clone)]
pub(crate) struct Node {
    pub(crate) id: u32,
    pub(crate) is_faulty: bool, // whether this node is faulty
    pub(crate) node_table: HashMap<u32, String>,  // Node.id -> url:port
    pub(crate) view: View,
    pub(crate) current_state: consensus::pbft::State, // current state of the node
    committed_msgs: Vec<RequestMsg>,
    pub(crate) msg_buffer : MsgBuffer,
}

#[derive(Clone)]
pub(crate) struct View {
    pub(crate) id: u32,
    pub(crate) primary_node_id: u32,
}

#[derive(Clone)]
pub(crate) struct MsgBuffer {
    pub(crate) request_msgs: Arc<Mutex<Vec<RequestMsg>>>,
    pub(crate) preprepare_msgs: Arc<Mutex<Vec<consensus::message::PrePrepareMsg>>>,
    pub(crate) prepare_msgs: Arc<Mutex<Vec<consensus::message::VoteMsg>>>,
    pub(crate) commit_msgs: Arc<Mutex<Vec<consensus::message::VoteMsg>>>,
}

impl Node {
    pub fn new(id: u32, n: u32, is_faulty: bool) -> Node {
        let mut node_table = HashMap::new();
        let mut base_port = 8000;
        for i in 0..n {
            node_table.insert(i, "localhost:".to_string() + &(base_port + i).to_string());
        }
        let view = View {
            id: 9999, // initial view id
            primary_node_id: 0, // static primary node with node_id = 0
        };

        let msg_buffer = MsgBuffer {
            request_msgs: Arc::new(Mutex::new(Vec::new())),
            preprepare_msgs: Arc::new(Mutex::new(Vec::new())),
            prepare_msgs: Arc::new(Mutex::new(Vec::new())),
            commit_msgs: Arc::new(Mutex::new(Vec::new())),
        };

        let current_state = consensus::pbft::State {
            view_id: 0,
            current_stage: Arc::new(Mutex::new(consensus::pbft::Stage::Idle)),
        };

        Self {
            id,
            is_faulty,
            node_table,
            view,
            current_state,
            committed_msgs: vec![],
            msg_buffer,
        }
    }
}