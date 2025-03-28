use rand::Rng;
use serde::{Serialize, Deserialize};

use crate::validate;
use crate::utils::*;
use crate::schema::Schema;
use crate::coin::{coin_validate, coin_order};
use crate::state::State;


/// Types of transaction or group. In case of group Fee must be incorrect.
#[derive(Debug, PartialEq)]
pub enum Type {
    Transfer,
    Fee,
    Split,
    Merge,
}


/// Transaction base structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub coin: U256,
    pub addr: U256,
    pub sign_r: U256,
    pub sign_s: U256,
}


impl Transaction {
    /// Create a new transaction object.
    pub fn new(coin: U256, addr: U256, sign_r: U256, sign_s: U256) -> Self {
        Self { coin, addr, sign_r, sign_s }
    }

    /// Build a transaction of the `coin` from `key` to `addr`. In case of
    /// fee, split and merge use 0, 1 and 2 for `addr` respectively.
    pub fn build<R: Rng>(rng: &mut R, coin: U256, addr: U256, key: &U256, 
                         counter: u64, schema: &Schema) -> Self {
        let hash = Self::calc_msg(&coin, &addr, counter);
        let (sign_r, sign_s) = schema.build_signature(rng, &hash, key);
        Self::new(coin, addr, sign_r, sign_s)
    }

    /// Get transaction type.
    pub fn get_type(&self) -> Type {
        if self.addr == U256::from(0) {
            Type::Fee
        } else if self.addr == U256::from(1) {
            Type::Split
        } else if self.addr == U256::from(2) {
            Type::Merge
        } else {
            Type::Transfer
        }
    }

    /// Get transaction message as hash of coin and address.
    pub fn get_msg(&self, counter: u64) -> U256 {
        Self::calc_msg(&self.coin, &self.addr, counter)
    }

    /// Get transaction hash.
    pub fn get_hash(&self) -> U256 {
        hash_of_u256(
            [&self.coin, &self.addr, &self.sign_r, &self.sign_s].into_iter()
        )
    }

    /// Get transaction sender.
    pub fn get_sender(&self, state: &State, schema: &Schema) -> U256 {
        let counter = state.get_coin_counter(&self.coin);
        schema.extract_public(
            &self.get_msg(counter), 
            &(self.sign_r.clone(), self.sign_s.clone())
        )
    }

    /// Get order of the coin.
    pub fn get_order(&self, state: &State, schema: &Schema) -> u64 {
        if let Some(coin_info) = state.get_coin_info(&self.coin) {
            coin_info.order
        } else {
            let miner = self.get_sender(state, schema);
            coin_order(&self.coin, &miner)
        }
    }

    /// Validate coin in the transaction. The checks:
    /// 1. Sender is the owner of each coin, if it met before.
    /// 2. The coin number corresponds the previous block hash and the sender
    /// if the coin is new (just mined).
    pub fn validate_coin(&self, schema: &Schema, 
                         state: &State) -> UqoinResult<()> {
        // Get sender
        let sender = &self.get_sender(state, schema);

        // Try to find the coin in coin-owner map
        if let Some(owner) = state.get_owner(&self.coin) {
            // Check ownership
            validate!(owner == sender, TransactionInvalidSender)?;
        } else {
            // Check mining
            coin_validate(&self.coin, sender)?;
        }

        Ok(())
    }

    /// Calculate transaction message as hash of the `coin` and `addr`.
    pub fn calc_msg(coin: &U256, addr: &U256, counter: u64) -> U256 {
        hash_of_u256([coin, addr, &U256::from(counter)].into_iter())
    }
}


/// Group of transactions. Due to the check on create, group cannot be invalid.
/// The valid group must have: 1) unique coins, 2) the same sender, 
/// 3) consistent transaction order, types, values and count. Empty group is
/// not allowed. Coins are supposed to be correct, group does not check them,
/// use `Block::validate_coins` to check if necessary.
#[derive(Debug, Clone)]
pub struct Group(Vec<Transaction>);


impl Group {
    /// Create group from transactions. Validation is included, so if the
    /// vector is not valid, `None` will be returned.
    pub fn new(transactions: Vec<Transaction>, schema: &Schema, 
               state: &State) -> Option<Self> {
        if Self::validate_transactions(&transactions, schema, state).is_ok() {
            Some(Self(transactions))
        } else {
            None
        }
    }

    /// Try to create a group from the leading transactions in the given slice.
    /// Fees are joined by the greedy approach.
    pub fn from_vec(transactions: &mut Vec<Transaction>, schema: &Schema, 
                    state: &State) -> Option<Self> {
        if transactions.is_empty() {
            // `None` if the slice is empty
            None
        } else {
            // Size of the group without fee
            let mut size = match transactions[0].get_type() {
                Type::Split => 1,
                Type::Merge => 3,
                Type::Transfer => 1,
                _ => 0,
            };

            if size == 0 {
                // `None` if we start from a fee transaction
                None
            } else {
                // Increment size if the next transaction is fee
                if (size < transactions.len()) && 
                   (transactions[size].get_type() == Type::Fee) {
                    size += 1;
                }

                // Try to create a group using validation in `Self::new`
                let trs = vec_split_left(transactions, size);
                Self::new(trs, schema, state)
            }
        }
    }

    /// Accessor to the inner transactions.
    pub fn transactions(&self) -> &[Transaction] {
        &self.0
    }

    /// Get type of the group.
    pub fn get_type(&self) -> Type {
        self.0[0].get_type()
    }

    /// Get sender of the group.
    pub fn get_sender(&self, state: &State, schema: &Schema) -> U256 {
        self.0[0].get_sender(state, schema)
    }

    /// Get fee transaction.
    pub fn get_fee(&self) -> Option<&Transaction> {
        let fee_ix = match self.0[0].get_type() {
            Type::Split => 1,
            Type::Merge => 3,
            Type::Transfer => 1,
            _ => panic!("Invalid group."),
        };
        self.0.get(fee_ix)
    }

    /// Get hash of the group as the hash of leading transaction.
    pub fn get_hash(&self) -> U256 {
        self.0[0].get_hash()
    }

    /// Get total number of transactions.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Get order of the main coins.
    pub fn get_order(&self, state: &State, schema: &Schema) -> u64 {
        match self.get_type() {
            Type::Split => self.0[0].get_order(state, schema),
            Type::Merge => self.0[0].get_order(state, schema) + 1,
            Type::Transfer => self.0[0].get_order(state, schema),
            _ => panic!("Invalid transactions in the group."),
        }
    }

    /// Get number or required response transactions from the validator.
    pub fn ext_size(&self) -> usize {
        match self.get_type() {
            Type::Split => 3,
            Type::Merge => 1,
            Type::Transfer => 0,
            _ => panic!("Invalid transactions in the group."),
        }
    }

    /// Validate transactions for the group creation.
    pub fn validate_transactions(transactions: &[Transaction], schema: &Schema, 
                                 state: &State) -> UqoinResult<()> {
        // Error if no transactions in the slice
        validate!(!transactions.is_empty(), TransactionEmpty)?;

        // Check unique coins
        validate!(check_unique(transactions.iter().map(|tr| &tr.coin)), 
                  CoinNotUnique)?;

        // Check same sender
        validate!(check_same(transactions.iter()
                        .map(|tr| tr.get_sender(state, schema))), 
                  TransactionInvalidSender)?;

        // Check the first type
        match transactions[0].get_type() {
            // Error if the first transaction is fee
            Type::Fee => validate!(false, TransactionBrokenGroup)?,

            // Check the rest fees if split
            Type::Split => {
                if transactions.len() > 1 {
                    validate!(transactions.len() == 2, TransactionBrokenGroup)?;
                    validate!(transactions[1].get_type() == Type::Fee, 
                              TransactionBrokenGroup)?;
                }
            },

            // Check fees, other types and values for the rest if merge
            Type::Merge => {
                let fee_check = (transactions.len() == 3) || (
                    (transactions.len() == 4) && 
                    (transactions[3].get_type() == Type::Fee)
                );

                validate!(fee_check, TransactionBrokenGroup)?;

                let type_check = 
                    (transactions[1].get_type() == Type::Merge) && 
                    (transactions[2].get_type() == Type::Merge);

                validate!(type_check, TransactionBrokenGroup)?;

                let order0 = transactions[0].get_order(state, schema);
                let order1 = transactions[1].get_order(state, schema);
                let order2 = transactions[2].get_order(state, schema);

                let order_check = (order1 + 1 == order0) && 
                                  (order2 + 1 == order0);

                validate!(order_check, TransactionBrokenGroup)?;
            },

            // Check the rest fees if transfer
            Type::Transfer => {
                if transactions.len() > 1 {
                    validate!(transactions.len() == 2, TransactionBrokenGroup)?;
                    validate!(transactions[1].get_type() == Type::Fee, 
                              TransactionBrokenGroup)?;
                }
            },
        }

        Ok(())
    }
}


/// Extension for the group of transactions. It must be filled by the validator
/// in `Split` or `Merge` types. Due to the check on create, extenstion cannot  
/// be invalid.  The valid extension must have: 1) unique coins, 2) the same  
/// sender (validator), 3) consistent transaction order, types, values and 
/// count depending on the group type. Extension can be empty for `Transfer` 
/// type. Coins are supposed to be correct, extension does not check them,
/// use `Block::validate_coins` to check if necessary.
#[derive(Debug, Clone)]
pub struct Ext(Vec<Transaction>);


impl Ext {
    /// Create a new extension from transactions.
    pub fn new(transactions: Vec<Transaction>, schema: &Schema, 
               state: &State) -> Option<Self> {
        if Self::validate_transactions(&transactions, schema, state).is_ok() {
            Some(Self(transactions))
        } else {
            None
        }
    }

    /// Accessor to the inner transactions.
    pub fn transactions(&self) -> &[Transaction] {
        &self.0
    }

    /// Get type of the extension.
    pub fn get_type(&self) -> Type {
        match self.0.len() {
            0 => Type::Transfer,
            1 => Type::Merge,
            3 => Type::Split,
            _ => panic!("Invalid size of extension."),
        }
    }

    /// Get sender of the extension.
    pub fn get_sender(&self, state: &State, schema: &Schema) -> Option<U256> {
        if self.0.is_empty() {
            None
        } else {
            Some(self.0[0].get_sender(state, schema))
        }
    }

    /// Get total number of transactions.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Get order of the main coins in the extension.
    pub fn get_order(&self, state: &State, schema: &Schema) -> u64 {
        match self.0.len() {
            0 => 0,
            1 => self.0[0].get_order(state, schema),
            3 => &self.0[0].get_order(state, schema) + 1,
            _ => panic!("Invalid transactions in the group."),
        }
    }

    /// Validate transactions for the extension creation.
    pub fn validate_transactions(transactions: &[Transaction], schema: &Schema, 
                                 state: &State) -> UqoinResult<()> {
        // Check unique coins
        validate!(check_unique(transactions.iter().map(|tr| &tr.coin)), 
                  CoinNotUnique)?;

        // Check same sender
        validate!(check_same(transactions.iter()
                        .map(|tr| tr.get_sender(state, schema))), 
                  TransactionInvalidSender)?;

        // Check the size
        match transactions.len() {
            // Ok for the transfer type
            0 => {},

            // Check the type for the merge type
            1 => validate!(transactions[0].get_type() == Type::Transfer, 
                           TransactionBrokenExt)?,

            // Complex check for the split check
            3 => {
                // Get the first sender and addr
                let sender = transactions[0].get_sender(state, schema);
                let addr = &transactions[0].addr;

                // Check transfer type
                let type_check = transactions.iter()
                    .all(|tr| tr.get_type() == Type::Transfer);

                validate!(type_check, TransactionBrokenExt)?;

                // Check same sender
                let sender_check = 
                    (transactions[1].get_sender(state, schema) == sender) && 
                    (transactions[2].get_sender(state, schema) == sender);

                validate!(sender_check, TransactionBrokenExt)?;

                // Check same addr
                let addr_check = 
                    (&transactions[1].addr == addr) && 
                    (&transactions[2].addr == addr);

                validate!(addr_check, TransactionBrokenExt)?;

                // Check order
                let order0 = transactions[0].get_order(state, schema);
                let order1 = transactions[1].get_order(state, schema);
                let order2 = transactions[2].get_order(state, schema);

                let order_check = (order1 + 1 == order0) && 
                                  (order2 + 1 == order0);

                validate!(order_check, TransactionBrokenExt)?;
            },

            // Panic if the wrong size
            _ => panic!("Invalid size of extension."),
        }

        Ok(())
    }
}


/// Try to split transactions into groups and extensions. In case of not valid
/// `transactions` the iterator stops until the first error, so for the
/// validation purpose check the total size of yielded groups and extensions.
pub fn group_transactions(mut transactions: Vec<Transaction>, schema: &Schema, 
                          state: &State) -> impl Iterator<Item = (Group, Ext)> {
    std::iter::from_fn(move || {
        if let Some(group) = Group::from_vec(&mut transactions, schema, state) {
            let ext_size = group.ext_size();
            let ext_trs = vec_split_left(&mut transactions, ext_size);

            if let Some(ext) = Ext::new(ext_trs, schema, state) {
                Some((group, ext))
            } else {
                None
            }
        } else {
            None
        }
    })
}
