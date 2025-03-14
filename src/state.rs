use std::collections::{HashMap, HashSet};

use crate::utils::*;
use crate::crypto::Schema;
use crate::coin::Coin;
use crate::block::Block;
use crate::transaction::Transaction;


/// Hash of the zero block.
const GENESIS_HASH: &str = 
    "E12BA98A17FD8F70608668AA32AEB3BE1F202B4BD69880A6C0CFE855B1A0706B";


/// Information about coin.
#[derive(Clone)]
pub struct CoinInfo {
    /// Order of the coin (value is 2^order)
    pub order: u64,

    /// Transaction number index where the coin was mined.
    pub tix: u64,

    /// Block number where the coin was miner.
    pub bix: u64,
}


/// Information about the last block.
#[derive(Clone)]
pub struct LastBlockInfo {
    /// Last block number.
    pub bix: u64,

    /// Last block hash.
    pub hash: U256,
}


/// Uqoin state for fast access to the last block, coin and ownership
/// information.
#[derive(Clone)]
pub struct State {
    coin_owner_map: HashMap<U256, U256>,
    coin_info_map: HashMap<U256, CoinInfo>,
    owner_coin_map: HashMap<U256, HashMap<u64, HashSet<U256>>>,
    last_block_info: LastBlockInfo,
}


impl State {
    /// Create initial state.
    pub fn new() -> Self {
        Self {
            coin_owner_map: HashMap::new(),
            coin_info_map: HashMap::new(),
            owner_coin_map: HashMap::new(),
            last_block_info: LastBlockInfo {
                bix: 0, 
                hash: U256::from_hex(GENESIS_HASH),
            },
        }
    }

    /// Get owner of the coin by number.
    pub fn get_owner(&self, coin: &U256) -> &U256 {
        &self.coin_owner_map[coin]
    }

    /// Get coin info by number.
    pub fn get_coin_info(&self, coin: &U256) -> &CoinInfo {
        &self.coin_info_map[coin]
    }

    /// Get coins of the owner.
    pub fn get_coins(&self, owner: &U256) -> &HashMap<u64, HashSet<U256>> {
        &self.owner_coin_map[owner]
    }

    /// Get last block info.
    pub fn get_last_block_info(&self) -> &LastBlockInfo {
        &self.last_block_info
    }

    /// Roll up the state with the next block.
    pub fn roll_up(&mut self, bix: u64, block: &Block, 
                   transactions: &[Transaction], schema: &Schema) {
        // Check the block
        assert_eq!(bix, self.last_block_info.bix + 1);
        assert_eq!(block.hash_prev, self.last_block_info.hash);

        // Iterate transactions
        for (ix, transaction) in transactions.iter().enumerate() {
            // tix
            let tix = block.tix + ix as u64;

            // Get sender
            let sender = transaction.get_sender(schema);

            // Get receiver
            let receiver = if transaction.is_fee() {
                &block.validator
            } else {
                &transaction.addr
            };
            
            // Check the coin already exists
            if self.coin_info_map.contains_key(&transaction.coin) {
                // Update coin owner
                *self.coin_owner_map.get_mut(&transaction.coin).unwrap() = 
                    receiver.clone();

                // Remove coin from the sender
                self.owner_coin_remove(&sender, &transaction.coin);

                // Add coin to the receiver
                self.owner_coin_add(&receiver, &transaction.coin);
            } else {
                // Calculate coin order
                let coin = Coin::new(transaction.coin.clone(), 
                                     block.hash_prev.clone(), sender.clone());

                // Insert into coin info map
                self.coin_info_map.insert(
                    transaction.coin.clone(), 
                    CoinInfo { order: coin.order(), bix, tix }
                );

                // Insert into coin owner map
                self.coin_owner_map.insert(transaction.coin.clone(), 
                                           receiver.clone());

                // Add coin to the receiver
                self.owner_coin_add(&receiver, &transaction.coin);
            }
        }

        // Update last block info
        self.last_block_info.bix = bix;
        self.last_block_info.hash = block.hash.clone();
    }

    /// Roll down the state with the last block.
    pub fn roll_down(&mut self, bix: u64, block: &Block, 
                     transactions: &[Transaction], schema: &Schema) {
        // Check the block
        assert_eq!(bix, self.last_block_info.bix);
        assert_eq!(block.hash, self.last_block_info.hash);

        // Update last block info
        self.last_block_info.bix -= 1;
        self.last_block_info.hash = block.hash_prev.clone();

        // Iterate transactions
        for (ix, transaction) in transactions.iter().enumerate() {
            // tix
            let tix = block.tix + ix as u64;

            // Get sender
            let sender = transaction.get_sender(schema);

            // Get receiver
            let receiver = if transaction.is_fee() {
                &block.validator
            } else {
                &transaction.addr
            };

            // Check the coin was mined in this block
            if self.coin_info_map[&transaction.coin].tix == tix {
                // Remove from owner coin map
                self.owner_coin_remove(&receiver, &transaction.coin);

                // Remove from coin owner map
                self.coin_owner_map.remove(&transaction.coin);

                // Remove from coin info map
                self.coin_info_map.remove(&transaction.coin);
            } else {
                // Remove coin from the receiver
                self.owner_coin_remove(&receiver, &transaction.coin);

                // Add coin to the sender
                self.owner_coin_add(&sender, &transaction.coin);

                // Update coin owner
                *self.coin_owner_map.get_mut(&transaction.coin).unwrap() = 
                    sender.clone();
            }
        }
    }

    fn owner_coin_add(&mut self, owner: &U256, coin: &U256) {
        // Get coin order
        let order = self.coin_info_map[coin].order;

        // Ensure map for owner
        if !self.owner_coin_map.contains_key(owner) {
            self.owner_coin_map.insert(owner.clone(), HashMap::new());
        }

        // Ensure set for order
        if !self.owner_coin_map[owner].contains_key(&order) {
            self.owner_coin_map.get_mut(owner).unwrap()
                .insert(order, HashSet::new());
        }

        // Insert the coin
        self.owner_coin_map.get_mut(owner).unwrap()
            .get_mut(&order).unwrap().insert(coin.clone());
    }

    fn owner_coin_remove(&mut self, owner: &U256, coin: &U256) {
        // Get coin order
        let order = self.coin_info_map[coin].order;

        // Remove the coin
        self.owner_coin_map.get_mut(owner).unwrap()
            .get_mut(&order).unwrap().remove(coin);

        // Remove empty set
        if self.owner_coin_map[owner][&order].is_empty() {
            self.owner_coin_map.get_mut(owner).unwrap().remove(&order);
        }

        // Remove empty map
        if self.owner_coin_map[owner].is_empty() {
            self.owner_coin_map.remove(owner);
        }
    }
}
