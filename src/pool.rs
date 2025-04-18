//! Pool is a structure that keeps groups of transactions that are considered
//! to be added to a new block. Since groups are valid within a certain state
//! it is recommended to update the pool any time if the state is changed so
//! the pool will contain only the relevant groups. Pool is needed to prepare
//! transactions for a new block by `prepare`.

use std::collections::HashSet;

use rand::Rng;

use crate::utils::*;
use crate::transaction::{Type, Transaction, Group};
use crate::schema::Schema;
use crate::state::{State, OrderCoinsMap};


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

    /// Clear pool.
    pub fn clear(&mut self) {
        self.groups.clear();
        self.senders.clear();
    }

    /// Add a new group. `sender` must correspond to the group sender that is
    /// required on group creation.
    pub fn add(&mut self, group: Group, sender: U256) {
        self.groups.push(group);
        self.senders.push(sender);
    }

    /// Update the pool according to the given state. Valid group in one state
    /// may be invalid in another. This function recalculates senders based on
    /// the state, so it may take a while.
    pub fn update(&mut self, state: &State, schema: &Schema) {
        let old_groups = self.groups.clone();
        self.groups = Vec::new();
        self.senders = Vec::new();
        for old_group in old_groups.iter() {
            let senders = Transaction::calc_senders(&old_group.transactions(), 
                                                    state, schema);
            if let Ok(group) = Group::new(old_group.transactions().to_vec(), 
                                          state, &senders) {
                self.add(group, senders[0].clone());
            }
        }
    }

    /// Prepare transactions and senders for the next block. The pool must be
    /// updated according to this state.
    pub fn prepare<R: Rng>(&self, rng: &mut R, state: &State, schema: &Schema,
                           validator_key: &U256, groups_max: Option<usize>) -> 
                           (Vec<Transaction>, Vec<U256>) {
        // Transactions and senders to fill
        let mut transactions = Vec::new();
        let mut senders = Vec::new();

        // Validator public
        let validator = schema.get_public(validator_key);

        // Validator resource
        let mut validator_resource = state.get_coins(&validator).cloned()
                                          .unwrap_or(OrderCoinsMap::new());

        // Set of seen coins
        let mut coins_seen = HashSet::new();

        // Counter of added groups
        let mut counter = 0;

        // Loop for groups and corresponding senders
        for (group, sender) in self.groups.iter().zip(self.senders.iter()) {
            // Leave if groups_max is reached
            if let Some(groups_max) = groups_max {
                if counter >= groups_max {
                    break;
                }
            }

            // Skip if the group contains any seen coin
            if group.transactions().iter()
                    .any(|tr| coins_seen.contains(&tr.coin)) {
                continue;
            }

            // Update seen coins
            for tr in group.transactions().iter() {
                coins_seen.insert(tr.coin.clone());
            }

            // Group senders
            let group_senders = vec![sender.clone(); group.len()];

            // Get order
            let order = group.get_order(state, &group_senders);

            // Calculate ext transactions
            let ext_trs: Option<Vec<Transaction>> = match group.get_type() {
                Type::Transfer => Some(vec![]),
                Type::Merge => [order].iter().map(|ord| {
                    let coin = Self::get_validator_coin(
                        ord, &mut validator_resource, &coins_seen
                    )?;
                    let counter = state.get_coin_counter(&coin);
                    coins_seen.insert(coin.clone());
                    Some(Transaction::build(rng, coin, sender.clone(), 
                                            validator_key, counter, schema))
                }).collect(),
                Type::Split => [order-1, order-2, order-2].iter().map(|ord| {
                    let coin = Self::get_validator_coin(
                        ord, &mut validator_resource, &coins_seen
                    )?;
                    let counter = state.get_coin_counter(&coin);
                    coins_seen.insert(coin.clone());
                    Some(Transaction::build(rng, coin, sender.clone(), 
                                            validator_key, counter, schema))
                }).collect(),
                _ => panic!("Invalid group type"),
            };

            // Extend transactions and senders if ext was added
            if let Some(ext_trs) = ext_trs {
                senders.extend(group_senders);
                senders.extend(vec![validator.clone(); ext_trs.len()]);

                transactions.extend(group.transactions().iter().cloned());
                transactions.extend(ext_trs);

                counter += 1;
            }
        }

        // Return transactions and senders
        (transactions, senders)
    }

    /// Pop coin from the resource by order ignoring specified coins.
    fn get_validator_coin(order: &u64, resource: &mut OrderCoinsMap, 
                          ignore_coins: &HashSet<U256>) -> Option<U256> {
        if let Some(set) = resource.get_mut(&order) {
            let coin_opt = set.iter().filter(|c| !ignore_coins.contains(c))
                              .next().cloned();

            if let Some(coin) = coin_opt {
                set.remove(&coin);
                return Some(coin);
            }
        }
        None
    }
}
