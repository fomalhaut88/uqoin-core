use rand::Rng;
use sha3::{Sha3_256, Digest};
use serde::{Serialize, Deserialize};

use crate::validate;
use crate::utils::*;
use crate::transaction::{Type, Transaction, group_transactions};
use crate::state::State;


/// Hash of the zero block.
pub const GENESIS_HASH: &str = 
    "E12BA98A17FD8F70608668AA32AEB3BE1F202B4BD69880A6C0CFE855B1A0706B";

/// Complexity after calibration.
pub const COMPLEXITY: usize = 24;


/// Basic structure for block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub offset: u64,
    pub size: u64,
    pub hash_prev: U256,
    pub validator: U256,
    pub nonce: U256,
    pub hash: U256,
}


impl Block {
    /// New block.
    pub fn new(offset: u64, size: u64, hash_prev: U256, validator: U256, 
               nonce: U256, hash: U256) -> Self {
        Self { offset, size, hash_prev, validator, nonce, hash }
    }

    /// Full validation of the block that includes transactions, info of the 
    /// previous block, complexity, state between this block and the previous
    /// one.
    pub fn validate(&self, transactions: &[Transaction], 
                    block_info_prev: &BlockInfo, complexity: usize, 
                    state: &State, senders: &[U256]) -> UqoinResult<()> {
        // Check block hash
        validate!(block_info_prev.hash == self.hash_prev, 
                  BlockPreviousHashMismatch)?;

        // Check block offset
        validate!(block_info_prev.offset == self.offset, 
                  BlockOffsetMismatch)?;

        // Validate transactions
        Self::validate_transactions(transactions, &self.validator, state, 
                                    senders)?;

        // Calculate the message
        let msg = Self::calc_msg(&self.hash_prev, &self.validator, 
                                 transactions);

        // Calculate the hash
        let hash = Self::calc_hash(&msg, &self.nonce);

        // Check hash
        validate!(hash == self.hash, BlockInvalidHash)?;

        // Validate hash
        Self::validate_hash_complexity(&self.hash, transactions.len(), 
                                       complexity)?;

        // Return
        Ok(())
    }

    /// Build a new block for the transactions. It validates the final hash.
    pub fn build(block_info_prev: &BlockInfo, validator: U256, 
                 transactions: &[Transaction], nonce: U256,
                 complexity: usize, state: &State, 
                 senders: &[U256]) -> UqoinResult<Self> {
        // Validate transactions
        Self::validate_transactions(transactions, &validator, state, senders)?;

        // Calculate the message
        let msg = Self::calc_msg(&block_info_prev.hash, &validator, 
                                 transactions);

        // Calculate the hash
        let hash = Self::calc_hash(&msg, &nonce);

        // Validate hash
        Self::validate_hash_complexity(&hash, transactions.len(), complexity)?;

        // Create a block
        Ok(Self::new(block_info_prev.offset, 
                     transactions.len() as u64, 
                     block_info_prev.hash.clone(),
                     validator, nonce, hash))
    }

    /// Validate coins. The checks:
    /// 1. All coins are unique.
    /// 2. All transactions are valid (see `Transaction::validate_coins()`).
    #[deprecated(since="0.1.0", note="use groups and check_unique instead")]
    pub fn validate_coins(transactions: &[Transaction], state: &State, 
                          senders: &[U256]) -> UqoinResult<()> {
        // Repeated coins are not valid
        validate!(check_unique(transactions.iter().map(|tr| &tr.coin)), 
                  CoinNotUnique)?;

        // Validate coin in each transaction
        for (transaction, sender) in transactions.iter().zip(senders.iter()) {
            transaction.validate_coin(state, sender)?;
        }

        Ok(())
    }

    /// Validate transactions. The checks:
    /// 1. All coins are valid (see `validate_coins`).
    /// 2. All transactions can be groupped into groups and extensions.
    /// 3. Sender of each extension is the validator.
    /// 4. Values of groups and extensions correspond each other.
    /// Each group or extension has valid structure after the groupping because
    /// they cannot be created invalid due to inner validation.
    pub fn validate_transactions(transactions: &[Transaction], validator: &U256, 
                                 state: &State, senders: &[U256]) -> 
                                 UqoinResult<()> {
        // // Check coins
        // Self::validate_coins(transactions, state, senders)?;

        // Repeated coins are not valid
        validate!(check_unique(transactions.iter().map(|tr| &tr.coin)), 
                  CoinNotUnique)?;

        // Set a countdown for groupped transactions
        let mut countdown = transactions.len();

        // Loop for groups and extensions
        for (offset, group, ext) in group_transactions(transactions.to_vec(), 
                                                       state, senders) {
            // Get senders
            let group_senders = &senders[offset .. offset + group.len()];
            let ext_senders = &senders[
                offset + group.len() .. offset + group.len() + ext.len()
            ];

            // Check validator
            if let Some(ext_sender) = ext.get_sender(ext_senders) {
                validate!(&ext_sender == validator, BlockValidatorMismatch)?;
            }

            // Check value
            if ext.get_type() != Type::Transfer {
                validate!(group.get_order(state, group_senders) 
                    == ext.get_order(state, ext_senders), BlockOrderMismatch)?;
            }

            // Decrement the countdown
            countdown -= group.len() + ext.len();
        }

        // Validate that all transactions have been groupped
        validate!(countdown == 0, BlockBroken)?;

        Ok(())
    }

    /// Validate hash for the certain complexity.
    pub fn validate_hash_complexity(hash: &U256, size: usize, 
                                    complexity: usize) -> UqoinResult<()> {
        let limit_hash = Self::calc_limit_hash(size, complexity);
        validate!(Self::is_hash_valid(&hash.to_bytes(), &limit_hash), 
                  BlockInvalidHashComplexity)
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
                        complexity: usize, 
                        iterations: Option<usize>) -> Option<[u8; 32]> {
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
        for iteration in 0.. {
            // Stop by iterations
            if let Some(iterations) = iterations {
                if iteration >= iterations {
                    break;
                }
            }

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
                return Some(nonce_bytes);
            }
        }

        // Return `None` if nothing mined
        None
    }

    /// Calculate maximum allowed block hash depending on the size.
    fn calc_limit_hash(size: usize, complexity: usize) -> Vec<u8> {
        assert!(complexity > 0);
        let mut num = U256::from(1);
        num <<= 256 - complexity;
        let bytes = if size > 1 {
            num.divide_unit(size as u64).unwrap().0.to_bytes()
        } else {
            num.to_bytes()
        };
        bytes.into_iter().rev().collect::<Vec<u8>>()
    }
}


/// Short information about the block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockInfo {
    /// Block number.
    pub bix: u64,

    /// Total number of transaction up to this block (`offset` for the next 
    /// block).
    pub offset: u64,

    /// Last block hash.
    pub hash: U256,
}


impl BlockInfo {
    /// Get information of the genesis block (`bix=0`).
    pub fn genesis() -> Self {
        Self {
            bix: 0,
            offset: 0,
            hash: U256::from_hex(GENESIS_HASH),
        }
    }
}


/// Full information about the block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockData {
    /// Block number.
    pub bix: u64,

    /// Block data.
    pub block: Block,

    /// Included transactions.
    pub transactions: Vec<Transaction>,
}


impl BlockData {
    /// Get data of the genesis block (`bix=0`).
    pub fn genesis() -> Self {
        Self {
            bix: 0,
            block: Block {
                offset: 0,
                size: 0,
                hash_prev: U256::from(0),
                validator: U256::from(0),
                nonce: U256::from(0),
                hash: U256::from_hex(GENESIS_HASH),
            },
            transactions: Vec::new(),
        }
    }

    /// Get short information.
    pub fn get_block_info(&self) -> BlockInfo {
        BlockInfo {
            bix: self.bix,
            offset: self.block.offset + self.block.size,
            hash: self.block.hash.clone(),
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;
    use crate::schema::Schema;

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
                                      &transactions, complexity, 
                                      Some(10000)).unwrap();

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
    fn bench_mine_10(bencher: &mut Bencher) {
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
                &mut rng, coin.clone(), addr.clone(), &key, 0, &schema
            );
            size
        ];

        bencher.iter(|| {
            let _nonce = Block::mine(&mut rng, &block_hash_prev, &validator, 
                                     &transactions, 1, None);
        });
    }
    
    // Uncomment it to start calibration: 
    //     `cargo bench block::tests::bench_mine_calibration`
    // #[bench]
    // fn bench_mine_calibration(bencher: &mut Bencher) {
    //     // The result is ~4 s/iter
    //     let complexity = 24;

    //     let mut rng = rand::rng();
    //     let schema = Schema::new();

    //     let block_hash_prev: U256 = rng.random();
    //     let validator: U256 = schema.gen_pair(&mut rng).1;
    //     let coin: U256 = rng.random();
    //     let addr: U256 = rng.random();
    //     let key: U256 = schema.gen_key(&mut rng);

    //     let transactions: Vec<Transaction> = vec![
    //         Transaction::build(
    //             &mut rng, coin.clone(), addr.clone(), &key, 0, &schema
    //         ),
    //     ];

    //     bencher.iter(|| {
    //         let _nonce = Block::mine(&mut rng, &block_hash_prev, &validator, 
    //                                  &transactions, complexity, None);
    //     });
    // }
}
