//! Coin structure.

use rand::Rng;
use finitelib::prelude::*;

use crate::utils::*;
use crate::hash::*;


/// Coin structure that keeps the numberm hash, block_hash and value.
pub struct Coin {
    number: U256,
    hash: U256,
    block_hash: U256,    
    value: u32,
}


impl Coin {
    /// Create a new coin having the number and the block_hash.
    pub fn new(number: U256, block_hash: U256) -> Self {
        let hash = Self::calc_hash(&number, &block_hash);
        let value = Self::calc_value(&hash);
        Self { number, hash, block_hash, value }
    }

    /// Check if the coin is valid. This means first 128 bit must be the same
    /// as block_hash.
    pub fn is_valid(&self) -> bool {
        self.number.as_array()[2..] == self.block_hash.as_array()[2..]
    }

    /// Number of the coin.
    pub fn number(&self) -> &U256 {
        &self.number
    }

    /// Hash of the coin.
    pub fn hash(&self) -> &U256 {
        &self.hash
    }

    /// Block hash of the coin.
    pub fn block_hash(&self) -> &U256 {
        &self.block_hash
    }

    /// Denomination level of the coin. It is a power of 2, for example,
    /// value = 11 means 2048 units.
    pub fn value(&self) -> u32 {
        self.value
    }

    /// Symbol of the coin value.
    pub fn symbol(&self) -> String {
        let letter: char = ('A' as u8 + (self.value / 10) as u8) as char;
        let number: u32 = 1 << (self.value % 10);
        format!("{}{}", letter, number)
    }

    /// Geterate a random coin having the given block hash.
    pub fn gen_random<R: Rng>(rng: &mut R, block_hash: &U256) -> Self {
        // Prefix from block_hash
        let prefix = &block_hash.as_array()[2..];

        // Suffix as random 128-bit value
        let suffix = rng.random::<Bigi<2>>();
        
        // Concatenate prefix and suffix to get a new coin
        let mut number: U256 = (&suffix).into();
        number.as_array_mut()[2..].clone_from_slice(prefix);
        
        // Return coin
        Self::new(number, block_hash.clone())
    }

    /// Mine iterator for the given block hash filteging by `min_value`.
    /// It uses one thread.
    pub fn mine<R: Rng>(rng: &mut R, block_hash: &U256, 
                        min_value: u32) -> impl Iterator<Item = Self> {
        std::iter::repeat(1)
            .map(|_| Self::gen_random(rng, block_hash))
            .filter(move |coin| coin.value() >= min_value)
    }

    /// Calculate coin hash from the number and block hash.
    pub fn calc_hash(number: &U256, block_hash: &U256) -> U256 {
        hash_of_u256(&[&number, &block_hash])
    }

    /// Calculate coin value from its hash.
    pub fn calc_value(hash: &U256) -> u32 {
        256 - hash.bit_len() as u32
    }
}


impl ToString for Coin {
    fn to_string(&self) -> String {
        format!("{} [{}]", self.symbol(), self.number().to_hex())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;

    #[test]
    fn test_coin() {
        let number = U256::from_hex(
            "59475E1B6C3C729B1BD5A34486AD423EDAFFEB954E9CCC2F9654C77873F81574"
        );
        let block_hash = U256::from_hex(
            "59475E1B6C3C729B1BD5A34486AD423E2EE3EBE7DEAE316A71FEE1AFBED3D9B8"
        );

        let coin = Coin::new(number, block_hash);

        assert_eq!(coin.is_valid(), true);
        assert_eq!(coin.symbol(), "C1");
        assert_eq!(coin.value(), 20);
        assert_eq!(
            coin.hash().to_hex(), 
            "00000D49EE12DA8FCB7DD134DB2C28BE8FB1B080B40907F1DD5827AFF0F3156B"
        );
        assert_eq!(
            coin.to_string(), 
            "C1 [59475E1B6C3C729B1BD5A34486AD423EDAFFEB954E9CCC2F9654C77873F81574]"
        );
    }

    #[test]
    fn test_mine() {
        let block_hash = U256::from_hex(
            "59475E1B6C3C729B1BD5A34486AD423E2EE3EBE7DEAE316A71FEE1AFBED3D9B8"
        );

        let mut rng = rand::rng();

        let coins = Coin::mine(&mut rng, &block_hash, 10)
            .take(3).collect::<Vec<Coin>>();

        assert!(coins.iter().all(|coin| coin.is_valid()));
        assert!(coins.iter().all(|coin| coin.value() >= 10));
    }

    #[bench]
    fn bench_gen_random(bencher: &mut Bencher) {
        let block_hash = U256::from_hex(
            "59475E1B6C3C729B1BD5A34486AD423E2EE3EBE7DEAE316A71FEE1AFBED3D9B8"
        );
        let mut rng = rand::rng();
        bencher.iter(|| {
            let _coin = Coin::gen_random(&mut rng, &block_hash);
        });
    }

    #[bench]
    fn bench_mine_10(bencher: &mut Bencher) {
        let block_hash = U256::from_hex(
            "59475E1B6C3C729B1BD5A34486AD423E2EE3EBE7DEAE316A71FEE1AFBED3D9B8"
        );
        let mut rng = rand::rng();
        let mut it = Coin::mine(&mut rng, &block_hash, 10);
        bencher.iter(|| {
            let _coin = it.next();
        });
    }
}
