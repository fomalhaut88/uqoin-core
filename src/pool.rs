use std::collections::{HashSet, HashMap};

use rand::Rng;

use crate::utils::*;
use crate::crypto::Schema;
use crate::coin::coin_is_valid;
use crate::transaction::{Type, Transaction, Group, Ext};
use crate::block::Block;
use crate::state::{CoinOwnerMap, CoinInfoMap, OwnerCoinsMap};


pub struct PoolConfig {
    validator: U256,
    // max_groups: Option<usize>,
    // fee_order_limit: u64,
    // ignore_exchange: bool,
}


/// Validator pool that keeps requested transactions.
pub struct Pool {
    config: PoolConfig,
    groups: Vec<Group>,
}


impl Pool {
    /// Create an empty pool.
    pub fn new(config: PoolConfig) -> Self {
        Self {
            config,
            groups: Vec::new(),
        }
    }

    /// Add group to waiting transactions.
    pub fn add_group(&mut self, group: &Group, coin_owner_map: &CoinOwnerMap, 
                     block_hash_prev: &U256, schema: &Schema) -> bool {
        if !Block::validate_coins(group.transactions(), schema, coin_owner_map, 
                                  block_hash_prev) {
            return false;
        }
        self.groups.push(group.clone());
        true
    }

    /// Get ready transactions for next block.
    pub fn get_ready(&self, owner_coins_map: &OwnerCoinsMap) -> Vec<Transaction> {
        // Transactions to return
        let mut transactions = vec![];

        // Validator resource
        let mut validator_resource = owner_coins_map[&self.config.validator]
            .iter().map(|(order, coins)| 
                (*order, Vec::<U256>::from_iter(coins.iter().cloned()))
            ).collect::<HashMap<u64, Vec<U256>>>();

        // Repeated coins are not valid
        let mut coin_set = HashSet::new();

        for group in self.groups.iter() {
            // Check same coins
            if group.transactions().iter().any(|tr| coin_set.contains(&tr.coin)) {
                continue;
            }
            for tr in group.transactions().iter() {
                coin_set.insert(tr.coin.clone());
            }

            // Add ext transactions
            // ...

            // Extend transactions if ext was added
            // ...
        }

        transactions
    }

    pub fn roll_up(&mut self) {
        // Drop groups by intersected coins
        // ...

        // Remove mined extra coins
        // ...
    }

    pub fn roll_down(&mut self) {
        // Add new groups from rolled transactions
        // ...
    }

    // /// Get transactions for mining the next block.
    // pub fn ready(&self) -> &[(Group, Ext)] {
    //     &self.ready
    // }

    // /// Use `update` after new block added to blockchain (after syncing or 
    // /// mining).
    // pub fn update(&mut self) {
    //     unimplemented!()
    // }

    // /// Prepare and move groups from `waiting` to `ready`.
    // pub fn prepare<R: Rng>(&mut self, rng: &mut R, coin_info_map: &CoinInfoMap, 
    //                        owner_coins_map: &OwnerCoinsMap, schema: &Schema) {
    //     let mut coins_seen = HashSet::<U256>::new();
    //     let mut gix_to_move = Vec::<usize>::new();
    //     let mut validator_resource = owner_coins_map[&self.config.validator]
    //         .iter().map(|(order, coins)| 
    //             (*order, Vec::<U256>::from_iter(coins.iter().cloned()))
    //         ).collect::<HashMap<u64, Vec<U256>>>();

    //     for (gix, gr) in self.waiting.iter().enumerate() {
    //         let all_coins_not_seen = gr.transactions().iter()
    //             .all(|tr| !coins_seen.contains(&tr.coin));

    //         if all_coins_not_seen {
    //             let sender = gr.get_sender(schema);
    //             let order = gr.get_order(coin_info_map);

    //             let ext_trs: Option<Vec<Transaction>> = match gr.get_type() {
    //                 Type::Transfer => Some(vec![]),
    //                 Type::Split => {
    //                     [order - 1, order - 2, order - 2].iter().map(|ord| {
    //                         let coin_vec = validator_resource.get_mut(ord)?;
    //                         let coin = coin_vec.pop()?;
    //                         Some(Transaction::prepare_transfer(
    //                             rng, &self.config.validator_key, &coin, 
    //                             &sender, &[], schema
    //                         )[0].clone())
    //                     }).collect()
    //                 },
    //                 Type::Merge => {
    //                     [order].iter().map(|ord| {
    //                         let coin_vec = validator_resource.get_mut(ord)?;
    //                         let coin = coin_vec.pop()?;
    //                         Some(Transaction::prepare_transfer(
    //                             rng, &self.config.validator_key, &coin, 
    //                             &sender, &[], schema
    //                         )[0].clone())
    //                     }).collect()
    //                 },
    //                 _ => panic!("Invalid group type"),
    //             };

    //             if let Some(ext_trs) = ext_trs {
    //                 if let Some(ext) = Ext::new(ext_trs, schema, 
    //                                             coin_info_map) {
    //                     self.ready.push((gr.clone(), ext));
    //                     gix_to_move.push(gix);
    //                     coins_seen.extend(
    //                         gr.transactions().iter().map(|tr| tr.coin.clone())
    //                     );
    //                 }
    //             }
    //         }
    //     }

    //     let gix_to_move = gix_to_move.into_iter().collect::<HashSet<usize>>();

    //     self.waiting = self.waiting.iter().enumerate()
    //         .filter(|(gix, _)| !gix_to_move.contains(gix))
    //         .map(|(_, gr)| gr.clone())
    //         .collect();
    // }
}
