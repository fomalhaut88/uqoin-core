use std::collections::{HashSet, HashMap};

use rand::Rng;

use crate::utils::*;
use crate::schema::Schema;
use crate::transaction::{Type, Transaction, Group, group_transactions};
use crate::block::Block;
use crate::state::State;


/// Validator pool that keeps requested transactions.
#[derive(Debug, Clone)]
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
        if Block::validate_coins(group.transactions(), schema, state).is_ok() {
            self.groups.push(group.clone());
            true
        } else {
            false
        }
    }

    /// Get ready transactions for next block.
    pub fn prepare<R: Rng>(&self, rng: &mut R, schema: &Schema, state: &State, 
                           validator_key: &U256, groups_max: Option<usize>) -> 
                           Vec<Transaction> {
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
        let groups_max = groups_max.unwrap_or(1000000000);
        for group in self.groups.iter().take(groups_max) {
            // Get sender and order
            let sender = group.get_sender(state, schema);
            let order = group.get_order(state, schema);

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
                    let counter = state.get_coin_counter(&coin);
                    coin_set.insert(coin.clone());
                    Some(Transaction::build(rng, coin, sender.clone(), 
                                            validator_key, counter, schema))
                }).collect(),
                Type::Split => [order-1, order-2, order-2].iter().map(|ord| {
                    let coin = Self::get_validator_coin(
                        ord, &mut validator_resource, &coin_set
                    )?;
                    let counter = state.get_coin_counter(&coin);
                    coin_set.insert(coin.clone());
                    Some(Transaction::build(rng, coin, sender.clone(), 
                                            validator_key, counter, schema))
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

        // Update existing groups
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

        // Update existing groups
        self.update_groups(schema, state);
    }

    /// Remove invalid groups according to the given state.
    pub fn update_groups(&mut self, schema: &Schema, state: &State) {
        // Remove invalid groups in the current state
        self.groups = self.groups.iter().filter(
            |gr| Block::validate_coins(gr.transactions(), schema, state).is_ok()
        ).cloned().collect();
    }

    /// Merge pools, the state must correspond to `other` pool.
    pub fn merge(&mut self, other: &Self, schema: &Schema, state: &State) {
        // Update existing groups
        self.update_groups(schema, state);

        // Add groups
        for gr in other.groups.iter() {
            self.add_group(&gr, schema, state);
        }
    }
}
