//! Provides utilities for representing, validating, and mining coins in the 
//! Uqoin protocol.
//!
//! In Uqoin, a coin is a 256-bit unsigned integer (`U256`) with specific 
//! structural properties:
//!
//! - The last 128 bits of the coin must match the last 128 bits of the miner's
//! address.
//! - The "order" of a coin is defined by the number of leading zeros in the
//! hash of the coin and miner address.
//! - The coin's value is `2^order`, making higher-order coins more valuable and
//! harder to mine.
//!
//! This module includes functions for coin validation, order and value
//! computation, symbol conversion, random coin generation, and mining.


use rand::Rng;

use crate::validate;
use crate::utils::*;


/// Validates a coin by ensuring its last 128 bits match those of the miner's 
/// address.
pub fn coin_validate(coin: &U256, miner: &U256) -> UqoinResult<()> {
    validate!(coin.as_array()[2..4] == miner.as_array()[2..4], CoinInvalid)
}


/// Calculates the order of a coin based on the number of leading zeros in the 
/// hash of the coin and miner address.
pub fn coin_order(coin: &U256, miner: &U256) -> u64 {
    let hash = hash_of_u256([coin, miner].into_iter());
    256 - hash.bit_len() as u64
}


/// Converts a coin's order into a symbolic representation (e.g., "C32").
pub fn coin_symbol(order: u64) -> String {
    let letter: char = ('A' as u8 + (order / 10) as u8) as char;
    let number: u32 = 1 << (order % 10);
    format!("{}{}", letter, number)
}


/// Parses a coin's symbolic representation to retrieve its order.
pub fn coin_order_by_symbol(symbol: &str) -> u64 {
    let letter = symbol.chars().next().unwrap() as u64 - 'A' as u64;
    let number: u64 = symbol[1..].parse().unwrap();
    10 * letter + number.trailing_zeros() as u64
}


/// Calculates the value of a coin based on its order.
pub fn coin_value(order: u64) -> U256 {
    &U256::from(1) << order as usize
}


/// Generates a random coin with a valid structure for a given miner.
pub fn coin_random<R: Rng>(rng: &mut R, miner: &U256) -> U256 {
    // Empty coin
    let mut coin = U256::from(0);

    // Random 128-bit head
    coin.as_array_mut()[0..2].clone_from_slice(
        &rng.random::<[u64; 2]>()
    );

    // Tail is XOR of miner and block_hash
    coin.as_array_mut()[2..4].clone_from_slice(&miner.as_array()[2..4]);
    
    // Return coin
    coin
}


/// Returns an infinite iterator that yields valid coins meeting a minimum order
/// requirement.
pub fn coin_mine<R: Rng>(rng: &mut R, miner: &U256,
                         min_order: u64) -> impl Iterator<Item = U256> {
    std::iter::repeat(1)
        .map(|_| coin_random(rng, miner))
        .filter(
            move |coin| coin_order(coin, miner) >= min_order
        )
}


#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;

    #[test]
    fn test_coin() {
        let coin = U256::from_hex(
            "E7646626CB303A9EEBAAD078ACD5632862232A27EF6426CC7D7A92251FBFEE94"
        );
        let miner = U256::from_hex(
            "E7646626CB303A9EEBAAD078ACD56328DC4BFFC745FD5063738D9E10BF513204"
        );

        assert_eq!(
            hash_of_u256([&coin, &miner].into_iter()).to_hex(), 
            "0000001462535B76AFA05824673FA8A3AEDC030B7D3BB354B1A7463191134609"
        );

        assert!(coin_validate(&coin, &miner).is_ok());

        let order = coin_order(&coin, &miner);

        assert_eq!(order, 27);
        assert_eq!(coin_symbol(order), "C128");
        assert_eq!(coin_value(order), &U256::from(1) << 27);
    }

    #[test]
    fn test_coin_order_by_symbol() {
        assert_eq!(coin_order_by_symbol("C32"), 25);
        assert_eq!(coin_order_by_symbol("D4"), 32);
        assert_eq!(coin_order_by_symbol("B1"), 10);
        assert_eq!(coin_order_by_symbol("A1"), 0);
        assert_eq!(coin_order_by_symbol("Z32"), 255);
    }

    #[test]
    fn test_mine() {
        let miner = U256::from_hex(
            "E7646626CB303A9EEBAAD078ACD56328DC4BFFC745FD5063738D9E10BF513204"
        );

        let mut rng = rand::rng();

        let coins = coin_mine(&mut rng, &miner, 10)
            .take(3).collect::<Vec<U256>>();

        assert!(coins.iter().all(
            |coin| coin_validate(&coin, &miner).is_ok()
        ));
        assert!(coins.iter().all(
            |coin| coin_order(&coin, &miner) >= 10
        ));
    }

    #[bench]
    fn bench_gen_random(bencher: &mut Bencher) {
        let miner = U256::from_hex(
            "E7646626CB303A9EEBAAD078ACD56328DC4BFFC745FD5063738D9E10BF513204"
        );
        let mut rng = rand::rng();
        bencher.iter(|| {
            let _coin = coin_random(&mut rng, &miner);
        });
    }

    #[bench]
    fn bench_mine_10(bencher: &mut Bencher) {
        let miner = U256::from_hex(
            "E7646626CB303A9EEBAAD078ACD56328DC4BFFC745FD5063738D9E10BF513204"
        );
        let mut rng = rand::rng();
        let mut it = coin_mine(&mut rng, &miner, 10);
        bencher.iter(|| {
            let _coin = it.next();
        });
    }
}
