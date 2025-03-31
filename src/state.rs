use std::collections::{HashMap, HashSet};

use serde::{Serialize, Deserialize};

#[cfg(feature = "blockchain")]
use tokio::io::{Result as TokioResult};

use crate::utils::*;
use crate::schema::Schema;
use crate::coin::coin_order;
use crate::block::{Block, BlockInfo};
use crate::transaction::{Transaction, Type};


/// State information about coin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinInfo {
    /// Current owner.
    pub owner: U256,

    /// Order (it does not change).
    pub order: u64,

    /// Counter of transfers.
    pub counter: u64,
}


/// Map coin-state
pub type CoinInfoMap = HashMap<U256, CoinInfo>;

/// Map order-coins
pub type OrderCoinsMap = HashMap<u64, HashSet<U256>>;

/// Map owner-coins
pub type OwnerCoinsMap = HashMap<U256, OrderCoinsMap>;


/// Uqoin state for fast access to the last block, coin and ownership
/// information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    coin_info_map: CoinInfoMap,
    owner_coins_map: OwnerCoinsMap,
    last_block_info: BlockInfo,
}


impl State {
    /// Create initial state.
    pub fn new() -> Self {
        Self {
            coin_info_map: CoinInfoMap::new(),
            owner_coins_map: OwnerCoinsMap::new(),
            last_block_info: BlockInfo::genesis(),
        }
    }

    /// Load from a file.
    #[cfg(feature = "blockchain")]
    pub async fn load(path: &str) -> TokioResult<Self> {
        let bytes = tokio::fs::read(path).await?;
        let content = String::from_utf8(bytes).unwrap();
        let instance = serde_json::from_str(&content)?;
        Ok(instance)
    }

    /// Dump to a file.
    #[cfg(feature = "blockchain")]
    pub async fn dump(&self, path: &str) -> TokioResult<()> {
        let content = serde_json::to_string(self).unwrap();
        tokio::fs::write(path, content.as_bytes()).await
    }

    /// Get owner of the coin by number.
    pub fn get_owner(&self, coin: &U256) -> Option<&U256> {
        self.coin_info_map.get(coin).map(|cs| &cs.owner)
    }

    /// Get coin state by number.
    pub fn get_coin_info(&self, coin: &U256) -> Option<&CoinInfo> {
        self.coin_info_map.get(coin)
    }

    /// Get coin state by number.
    pub fn get_coin_counter(&self, coin: &U256) -> u64 {
        self.coin_info_map.get(coin).map(|cs| cs.counter).unwrap_or(0)
    }

    /// Get coins of the owner.
    pub fn get_coins(&self, owner: &U256) -> Option<&OrderCoinsMap> {
        self.owner_coins_map.get(owner)
    }

    /// Get last block info.
    pub fn get_last_block_info(&self) -> &BlockInfo {
        &self.last_block_info
    }

    /// Roll up the state with the next block.
    pub fn roll_up(&mut self, bix: u64, block: &Block, 
                   transactions: &[Transaction], schema: &Schema) {
        // Check the block
        assert_eq!(bix, self.last_block_info.bix + 1);
        assert_eq!(block.offset, self.last_block_info.offset);
        assert_eq!(block.hash_prev, self.last_block_info.hash);

        // Calc senders (it is important to calculate it before counter updates)
        let senders = Transaction::calc_senders(&transactions, self, &schema);

        // Iterate transactions
        for (transaction, sender) in transactions.iter().zip(senders.iter()) {
            // Get receiver
            let receiver = if transaction.get_type() == Type::Transfer {
                &transaction.addr
            } else {
                &block.validator
            };
            
            // Check the coin already exists
            if let Some(coin_info) = self.coin_info_map
                                         .get_mut(&transaction.coin) {
                // Update coin state
                coin_info.owner = receiver.clone();
                coin_info.counter += 1;

                // Remove coin from the sender
                self.owner_coin_remove(&sender, &transaction.coin);

                // Add coin to the receiver
                self.owner_coin_add(&receiver, &transaction.coin);
            } else {
                // Calculate coin order
                let order = coin_order(&transaction.coin, &sender);

                // Create new coin state
                let coin_info = CoinInfo {
                    owner: receiver.clone(), order, counter: 1,
                };

                // Insert into coin info map
                self.coin_info_map.insert(transaction.coin.clone(), 
                                          coin_info);

                // Add coin to the receiver
                self.owner_coin_add(&receiver, &transaction.coin);
            }
        }

        // Update last block info
        self.last_block_info.bix = bix;
        self.last_block_info.offset += transactions.len() as u64;
        self.last_block_info.hash = block.hash.clone();
    }

    /// Roll down the state with the last block.
    pub fn roll_down(&mut self, bix: u64, block: &Block, 
                     transactions: &[Transaction], schema: &Schema) {
        // Check the block
        assert_eq!(bix, self.last_block_info.bix);
        assert_eq!(block.offset + transactions.len() as u64, 
                   self.last_block_info.offset);
        assert_eq!(block.hash, self.last_block_info.hash);

        // Update last block info
        self.last_block_info.bix -= 1;
        self.last_block_info.offset = block.offset;
        self.last_block_info.hash = block.hash_prev.clone();

        // First decrement counters in each coin so the message of the 
        // transaction will be correct to calculate the sender
        for transaction in transactions.iter() {
            self.coin_info_map.get_mut(&transaction.coin).unwrap().counter -= 1;
        }

        // Calc senders (it is important to calculate it after counter updates)
        let senders = Transaction::calc_senders(&transactions, self, &schema);

        // Iterate transactions
        for (transaction, sender) in transactions.iter().zip(senders.iter()) {
            // Get receiver
            let receiver = if transaction.get_type() == Type::Transfer {
                &transaction.addr
            } else {
                &block.validator
            };

            // Get coin info
            let coin_info = self.coin_info_map.get_mut(&transaction.coin)
                                              .unwrap();

            // Check the coin was mined in this block
            if coin_info.counter == 0 {
                // Remove from owner coin map
                self.owner_coin_remove(&receiver, &transaction.coin);

                // Remove from coin owner map
                self.coin_info_map.remove(&transaction.coin);
            } else {
                // Update coin owner
                coin_info.owner = sender.clone();

                // Remove coin from the receiver
                self.owner_coin_remove(&receiver, &transaction.coin);

                // Add coin to the sender
                self.owner_coin_add(&sender, &transaction.coin);
            }
        }
    }

    fn owner_coin_add(&mut self, owner: &U256, coin: &U256) {
        // Get coin order
        let order = self.coin_info_map[coin].order;

        // Ensure map for owner
        if !self.owner_coins_map.contains_key(owner) {
            self.owner_coins_map.insert(owner.clone(), HashMap::new());
        }

        // Ensure set for order
        if !self.owner_coins_map[owner].contains_key(&order) {
            self.owner_coins_map.get_mut(owner).unwrap()
                .insert(order, HashSet::new());
        }

        // Insert the coin
        self.owner_coins_map.get_mut(owner).unwrap()
            .get_mut(&order).unwrap().insert(coin.clone());
    }

    fn owner_coin_remove(&mut self, owner: &U256, coin: &U256) {
        // Get coin order
        let order = self.coin_info_map[coin].order;

        // Remove the coin
        let coins_map = self.owner_coins_map.get_mut(owner).unwrap();
        coins_map.get_mut(&order).unwrap().remove(coin);

        // Remove empty set
        if self.owner_coins_map[owner][&order].is_empty() {
            self.owner_coins_map.get_mut(owner).unwrap().remove(&order);
        }

        // Remove empty map
        if self.owner_coins_map[owner].is_empty() {
            self.owner_coins_map.remove(owner);
        }
    }
}
