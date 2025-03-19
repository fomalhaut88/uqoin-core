use std::collections::{HashSet, HashMap};

use rand::Rng;

use crate::utils::*;
use crate::schema::Schema;
use crate::transaction::{Type, Transaction, Group, group_transactions};
use crate::block::Block;
use crate::state::State;


// /// Config for pool instance.
// pub struct PoolConfig {
//     pub validator_key: U256,
// }


// /// Request status.
// #[derive(PartialEq)]
// pub enum RequestStatus {
//     Pending,
//     Success,
//     Cancel,
// }


// /// Request group that keep the status information.
// pub struct Request {
//     /// Group to process.
//     pub group: Group,

//     /// Timestamp.
//     pub ts: u64,

//     /// Status of the request.
//     pub status: RequestStatus,

//     /// Transaction index (tix) in the inserted block.
//     pub tix: Option<u64>,
// }


// /// Request mapping group hash -> request.
// pub type RequestMap = HashMap<U256, Request>;


/// Validator pool that keeps requested transactions.
pub struct Pool {
    // config: PoolConfig,
    groups: Vec<Group>,
}


impl Pool {
    /// Create an empty pool.
    pub fn new() -> Self {
        Self {
            groups: Vec::new(),
        }
    }

    /// Add group to waiting transactions.
    pub fn add_group(&mut self, group: &Group, schema: &Schema, 
                     state: &State) -> bool {
        if !Block::validate_coins(group.transactions(), schema, state) {
            return false;
        }
        self.groups.push(group.clone());
        true
    }

    /// Get ready transactions for next block.
    pub fn prepare<R: Rng>(&self, rng: &mut R, schema: &Schema, state: &State, 
                           validator_key: &U256) -> Vec<Transaction> {
        // Transactions to return
        let mut transactions = vec![];

        // Validator public
        let validator = schema.get_public(validator_key);

        // Validator resource
        let mut validator_resource = if let Some(coins_map) = 
                state.get_coins(&validator) {
            coins_map.iter().map(|(order, coins)| 
                (*order, Vec::<U256>::from_iter(coins.iter().cloned()))
            ).collect::<HashMap<u64, Vec<U256>>>()
        } else {
            HashMap::new()
        };

        // Set for seen coins to avoid repeating
        let mut coin_set = HashSet::new();

        // Look for all groups
        for group in self.groups.iter() {
            // Get sender and order
            let sender = group.get_sender(schema);
            let order = group.get_order(state);

            // Skip if the group contains any seen coin
            if group.transactions().iter()
                    .any(|tr| coin_set.contains(&tr.coin)) {
                continue;
            }

            // Update seen coins
            for tr in group.transactions().iter() {
                coin_set.insert(tr.coin.clone());
            }

            // Calculate ext transactions
            let ext_trs: Option<Vec<Transaction>> = match group.get_type() {
                Type::Transfer => Some(vec![]),
                Type::Merge => [order].iter().map(|ord| {
                    let coin = Self::get_validator_coin(
                        ord, &mut validator_resource, &coin_set
                    )?;
                    coin_set.insert(coin.clone());
                    Some(Transaction::build(rng, coin, sender.clone(), 
                                            validator_key, schema))
                }).collect(),
                Type::Split => [order-1, order-2, order-2].iter().map(|ord| {
                    let coin = Self::get_validator_coin(
                        ord, &mut validator_resource, &coin_set
                    )?;
                    coin_set.insert(coin.clone());
                    Some(Transaction::build(rng, coin, sender.clone(), 
                                            validator_key, schema))
                }).collect(),
                _ => panic!("Invalid group type"),
            };

            // Extend transactions if ext was added
            if let Some(ext_trs) = ext_trs {
                transactions.extend(group.transactions().iter().cloned());
                transactions.extend(ext_trs);
            }
        }

        // Return collected transactions
        transactions
    }

    /// Pop coin from the resource by order ignoring specified coins.
    fn get_validator_coin(order: &u64, resource: &mut HashMap<u64, Vec<U256>>, 
                          ignore_coins: &HashSet<U256>) -> Option<U256> {
        if let Some(series) = resource.get_mut(&order) {
            while let Some(coin) = series.pop() {
                if !ignore_coins.contains(&coin) {
                    return Some(coin);
                }
            }
        }
        None
    }

    /// Update the pool with transactions of the new block.
    pub fn roll_up(&mut self, transactions: &[Transaction], schema: &Schema, 
                   state: &State) {
        // Drop groups by intersected coins
        let coins = transactions.iter().map(|tr| tr.coin.clone())
            .collect::<HashSet<U256>>();
        self.groups = self.groups.iter().filter(
            |gr| gr.transactions().iter().all(|tr| !coins.contains(&tr.coin))
        ).cloned().collect();

        // Update existing groups.
        self.update_groups(schema, state);
    }

    /// Roll back the pool state with transactions of the last block.
    pub fn roll_down(&mut self, transactions: &[Transaction], schema: &Schema, 
                     state: &State) {
        // Add new groups from rolled transactions
        for (gr, _) in group_transactions(transactions.to_vec(), schema, 
                                          state) {
            self.add_group(&gr, schema, state);
        }

        // Update existing groups.
        self.update_groups(schema, state);
    }

    /// Remove invalid groups according to the given state.
    pub fn update_groups(&mut self, schema: &Schema, state: &State) {
        // Remove invalid groups in the current state
        self.groups = self.groups.iter().filter(
            |gr| Block::validate_coins(gr.transactions(), schema, state)
        ).cloned().collect();
    }
}
