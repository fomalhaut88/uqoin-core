use rand::Rng;

use crate::validate;
use crate::utils::*;


/// Check if the coin is valid. This means first 128 bit must be the same
/// as block_hash XOR miner.
pub fn coin_validate(coin: &U256, miner: &U256) -> UqoinResult<()> {
    validate!(coin.as_array()[2..4] == miner.as_array()[2..4], CoinInvalid)
}


/// Get coin order.
pub fn coin_order(coin: &U256, miner: &U256) -> u64 {
    let hash = hash_of_u256([coin, miner].into_iter());
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


/// Mine coins. The function returns an infinite iterator.
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
