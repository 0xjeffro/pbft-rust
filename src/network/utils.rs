use crate::consensus::message::{RequestMsg, PrePrepareMsg};
pub fn compute_digest(request_msg: &RequestMsg) -> String {
    use sha2::{Sha256, Digest};
    let serialized_request = serde_json::to_string(request_msg).unwrap();
    let mut hasher = Sha256::new();
    hasher.update(serialized_request);
    let result = hasher.finalize();
    hex::encode(result)
}

pub fn verify_msg(msg: &PrePrepareMsg, req_view_id: u32, req_digest: String) -> bool {
    let correct_digest = req_digest == msg.digest;
    let correct_view_id = req_view_id == msg.view_id;
    //TODO: adopt upper/lower bound check.
    // Paper page 4, top right:
    // the sequence number in the pre-prepare message is between a low water mark and a high water mark
    correct_digest && correct_view_id
}

pub fn generate_fake_request_msg() -> RequestMsg {
    let mut request_msg = RequestMsg {
        operation: "fake".to_string(),
        time_stamp: 0,
        client_id: 0,
        sequence_id: 0,
        digest: "".to_string(),
    };
    request_msg.digest = compute_digest(&request_msg);
    request_msg
}