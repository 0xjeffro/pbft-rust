use crate::consensus::message::{RequestMsg, PrePrepareMsg};
pub fn compute_digest(request_msg: &RequestMsg) -> String {
    use sha2::{Sha256, Digest};
    let serialized_request = serde_json::to_string(request_msg).unwrap();
    let mut hasher = Sha256::new();
    hasher.update(serialized_request);
    let result = hasher.finalize();
    hex::encode(result)
}