use sha3::{Sha3_256, Digest};

use crate::utils::*;


/// Get SHA3 hash of a slice of U256.
pub fn hash_of_u256(elems: &[&U256]) -> U256 {
    let mut hasher = Sha3_256::new();
    for elem in elems {
        hasher.update(elem.to_bytes());
    }
    let bytes = hasher.finalize();
    U256::from_bytes(&bytes)
}


/// Get SHA3 hash of a buffer.
pub fn hash_of_buffer(buffer: &[u8]) -> U256 {
    let mut hasher = Sha3_256::new();
    hasher.update(buffer);
    let bytes = hasher.finalize();
    U256::from_bytes(&bytes)
}
