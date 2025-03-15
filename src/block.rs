use std::collections::HashSet;

use rand::Rng;
use sha3::{Sha3_256, Digest};

use crate::utils::*;
use crate::transaction::{Type, Transaction, group_transactions};
use crate::crypto::Schema;
use crate::coin::coin_is_valid;
use crate::state::{CoinOwnerMap, CoinInfoMap};


/// Basic structure for block.
#[derive(Clone)]
pub struct Block {
    pub tix: u64,
    pub size: u64,
    pub hash_prev: U256,
    pub validator: U256,
    pub nonce: U256,
    pub hash: U256,
}


impl Block {
    /// New block.
    pub fn new(tix: u64, size: u64, hash_prev: U256, validator: U256,
               nonce: U256, hash: U256) -> Self {
        Self { tix, size, hash_prev, validator, nonce, hash }
    }

    /// Build a new block for the transactions. It validates the final hash.
    pub fn build(tix: u64, hash_prev: U256, validator: U256, 
                 transactions: &[Transaction], nonce: U256,
                 complexity: usize, schema: &Schema,
                 coin_owner_map: &CoinOwnerMap, 
                 coin_info_map: &CoinInfoMap,
                 block_hash_prev: &U256) -> Option<Self> {
        // Validate transactions
        if Self::validate_transactions(transactions, &validator, schema, 
                                       coin_owner_map, coin_info_map,
                                       block_hash_prev) {
            // Calculate the message
            let msg = Self::calc_msg(&hash_prev, &validator, transactions);

            // Calculate the hash
            let hash = Self::calc_hash(&msg, &nonce);

            // Validate hash
            if Self::validate_hash(&hash, transactions.len(), complexity) {
                Some(Self::new(tix, transactions.len() as u64, hash_prev,
                               validator, nonce, hash))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Validate coins.
    pub fn validate_coins(transactions: &[Transaction], schema: &Schema, 
                          coin_owner_map: &CoinOwnerMap,
                          block_hash_prev: &U256) -> bool {
        // Repeated coins are not valid
        let mut coin_set = HashSet::new();

        // Loop for transactions
        for transaction in transactions.iter() {
            // Get coin and sender from transaction
            let coin = &transaction.coin;
            let sender = &transaction.get_sender(schema);

            // Check same coin
            if coin_set.contains(coin) {
                return false;
            }
            coin_set.insert(coin);

            // Try to find the coin in coin-owner map
            if let Some(owner) = coin_owner_map.get(coin) {
                // Check ownership
                if owner != sender {
                    return false;
                }
            } else {
                // Check mining
                if !coin_is_valid(coin, block_hash_prev, sender) {
                    return false;
                }
            }
        }

        true
    }

    /// Validate transactions.
    pub fn validate_transactions(transactions: &[Transaction], 
                                 validator: &U256, schema: &Schema, 
                                 coin_owner_map: &CoinOwnerMap,
                                 coin_info_map: &CoinInfoMap,
                                 block_hash_prev: &U256) -> bool {
        // Check coins
        if !Self::validate_coins(transactions, schema, coin_owner_map, 
                                 block_hash_prev) {
            return false;
        }

        // Set a countdown for groupped transactions
        let mut countdown = transactions.len();

        // Loop for groups and extensions
        for (group, ext) in group_transactions(transactions.to_vec(), schema, 
                                               coin_info_map) {
            // Check validator
            if let Some(ext_sender) = ext.get_sender(schema) {
                if &ext_sender != validator {
                    return false;
                }
            }

            // Check value
            if ext.get_type() != Type::Transfer {
                if group.get_order(coin_info_map) != 
                   ext.get_order(coin_info_map) {
                    return false;
                }
            }

            // Decrement the countdown
            countdown -= group.len() + ext.len();
        }

        // Return `true` if all transactions have been groupped else `false`
        countdown == 0
    }

    /// Validate hash for the certain complexity.
    pub fn validate_hash(hash: &U256, size: usize, complexity: usize) -> bool {
        let limit_hash = Self::calc_limit_hash(size, complexity);
        Self::is_hash_valid(&hash.to_bytes(), &limit_hash)
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
    pub fn is_hash_valid(hash_bytes: &[u8], limit_hash_bytes: &[u8]) -> bool {
        hash_bytes <= limit_hash_bytes
    }

    /// Find correct nonce bytes to mine the block.
    pub fn mine<R: Rng>(rng: &mut R, block_hash_prev: &U256, validator: &U256, 
                        transactions: &[Transaction], 
                        complexity: usize) -> [u8; 32] {
        // Calculate the message bytes
        let msg = Self::calc_msg(block_hash_prev, validator, transactions);

        // Number of transactions
        let size = transactions.len();

        // Calculate limit hash
        let limit_hash = Self::calc_limit_hash(size, complexity);

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
            hasher_clone.update(nonce_bytes);

            // Get the bytes of the final hash
            let hash_bytes = hasher_clone.finalize();

            // If the hash is valid return the generated nonce and U256
            if Self::is_hash_valid(&hash_bytes, &limit_hash) {
                return nonce_bytes;
            }
        }
    }

    /// Calculate maximum allowed block hash depending on the size.
    fn calc_limit_hash(size: usize, complexity: usize) -> Vec<u8> {
        let mut num = U256::from(1);
        num <<= 255 - complexity;
        let bytes = num.divide_unit(size as u64 + 1).unwrap().0.to_bytes();
        bytes.into_iter().rev().collect::<Vec<u8>>()
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;
    use crate::crypto::Schema;

    #[test]
    fn test_mine() {
        // Best value is complexity = 24 that corresponds to ~10 seconds 
        // per empty block (for --release, 1 core, desktop)
        let complexity = 8;

        // Initial arguments
        let mut rng = rand::rng();
        let schema = Schema::new();

        let block_hash_prev: U256 = rng.random();
        let validator: U256 = schema.gen_pair(&mut rng).1;

        let transactions: Vec<Transaction> = vec![];

        // Mining the nonce
        let nonce_bytes = Block::mine(&mut rng, &block_hash_prev, &validator, 
                                      &transactions, complexity);

        // Calculate hash
        let msg = Block::calc_msg(&block_hash_prev, &validator, &transactions);
        let nonce = U256::from_bytes(&nonce_bytes);
        let hash = hash_of_u256([&msg, &nonce].into_iter());

        // Calculate limit hash
        let limit_hash = Block::calc_limit_hash(transactions.len(), complexity);

        // Check that the hash is valid
        assert!(hash.to_bytes() <= limit_hash);
        assert!(Block::is_hash_valid(&hash.to_bytes(), &limit_hash));
    }

    #[bench]
    fn bench_mine(bencher: &mut Bencher) {
        let size = 10;

        let mut rng = rand::rng();
        let schema = Schema::new();

        let block_hash_prev: U256 = rng.random();
        let validator: U256 = schema.gen_pair(&mut rng).1;
        let coin: U256 = rng.random();
        let addr: U256 = rng.random();
        let key: U256 = schema.gen_key(&mut rng);

        let transactions: Vec<Transaction> = vec![
            Transaction::build(
                &mut rng, coin.clone(), addr.clone(), &key, &schema
            );
            size
        ];

        bencher.iter(|| {
            let _nonce = Block::mine(&mut rng, &block_hash_prev, &validator, 
                                     &transactions, 0);
        });
    }
}
