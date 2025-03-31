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
    groups: Vec<Group>,
    senders: Vec<U256>,
}


impl Pool {
    /// Create an empty pool.
    pub fn new() -> Self {
        Self {
            groups: Vec::new(),
            senders: Vec::new(),
        }
    }

    /// Add group to waiting transactions.
    pub fn add_group(&mut self, group: &Group, state: &State, sender: &U256) -> bool {
        let group_senders = vec![sender.clone(); group.len()];
        if Block::validate_coins(group.transactions(), state, &group_senders).is_ok() {
            self.groups.push(group.clone());
            self.senders.push(sender.clone());
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
        for (group, sender) in self.groups.iter().zip(self.senders.iter()).take(groups_max) {
            // Get order
            let order = group.get_order(state, &vec![sender.clone(); group.len()]);

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

    /// Update the pool with transactions of the new block. `state` is supposed 
    /// to be already rolled up.
    pub fn roll_up(&mut self, transactions: &[Transaction], state: &State) {
        let coins = transactions.iter().map(|tr| tr.coin.clone())
            .collect::<HashSet<U256>>();

        let mut groups = Vec::new();
        let mut senders = Vec::new();

        for (group, sender) in self.groups.iter().zip(self.senders.iter()) {
            let group_senders = vec![sender.clone(); group.len()];
            let coins_are_not_repeated = group.transactions().iter()
                .all(|tr| !coins.contains(&tr.coin));
            let coins_are_valid = Block::validate_coins(
                group.transactions(), state, &group_senders
            ).is_ok();
            if coins_are_not_repeated && coins_are_valid {
                groups.push(group.clone());
                senders.push(sender.clone());
            }
        }

        self.groups = groups;
        self.senders = senders;
    }

    /// Roll back the pool state with transactions of the last block. `state` 
    /// is supposed to be already rolled down.
    pub fn roll_down(&mut self, transactions: &[Transaction], state: &State, senders: &[U256]) {
        for (ofs, gr, _) in group_transactions(transactions.to_vec(), state, senders) {
            self.add_group(&gr, state, &senders[ofs]);
        }
    }

    /// Remove invalid groups according to the given state.
    pub fn update_groups(&mut self, state: &State) {
        let mut groups = Vec::new();
        let mut senders = Vec::new();

        for (group, sender) in self.groups.iter().zip(self.senders.iter()) {
            let group_senders = vec![sender.clone(); group.len()];
            if Block::validate_coins(group.transactions(), state, &group_senders).is_ok() {
                groups.push(group.clone());
                senders.push(sender.clone());
            }
        }

        self.groups = groups;
        self.senders = senders;
    }

    /// Merge pools, the state must correspond to `other` pool.
    pub fn merge(&mut self, other: &Self, state: &State) {
        for (group, sender) in other.groups.iter().zip(other.senders.iter()) {
            self.add_group(&group, state, &sender);
        }
    }
}
