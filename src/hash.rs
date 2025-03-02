use rand::Rng;
use sha3::{Sha3_256, Digest};
use finitelib::prelude::*;

use crate::utils::*;


/// Get SHA3 hash of a slice of U256.
pub fn hash_of_u256(elems: &[&U256]) -> U256 {
    let mut hasher = Sha3_256::new();
    for elem in elems {
        hasher.update(elem.to_bytes());
    }
    let bytes = hasher.finalize();
    U256::from_bytes(&bytes)
}


/// Get SHA3 hash of a buffer.
pub fn hash_of_buffer(buffer: &[u8]) -> U256 {
    let mut hasher = Sha3_256::new();
    hasher.update(buffer);
    let bytes = hasher.finalize();
    U256::from_bytes(&bytes)
}


/// Generate a random coin the correct prefix that corresponds to
/// the miner and block.
fn gen_coin<R: Rng>(rng: &mut R, miner: &U256, block_hash: &U256) -> U256 {
    // Prefix from miner and block_hash
    let prefix_miner = &miner.as_array()[3..];
    let prefix_block = &block_hash.as_array()[3..];

    // Suffix as random 128-bit value
    let suffix = rng.random::<Bigi<2>>();
    
    // Concatenate prefix and suffix to get a new coin
    let mut coin: U256 = (&suffix).into();
    coin.as_array_mut()[3..].clone_from_slice(prefix_miner);
    coin.as_array_mut()[2..3].clone_from_slice(prefix_block);
    coin
}


/// Check if the coin has the correct prefix for the miner and the block.
fn is_coin_valid(coin: &U256, miner: &U256, block_hash: &U256) -> bool {
    (coin.as_array()[3] == miner.as_array()[3]) &&
        (coin.as_array()[2] == block_hash.as_array()[3])
}


/// Get coin value.
fn get_coin_value(coin_hash: &U256) -> u32 {
    256 - coin_hash.bit_len() as u32
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_of_u256() {
        let coin = U256::from_hex(
            "F4341BD686F0F49859475E1B6C3C729B463A01855BE481A0F15BFE8E07091819"
        );
        let miner = U256::from_hex(
            "F4341BD686F0F4985B6A74EF5CA1C1DC7C8880C7DF7F4CDABF917084D058C3BC"
        );
        let block_hash = U256::from_hex(
            "59475E1B6C3C729B1BD5A34486AD423E2EE3EBE7DEAE316A71FEE1AFBED3D9B8"
        );

        let coin_hash = hash_of_u256(&[&coin, &miner, &block_hash]);

        assert!(is_coin_valid(&coin, &miner, &block_hash));
        assert_eq!(
            coin_hash.to_hex(), 
            "00000AA7CF7EEAC02260FA22D7CD7201D7D750D6075E550FC4C79B86078A41F0"
        );
    }
}
