use rand::Rng;

use crate::utils::*;
use crate::hash::*;
use crate::crypto::Schema;


/// Types of transaction.
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

    /// Get transaction owner (sender).
    pub fn get_owner(&self, schema: &Schema) -> U256 {
        schema.extract_public(
            &self.get_msg(), 
            &(self.sign_r.clone(), self.sign_s.clone())
        )
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
