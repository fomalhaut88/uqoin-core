//! Coin structure.

use std::collections::HashMap;

use rand::Rng;

use crate::utils::*;


/// Map of coins number-coin
pub type CoinMap = HashMap<U256, Coin>;


/// Coin structure that keeps the numberm hash, block_hash and order.
pub struct Coin {
    number: U256,
    hash: U256,
    block_hash: U256,
    miner: U256,  
    order: u64,
}


impl Coin {
    /// Create a new coin having the number and the block_hash.
    pub fn new(number: U256, block_hash: U256, miner: U256) -> Self {
        let hash = Self::calc_hash(&number, &block_hash, &miner);
        let order = Self::calc_order(&hash);
        Self { number, hash, block_hash, miner, order }
    }

    /// Check if the coin is valid. This means first 128 bit must be the same
    /// as block_hash.
    pub fn is_valid(&self) -> bool {
        (self.number.as_array()[0] == 
            self.miner.as_array()[0] ^ self.block_hash.as_array()[0]) &&
        (self.number.as_array()[1] == 
            self.miner.as_array()[1] ^ self.block_hash.as_array()[1])
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

    /// Miner address of the coin.
    pub fn miner(&self) -> &U256 {
        &self.miner
    }

    /// Power of the denomination level, log2 of value.
    pub fn order(&self) -> u64 {
        self.order
    }

    /// Denomination level of the coin. It is a power of 2, for example,
    /// order = 11 means 2048 units.
    pub fn value(&self) -> U256 {
        &U256::from(1) << (self.order as usize)
    }

    /// Symbol of the coin value.
    pub fn symbol(&self) -> String {
        let letter: char = ('A' as u8 + (self.order / 10) as u8) as char;
        let number: u32 = 1 << (self.order % 10);
        format!("{}{}", letter, number)
    }

    /// Geterate a random coin having the given block hash.
    pub fn gen_random<R: Rng>(rng: &mut R, block_hash: &U256, 
                              miner: &U256) -> Self {
        // Empty number
        let mut number = U256::from(0);

        // Random 128-bit tail
        number.as_array_mut()[2] = rng.random::<u64>();
        number.as_array_mut()[3] = rng.random::<u64>();

        // Head is XOR of miner and block_hash
        number.as_array_mut()[0] = 
            miner.as_array()[0] ^ block_hash.as_array()[0];
        number.as_array_mut()[1] = 
            miner.as_array()[1] ^ block_hash.as_array()[1];
        
        // Return coin
        Self::new(number, block_hash.clone(), miner.clone())
    }

    /// Mine iterator for the given block hash filteging by `min_value`.
    /// It uses one thread.
    pub fn mine<R: Rng>(rng: &mut R, block_hash: &U256, miner: &U256,
                        min_order: u64) -> impl Iterator<Item = Self> {
        std::iter::repeat(1)
            .map(|_| Self::gen_random(rng, block_hash, miner))
            .filter(move |coin| coin.order >= min_order)
    }

    /// Calculate coin hash from the number and block hash.
    pub fn calc_hash(number: &U256, block_hash: &U256, miner: &U256) -> U256 {
        hash_of_u256([number, block_hash, miner].into_iter())
    }

    /// Calculate coin order from its hash.
    pub fn calc_order(hash: &U256) -> u64 {
        256 - hash.bit_len() as u64
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
            "AA813FCF71922F189450D4227C921DB4F2A814209B53610902737FBF0182EBBC"
        );
        let block_hash = U256::from_hex(
            "0000001B6C3C729B1BD5A34486AD423E2EE3EBE7DEAE316A71FEE1AFBED3D9B8"
        );
        let miner = U256::from_hex(
            "E7646626CB303A9EEBAAD078ACD56328DC4BFFC745FD5063738D9E10BF513204"
        );

        let coin = Coin::new(number, block_hash, miner);

        assert_eq!(coin.is_valid(), true);
        assert_eq!(coin.symbol(), "C1");
        assert_eq!(coin.value(), &U256::from(1) << 20);
        assert_eq!(
            coin.hash().to_hex(), 
            "00000DE62F61A94997E66136A71E9881B87FFB970CA73051B8E5C3137012F1B7"
        );
        assert_eq!(
            coin.to_string(), 
            "C1 [AA813FCF71922F189450D4227C921DB4F2A814209B53610902737FBF0182EBBC]"
        );
    }

    #[test]
    fn test_mine() {
        let block_hash = U256::from_hex(
            "0000001B6C3C729B1BD5A34486AD423E2EE3EBE7DEAE316A71FEE1AFBED3D9B8"
        );
        let miner = U256::from_hex(
            "E7646626CB303A9EEBAAD078ACD56328DC4BFFC745FD5063738D9E10BF513204"
        );

        let mut rng = rand::rng();

        let coins = Coin::mine(&mut rng, &block_hash, &miner, 10)
            .take(3).collect::<Vec<Coin>>();

        assert!(coins.iter().all(|coin| coin.is_valid()));
        assert!(coins.iter().all(|coin| coin.value() >= &U256::from(1) << 10));
    }

    #[bench]
    fn bench_gen_random(bencher: &mut Bencher) {
        let block_hash = U256::from_hex(
            "0000001B6C3C729B1BD5A34486AD423E2EE3EBE7DEAE316A71FEE1AFBED3D9B8"
        );
        let miner = U256::from_hex(
            "E7646626CB303A9EEBAAD078ACD56328DC4BFFC745FD5063738D9E10BF513204"
        );
        let mut rng = rand::rng();
        bencher.iter(|| {
            let _coin = Coin::gen_random(&mut rng, &block_hash, &miner);
        });
    }

    #[bench]
    fn bench_mine_10(bencher: &mut Bencher) {
        let block_hash = U256::from_hex(
            "0000001B6C3C729B1BD5A34486AD423E2EE3EBE7DEAE316A71FEE1AFBED3D9B8"
        );
        let miner = U256::from_hex(
            "E7646626CB303A9EEBAAD078ACD56328DC4BFFC745FD5063738D9E10BF513204"
        );
        let mut rng = rand::rng();
        let mut it = Coin::mine(&mut rng, &block_hash, &miner, 10);
        bencher.iter(|| {
            let _coin = it.next();
        });
    }
}
