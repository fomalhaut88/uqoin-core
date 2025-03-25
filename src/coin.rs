use rand::Rng;

use crate::validate;
use crate::utils::*;


/// Check if the coin is valid. This means first 128 bit must be the same
/// as block_hash XOR miner.
#[deprecated(since="0.1.0", 
             note="please use `coin_validate(...).is_ok()` instead")]
pub fn coin_is_valid(coin: &U256, block_hash_prev: &U256, 
                     miner: &U256) -> bool {
    coin.as_array()[2..4] == coin_tail(block_hash_prev, miner)
}


/// Check if the coin is valid. This means first 128 bit must be the same
/// as block_hash XOR miner.
pub fn coin_validate(coin: &U256, block_hash_prev: &U256, 
                     miner: &U256) -> UqoinResult<()> {
    validate!(coin.as_array()[2..4] == coin_tail(block_hash_prev, miner), 
              CoinInvalid)
}


/// Get coin order.
pub fn coin_order(coin: &U256, block_hash_prev: &U256, miner: &U256) -> u64 {
    let hash = hash_of_u256([coin, block_hash_prev, miner].into_iter());
    256 - hash.bit_len() as u64
}


/// Get coin symbol.
pub fn coin_symbol(order: u64) -> String {
    let letter: char = ('A' as u8 + (order / 10) as u8) as char;
    let number: u32 = 1 << (order % 10);
    format!("{}{}", letter, number)
}


/// Get coin order by symbol.
pub fn coin_order_by_symbol(symbol: &str) -> u64 {
    let letter = symbol.chars().next().unwrap() as u64 - 'A' as u64;
    let number: u64 = symbol[1..].parse().unwrap();
    10 * letter + number.trailing_zeros() as u64
}


/// Get coin value.
pub fn coin_value(order: u64) -> U256 {
    &U256::from(1) << order as usize
}


/// Generate random coin.
pub fn coin_random<R: Rng>(rng: &mut R, block_hash_prev: &U256, 
                           miner: &U256) -> U256 {
    // Empty coin
    let mut coin = U256::from(0);

    // Random 128-bit head
    coin.as_array_mut()[0..2].clone_from_slice(
        &rng.random::<[u64; 2]>()
    );

    // Tail is XOR of miner and block_hash
    coin.as_array_mut()[2..4].clone_from_slice(
        &coin_tail(block_hash_prev, miner)
    );
    
    // Return coin
    coin
}


/// Calculate last two u64 digits (128 bits) from `block_hash_prev` and `miner`.
pub fn coin_tail(block_hash_prev: &U256, miner: &U256) -> [u64; 2] {
    [
        miner.as_array()[2] ^ block_hash_prev.as_array()[2],
        miner.as_array()[3] ^ block_hash_prev.as_array()[3],
    ]
}


/// Mine coins. The function returns an infinite iterator.
pub fn coin_mine<R: Rng>(rng: &mut R, block_hash_prev: &U256, miner: &U256,
                         min_order: u64) -> impl Iterator<Item = U256> {
    std::iter::repeat(1)
        .map(|_| coin_random(rng, block_hash_prev, miner))
        .filter(
            move |coin| coin_order(coin, block_hash_prev, miner) >= min_order
        )
}


#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;

    #[test]
    fn test_coin() {
        let coin = U256::from_hex(
            "E764663DA70C4805F07F733C2A782116C7492C70EE67DD39C5DDA817816B8AB2"
        );
        let block_hash_prev = U256::from_hex(
            "0000001B6C3C729B1BD5A34486AD423E2EE3EBE7DEAE316A71FEE1AFBED3D9B8"
        );
        let miner = U256::from_hex(
            "E7646626CB303A9EEBAAD078ACD56328DC4BFFC745FD5063738D9E10BF513204"
        );

        assert_eq!(
            hash_of_u256([&coin, &block_hash_prev, &miner].into_iter()).to_hex(), 
            "00000A20A6620E0D48C7BDD76BF8E92D0CD26AFC4AB1A39A8A5DF8D7D7103F88"
        );

        assert!(coin_validate(&coin, &block_hash_prev, &miner).is_ok());

        let order = coin_order(&coin, &block_hash_prev, &miner);

        assert_eq!(order, 20);
        assert_eq!(coin_symbol(order), "C1");
        assert_eq!(coin_value(order), &U256::from(1) << 20);
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
        let block_hash_prev = U256::from_hex(
            "0000001B6C3C729B1BD5A34486AD423E2EE3EBE7DEAE316A71FEE1AFBED3D9B8"
        );
        let miner = U256::from_hex(
            "E7646626CB303A9EEBAAD078ACD56328DC4BFFC745FD5063738D9E10BF513204"
        );

        let mut rng = rand::rng();

        let coins = coin_mine(&mut rng, &block_hash_prev, &miner, 10)
            .take(3).collect::<Vec<U256>>();

        assert!(coins.iter().all(
            |coin| coin_validate(&coin, &block_hash_prev, &miner).is_ok()
        ));
        assert!(coins.iter().all(
            |coin| coin_order(&coin, &block_hash_prev, &miner) >= 10
        ));
    }

    #[bench]
    fn bench_gen_random(bencher: &mut Bencher) {
        let block_hash_prev = U256::from_hex(
            "0000001B6C3C729B1BD5A34486AD423E2EE3EBE7DEAE316A71FEE1AFBED3D9B8"
        );
        let miner = U256::from_hex(
            "E7646626CB303A9EEBAAD078ACD56328DC4BFFC745FD5063738D9E10BF513204"
        );
        let mut rng = rand::rng();
        bencher.iter(|| {
            let _coin = coin_random(&mut rng, &block_hash_prev, &miner);
        });
    }

    #[bench]
    fn bench_mine_10(bencher: &mut Bencher) {
        let block_hash_prev = U256::from_hex(
            "0000001B6C3C729B1BD5A34486AD423E2EE3EBE7DEAE316A71FEE1AFBED3D9B8"
        );
        let miner = U256::from_hex(
            "E7646626CB303A9EEBAAD078ACD56328DC4BFFC745FD5063738D9E10BF513204"
        );
        let mut rng = rand::rng();
        let mut it = coin_mine(&mut rng, &block_hash_prev, &miner, 10);
        bencher.iter(|| {
            let _coin = it.next();
        });
    }
}
