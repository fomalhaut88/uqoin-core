use std::collections::HashSet;

use rand::Rng;

use crate::validate;
use crate::utils::*;
use crate::transaction::{Type, Transaction, Group};
use crate::block::Block;
use crate::schema::Schema;
use crate::state::{State, OrderCoinsMap};


/// Validator pool that keeps requested transactions.
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

    /// Add a new group.
    pub fn add(&mut self, group: &Group, state: &State, 
               schema: &Schema) -> UqoinResult<()> {
        let senders = Transaction::calc_senders(group.transactions(), state, 
                                                schema);
        validate!(check_same(senders.iter()), TransactionInvalidSender)?;
        Block::validate_coins(group.transactions(), state, &senders)?;
        self.groups.push(group.clone());
        self.senders.push(senders[0].clone());
        Ok(())
    }

    /// Update the pool according to the given state. This function 
    /// recalculates senders, so it may take a while.
    pub fn update(&mut self, state: &State, schema: &Schema) {
        let groups = self.groups.clone();
        self.groups = Vec::new();
        self.senders = Vec::new();
        for group in groups.iter() {
            let _ = self.add(group, state, schema);
        }
    }

    /// Prepare transactions and senders for the next block. The pool must be
    /// updated to the state.
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
