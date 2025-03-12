use std::iter;

use rand::Rng;
use sha3::{Sha3_256, Digest};

use crate::utils::*;
use crate::hash::*;
use crate::transaction::Transaction;


/// Mining complexity (number of leading zeros for empty block).
const COMPLEXITY: usize = 32;


/// Basic structure for block.
pub struct Block {
    pub ix: u64,
    pub size: u64,
    pub validator: U256,
    pub nonce: U256,
    pub hash: U256,
}


impl Block {
    /// New block.
    pub fn new(ix: u64, size: u64, validator: U256) -> Self {
        Self {
            ix, size, validator,
            nonce: U256::from(0),
            hash: U256::from(0),
        }
    }

    /// calculate block message as hash of the important content.
    pub fn calc_msg(block_hash_prev: &U256, validator: &U256, 
                    transactions: &[Transaction]) -> U256 {
        let mut elems = vec![block_hash_prev.clone(), validator.clone()];
        elems.extend(transactions.iter().map(|tr| tr.get_hash()));
        hash_of_u256(elems.iter())
    }

    /// Calculate block hash from message and nonce.
    pub fn calc_hash(msg: &U256, nonce: &U256) -> U256 {
        hash_of_u256([msg, nonce].into_iter())
    }

    /// Chech if the hash corresponds to the necessary size.
    pub fn is_hash_valid(hash_bytes: &[u8], size: usize) -> bool {
        // TODO: Implement the algorithm.
        true
    }

    /// Find correct nonce to mine the block.
    pub fn mine<R: Rng>(rng: &mut R, block_hash_prev: &U256, validator: &U256, 
                        transactions: &[Transaction]) -> U256 {
        // Calculate the message bytes
        let msg = Self::calc_msg(block_hash_prev, validator, transactions);

        // Number of transactions
        let size = transactions.len();

        // Initialize SHA3 hasher with the block message
        let mut hasher = Sha3_256::new();
        hasher.update(msg.to_bytes());

        // Mining loop
        loop {
            // Clone the hasher state before adding nonce
            let mut hasher_clone = hasher.clone();

            // Generate a random 256-bit nonce
            let nonce_bytes: [u8; 32] = rng.random();

            // Update the hasher with the generated nonce
            hasher.update(nonce_bytes);

            // Get the bytes of the final hash
            let hash_bytes = hasher_clone.finalize();

            // If the hash is valid return the generated nonce and U256
            if Self::is_hash_valid(&hash_bytes, size) {
                return U256::from_bytes(&nonce_bytes);
            }
        }
    }
}
