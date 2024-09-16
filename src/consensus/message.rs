use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RequestMsg { //<REQUEST, o, t, c>
    pub(crate) operation: String,  // 'o', the operation to be executed
    pub(crate) time_stamp: u64, // 't', the time stamp
    pub(crate) client_id: u32, // 'c', the client id

    // When a primary node receives a request message from a client,
    // it assigns a sequence number to the request message
    // and multicasts a pre-prepare message for that sequence number.
    pub(crate) sequence_id: u32, // 'n', the sequence number
    #[serde(skip)]
    pub(crate) digest: String, // compute&save digest when receiving the request message for performance consideration
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PrePrepareMsg { //< <PRE-PREPARE, v, n, d>, m >
    pub(crate) view_id: u32, // 'v', the view number
    pub(crate) sequence_id: u32, // 'n', the sequence number
    pub(crate) digest: String, // 'd', the digest of the request message
    pub(crate) request_msg: RequestMsg, // 'm', the request message
}

struct ReplyMsg {
    time_stamp: u64,
    view_id: u32,
    node_id: u32, // 'r', the node(replica) id
    client_id: u32,
    result: String,
}



#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MsgType {
    PrepareMsg,
    CommitMsg,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VoteMsg {
    pub(crate) view_id: u32,
    pub(crate) sequence_id: u32,
    pub(crate) digest: String,
    pub(crate) node_id: u32,
    pub(crate) msg_type: MsgType,
}