use rand::Rng;

use crate::utils::*;


/// Check if the coin is valid. This means first 128 bit must be the same
/// as block_hash XOR miner.
pub fn coin_is_valid(coin: &U256, block_hash_prev: &U256, 
                     miner: &U256) -> bool {
    (coin.as_array()[0] == 
        miner.as_array()[0] ^ block_hash_prev.as_array()[0]) &&
    (coin.as_array()[1] == 
        miner.as_array()[1] ^ block_hash_prev.as_array()[1])
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


/// Get coin value.
pub fn coin_value(order: u64) -> U256 {
    &U256::from(1) << order as usize
}


/// Generate random coin.
pub fn coin_random<R: Rng>(rng: &mut R, block_hash_prev: &U256, 
                           miner: &U256) -> U256 {
    // Empty coin
    let mut coin = U256::from(0);

    // Random 128-bit tail
    coin.as_array_mut()[2] = rng.random::<u64>();
    coin.as_array_mut()[3] = rng.random::<u64>();

    // Head is XOR of miner and block_hash
    coin.as_array_mut()[0] = 
        miner.as_array()[0] ^ block_hash_prev.as_array()[0];
    coin.as_array_mut()[1] = 
        miner.as_array()[1] ^ block_hash_prev.as_array()[1];
    
    // Return coin
    coin
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
            "AA813FCF71922F189450D4227C921DB4F2A814209B53610902737FBF0182EBBC"
        );
        let block_hash_prev = U256::from_hex(
            "0000001B6C3C729B1BD5A34486AD423E2EE3EBE7DEAE316A71FEE1AFBED3D9B8"
        );
        let miner = U256::from_hex(
            "E7646626CB303A9EEBAAD078ACD56328DC4BFFC745FD5063738D9E10BF513204"
        );

        assert_eq!(
            hash_of_u256([&coin, &block_hash_prev, &miner].into_iter()).to_hex(), 
            "00000DE62F61A94997E66136A71E9881B87FFB970CA73051B8E5C3137012F1B7"
        );

        assert_eq!(coin_is_valid(&coin, &block_hash_prev, &miner), true);

        let order = coin_order(&coin, &block_hash_prev, &miner);

        assert_eq!(order, 20);
        assert_eq!(coin_symbol(order), "C1");
        assert_eq!(coin_value(order), &U256::from(1) << 20);
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
            |coin| coin_is_valid(&coin, &block_hash_prev, &miner)
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
