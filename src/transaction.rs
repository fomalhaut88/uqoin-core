use rand::Rng;

use crate::utils::*;
use crate::hash::*;
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
                                    addr: &U256, fee: Option<&U256>,
                                    schema: &Schema) -> Vec<Self> {
        let mut res = vec![
            Self::build(rng, coin.clone(), addr.clone(), key, schema)
        ];
        if let Some(fee_coin) = fee {
            res.push(
                Self::build(rng, fee_coin.clone(), U256::from(0), key, schema)
            )
        }
        res
    }

    /// Prepare transactions for split.
    pub fn prepare_split<R: Rng>(rng: &mut R, key: &U256, coin: &U256, 
                                 fee: Option<&U256>,
                                 schema: &Schema) -> Vec<Self> {
        let mut res = vec![
            Self::build(rng, coin.clone(), U256::from(1), key, schema)
        ];
        if let Some(fee_coin) = fee {
            res.push(
                Self::build(rng, fee_coin.clone(), U256::from(0), key, schema)
            )
        }
        res
    }

    /// Prepare transactions for merge.
    pub fn prepare_merge<R: Rng>(rng: &mut R, key: &U256, coins: [&U256; 3], 
                                 fee: Option<&U256>,
                                 schema: &Schema) -> Vec<Self> {
        let mut res = vec![
            Self::build(rng, coins[0].clone(), U256::from(2), key, schema),
            Self::build(rng, coins[1].clone(), U256::from(2), key, schema),
            Self::build(rng, coins[2].clone(), U256::from(2), key, schema),
        ];
        if let Some(fee_coin) = fee {
            res.push(
                Self::build(rng, fee_coin.clone(), U256::from(0), key, schema)
            )
        }
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
        hash_of_u256(&[&self.coin, &self.addr, &self.sign_r, &self.sign_s])
    }

    /// Get transaction sender.
    pub fn get_sender(&self, schema: &Schema) -> U256 {
        schema.extract_public(
            &self.get_msg(), 
            &(self.sign_r.clone(), self.sign_s.clone())
        )
    }

    /// Get value of the coin.
    pub fn get_value(&self, coin_map: &CoinMap) -> u64 {
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
        hash_of_u256(&[coin, addr])
    }
}


/// Group of transactions structure.
pub struct Group(Vec<Transaction>);


impl Group {
    pub fn new(transactions: Vec<Transaction>) -> Self {
        Self(transactions)
    }

    pub fn transactions(&self) -> &[Transaction] {
        &self.0
    }

    pub fn get_type(&self) -> Option<Type> {
        if self.0.is_empty() {
            None
        } else {
            match self.0[0].get_type() {
                Type::Fee => None,
                type_ => Some(type_),
            }
        }
    }

    pub fn get_sender(&self, schema: &Schema) -> Option<U256> {
        if self.0.is_empty() {
            None
        } else {
            match self.0[0].get_type() {
                Type::Fee => None,
                _ => {
                    let sender = self.0[0].get_sender(schema);
                    Some(sender)
                },
            }
        }
    }

    pub fn get_validator(&self) -> Option<U256> {
        None
    }

    pub fn is_ready(&self) -> bool {
        match self.fee_transactions_iter() {
            Some(mut it) => it.all(|tr| tr.get_type() == Type::Fee),
            None => false,
        }
    }

    pub fn is_complete(&self) -> bool {
        true
    }

    pub fn get_fee(&self, coin_map: &CoinMap) -> Option<u64> {
        Some(self.fee_transactions_iter()?
                .map(|tr| tr.get_value(coin_map))
                .sum())
    }

    pub fn complete(&mut self, _transactions: &[Transaction]) {}

    pub fn add_fee(&mut self, _transactions: &[Transaction]) {}

    fn fee_transactions_iter(&self) -> Option<impl Iterator<Item = &Transaction>> {
        // TODO: validator transactions are not considered.
        let fee_ix = match self.get_type()? {
            Type::Transfer => 1,
            Type::Split => 1,
            Type::Merge => 3,
            Type::Fee => 0,  // Not used because Fee cannot be a group type
        };
        Some(self.0[fee_ix..].iter())
    }
}
