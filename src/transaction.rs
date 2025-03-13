use rand::Rng;

use crate::utils::*;
use crate::crypto::Schema;
use crate::coin::CoinMap;


/// Types of transaction or group. In case of group Fee must be incorrect.
#[derive(PartialEq)]
pub enum Type {
    Transfer,
    Fee,
    Split,
    Merge,
}


/// Transaction base structure.
#[derive(Clone)]
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
                         schema: &Schema) -> Self {
        let hash = Self::calc_msg(&coin, &addr);
        let (sign_r, sign_s) = schema.build_signature(rng, &hash, key);
        Self::new(coin, addr, sign_r, sign_s)
    }

    /// Prepare transactions for transfer.
    pub fn prepare_transfer<R: Rng>(rng: &mut R, key: &U256, coin: &U256, 
                                    addr: &U256, fee_coins: &[U256],
                                    schema: &Schema) -> Vec<Self> {
        let mut res = vec![
            Self::build(rng, coin.clone(), addr.clone(), key, schema)
        ];
        res.extend(fee_coins.iter().map(|fee_coin| 
            Self::build(rng, fee_coin.clone(), U256::from(0), key, schema)
        ));
        res
    }

    /// Prepare transactions for split.
    pub fn prepare_split<R: Rng>(rng: &mut R, key: &U256, coin: &U256, 
                                 fee_coins: &[U256],
                                 schema: &Schema) -> Vec<Self> {
        let mut res = vec![
            Self::build(rng, coin.clone(), U256::from(1), key, schema)
        ];
        res.extend(fee_coins.iter().map(|fee_coin| 
            Self::build(rng, fee_coin.clone(), U256::from(0), key, schema)
        ));
        res
    }

    /// Prepare transactions for merge.
    pub fn prepare_merge<R: Rng>(rng: &mut R, key: &U256, coins: [&U256; 3], 
                                 fee_coins: &[U256],
                                 schema: &Schema) -> Vec<Self> {
        let mut res = vec![
            Self::build(rng, coins[0].clone(), U256::from(2), key, schema),
            Self::build(rng, coins[1].clone(), U256::from(2), key, schema),
            Self::build(rng, coins[2].clone(), U256::from(2), key, schema),
        ];
        res.extend(fee_coins.iter().map(|fee_coin| 
            Self::build(rng, fee_coin.clone(), U256::from(0), key, schema)
        ));
        res
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
    pub fn get_msg(&self) -> U256 {
        Self::calc_msg(&self.coin, &self.addr)
    }

    /// Get transaction hash.
    pub fn get_hash(&self) -> U256 {
        hash_of_u256(
            [&self.coin, &self.addr, &self.sign_r, &self.sign_s].into_iter()
        )
    }

    /// Get transaction sender.
    pub fn get_sender(&self, schema: &Schema) -> U256 {
        schema.extract_public(
            &self.get_msg(), 
            &(self.sign_r.clone(), self.sign_s.clone())
        )
    }

    /// Get order of the coin.
    pub fn get_order(&self, coin_map: &CoinMap) -> u64 {
        coin_map[&self.coin].order()
    }

    /// Get value of the coin.
    pub fn get_value(&self, coin_map: &CoinMap) -> U256 {
        coin_map[&self.coin].value()
    }

    /// Check signature with the given public key.
    pub fn check(&self, public: &U256, 
                 schema: &Schema) -> bool {
        schema.check_signature(
            &self.get_msg(), public, 
            &(self.sign_r.clone(), self.sign_s.clone())
        )
    }

    /// Calculate transaction message as hash of the `coin` and `addr`.
    pub fn calc_msg(coin: &U256, addr: &U256) -> U256 {
        hash_of_u256([coin, addr].into_iter())
    }
}


/// Group of transactions.
pub struct Group(Vec<Transaction>);


impl Group {
    /// Create group from transactions. Validation is included, so if the
    /// vector is not valid, `None` will be returned.
    pub fn new(transactions: Vec<Transaction>, schema: &Schema, 
               coin_map: &CoinMap) -> Option<Self> {
        if Self::validate_transactions(&transactions, schema, coin_map) {
            Some(Self(transactions))
        } else {
            None
        }
    }

    /// Try to create a group from the leading transactions in the given slice.
    /// Fees are joined by the greedy approach.
    pub fn from_vec(transactions: &mut Vec<Transaction>, schema: &Schema, 
                    coin_map: &CoinMap) -> Option<Self> {
        if transactions.is_empty() {
            // `None` if the slice is empty
            None
        } else {
            // Index where fee transactions start
            let fee_ix = match transactions[0].get_type() {
                Type::Split => 1,
                Type::Merge => 3,
                Type::Transfer => 1,
                _ => 0,
            };

            if fee_ix == 0 {
                // `None` if we start from a fee transaction
                None
            } else {
                // We set `size` to `fee_ix` and increment it until the slice 
                // ends or a non-fee transaction happens.
                let mut size = fee_ix;

                while (size < transactions.len()) && 
                      (transactions[size].get_type() == Type::Fee) {
                    size += 1;
                }

                // Try to create a group using validation in `Self::new`
                let trs = vec_split_left(transactions, size);
                Self::new(trs, schema, coin_map)
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
    pub fn get_sender(&self, schema: &Schema) -> U256 {
        self.0[0].get_sender(schema)
    }

    /// Get total number of transactions.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Get order of the main coins.
    pub fn get_order(&self, coin_map: &CoinMap) -> u64 {
        match self.get_type() {
            Type::Split => self.0[0].get_order(coin_map),
            Type::Merge => self.0[0].get_order(coin_map) + 1,
            Type::Transfer => self.0[0].get_order(coin_map),
            _ => panic!("Invalid transactions in the group."),
        }
    }

    /// Get total value of the group.
    pub fn get_value(&self, coin_map: &CoinMap) -> U256 {
        match self.get_type() {
            Type::Split => self.0[0].get_value(coin_map),
            Type::Merge => &self.0[0].get_value(coin_map) << 1,
            Type::Transfer => self.0[0].get_value(coin_map),
            _ => panic!("Invalid transactions in the group."),
        }
    }

    /// Get total fee of the group.
    pub fn get_fee(&self, coin_map: &CoinMap) -> U256 {
        let fee_ix = match self.get_type() {
            Type::Split => 1,
            Type::Merge => 3,
            Type::Transfer => 1,
            _ => panic!("Invalid transactions in the group."),
        };
        self.0[fee_ix..].iter().map(|tr| tr.get_value(coin_map))
            .fold(U256::from(0), |s, v| &s + &v)
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
                                 coin_map: &CoinMap) -> bool {
        if transactions.is_empty() {
            // False if no transactions in the slice
            false
        } else {
            // Get the first sender
            let sender = transactions[0].get_sender(schema);

            // Check same sender
            let is_same_sender = transactions.iter()
                .all(|tr| tr.get_sender(schema) == sender);
            
            if is_same_sender {
                // Check the first type
                match transactions[0].get_type() {
                    // False if the first transaction is fee
                    Type::Fee => false,

                    // Check the rest fees if split
                    Type::Split => transactions[1..].iter()
                        .all(|tr| tr.get_type() == Type::Fee),

                    // Check fees, other types and values for the rest if merge
                    Type::Merge => {
                        let fee_check = transactions[3..].iter()
                            .all(|tr| tr.get_type() == Type::Fee);

                        let type_check = 
                            (transactions[1].get_type() == Type::Merge) && 
                            (transactions[2].get_type() == Type::Merge);

                        let order0 = transactions[0].get_order(coin_map);
                        let order1 = transactions[1].get_order(coin_map);
                        let order2 = transactions[2].get_order(coin_map);

                        let order_check = (order1 + 1 == order0) && 
                                          (order2 + 1 == order0);

                        fee_check && type_check && order_check
                    },

                    // Check the rest fees if transfer
                    Type::Transfer => transactions[1..].iter()
                        .all(|tr| tr.get_type() == Type::Fee),
                }
            } else {
                // False if senders differ
                false
            }
        }
    }
}


/// Extension for the group of transactions. It must be filled by the validator
/// in split or merge types.
pub struct Ext(Vec<Transaction>);


impl Ext {
    /// Create a new extension from transactions.
    pub fn new(transactions: Vec<Transaction>, schema: &Schema, 
               coin_map: &CoinMap) -> Option<Self> {
        if Self::validate_transactions(&transactions, schema, coin_map) {
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
    pub fn get_sender(&self, schema: &Schema) -> Option<U256> {
        if self.0.is_empty() {
            None
        } else {
            Some(self.0[0].get_sender(schema))
        }
    }

    /// Get total number of transactions.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Get order of the main coins in the extension.
    pub fn get_order(&self, coin_map: &CoinMap) -> u64 {
        match self.0.len() {
            0 => 0,
            1 => self.0[0].get_order(coin_map),
            3 => &self.0[0].get_order(coin_map) + 1,
            _ => panic!("Invalid transactions in the group."),
        }
    }

    /// Get total value of the extension.
    pub fn get_value(&self, coin_map: &CoinMap) -> U256 {
        match self.0.len() {
            0 => U256::from(0),
            1 => self.0[0].get_value(coin_map),
            3 => &self.0[0].get_value(coin_map) << 1,
            _ => panic!("Invalid transactions in the group."),
        }
    }

    /// Validate transactions for the extension creation.
    pub fn validate_transactions(transactions: &[Transaction], schema: &Schema, 
                                 coin_map: &CoinMap) -> bool {
        // Check the size
        match transactions.len() {
            // `true` for the transfer type
            0 => true,

            // Check the type for the merge type
            1 => transactions[0].get_type() == Type::Transfer,

            // Complex check for the split check
            3 => {
                // Get the first sender and addr
                let sender = transactions[0].get_sender(schema);
                let addr = &transactions[0].addr;

                // Check transfer type
                let type_check = transactions.iter()
                    .all(|tr| tr.get_type() == Type::Transfer);

                // Check same sender
                let sender_check = 
                    (transactions[1].get_sender(schema) == sender) && 
                    (transactions[2].get_sender(schema) == sender);

                // Check same addr
                let addr_check = 
                    (&transactions[1].addr == addr) && 
                    (&transactions[2].addr == addr);

                // Check order
                let order0 = transactions[0].get_order(coin_map);
                let order1 = transactions[1].get_order(coin_map);
                let order2 = transactions[2].get_order(coin_map);

                let order_check = (order1 + 1 == order0) && 
                                  (order2 + 1 == order0);

                type_check && sender_check && addr_check && order_check
            },

            // Panic if the wrong size
            _ => panic!("Invalid size of extension."),
        }
    }
}


/// Try to split transactions into groups and extensions. In case of not valid
/// `transactions` the iterator stops until the first error, so for the
/// validation purpose check the total size of yielded groups and extensions.
pub fn group_transactions(mut transactions: Vec<Transaction>, schema: &Schema, 
                          coin_map: &CoinMap) -> 
                          impl Iterator<Item = (Group, Ext)> {
    std::iter::from_fn(move || {
        if let Some(group) = Group::from_vec(&mut transactions, schema, 
                                             coin_map) {
            let ext_size = group.ext_size();
            let ext_trs = vec_split_left(&mut transactions, ext_size);

            if let Some(ext) = Ext::new(ext_trs, schema, coin_map) {
                Some((group, ext))
            } else {
                None
            }
        } else {
            None
        }
    })
}
