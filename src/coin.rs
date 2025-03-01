//! Coin structure.

use rand::Rng;
use sha3::{Sha3_256, Digest};

use crate::utils::U256;


// pub struct Coin {
//     pub num: U256,
//     pub own: U256,
//     pub den: u32,
// }


// impl Coin {
//     pub fn new(num: U256, own: U256, den: u32) -> Self {
//         Self { num, own, den }
//     }
// }


// pub struct NewCoin {
//     pub number: U256,
//     pub miner: U256,
//     pub block_hash: U256,
//     pub denomination: u32,
//     pub split: bool,
// }


// impl NewCoin {
//     pub fn new(number: U256, miner: U256, block_hash: U256) -> Self {
//         let coin_hash = Self::calc_hash(&number, &miner, &block_hash);
//         let denomination = Self::calc_denomination(&coin_hash);
//         let split = Self::calc_split(&coin_hash);
//         Self { number, miner, block_hash, denomination, split}
//     }

//     pub fn get_hash(&self) -> U256 {
//         Self::calc_hash(&self.number, &self.miner, &self.block_hash)
//     }

//     pub fn is_valid(&self) -> bool {
//         let coin_hash = self.get_hash();
//         (self.denomination == Self::calc_denomination(&coin_hash)) && 
//             (self.split == Self::calc_split(&coin_hash))
//     }

//     pub fn calc_hash(number: &U256, miner: &U256, block_hash: &U256) -> U256 {
//         let mut hasher = Sha3_256::new();
//         hasher.update(number.to_bytes());
//         hasher.update(miner.to_bytes());
//         hasher.update(block_hash.to_bytes());
//         let coin_hash = hasher.finalize();
//         U256::from_bytes(&coin_hash)
//     }

//     pub fn calc_denomination(coin_hash: &U256) -> u32 {
//         256 - coin_hash.bit_len() as u32
//     }

//     pub fn calc_split(coin_hash: &U256) -> bool {
//         coin_hash.bit_get(0)
//     }
// }


pub fn coin_get_hash(coin: &U256, miner: &U256, block_hash: &U256) -> U256 {
    let mut hasher = Sha3_256::new();
    hasher.update(coin.to_bytes());
    hasher.update(miner.to_bytes());
    hasher.update(block_hash.to_bytes());
    let coin_hash = hasher.finalize();
    U256::from_bytes(&coin_hash)
}


pub fn coin_get_denomination(coin_hash: &U256) -> u32 {
    256 - coin_hash.bit_len() as u32
}


pub fn coin_get_name(den: u32) -> String {
    let letter: char = ('A' as u8 + (den / 10) as u8) as char;
    let number: u32 = 1 << (den % 10);
    format!("{}{}", letter, number)
}


pub fn coin_mine<R: Rng>(rng: &mut R, miner: &U256, block_hash: &U256, den_low: u32) -> (U256, u32) {
    loop {
        let coin: U256 = rng.random();
        let coin_hash = coin_get_hash(&coin, &miner, &block_hash);
        let den = coin_get_denomination(&coin_hash);
        if den >= den_low {
            return (coin, den);
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;

    #[test]
    fn test_coin_hash() {
        let coin = U256::from_hex("6D788362D32CB65D518B301C0685E1F9BAA7DBCD935DA3F150D629CAE12D8E3B");
        let miner = U256::from_hex("F4341BD686F0F4985B6A74EF5CA1C1DC7C8880C7DF7F4CDABF917084D058C3BC");
        let block_hash = U256::from_hex("59475E1B6C3C729B1BD5A34486AD423E2EE3EBE7DEAE316A71FEE1AFBED3D9B8");

        let coin_hash = coin_get_hash(&coin, &miner, &block_hash);
        let den = coin_get_denomination(&coin_hash);

        assert_eq!(coin_hash.to_hex(), "0000D026076CAF7D890376B0270DAB9FB0E70AB9F96926234C211B0D315B0B32".to_string());
        assert_eq!(den, 16);
    }

    #[test]
    fn test_coin_mine() {
        let mut rng = rand::rng();

        let miner = U256::from_hex("F4341BD686F0F4985B6A74EF5CA1C1DC7C8880C7DF7F4CDABF917084D058C3BC");
        let block_hash = U256::from_hex("59475E1B6C3C729B1BD5A34486AD423E2EE3EBE7DEAE316A71FEE1AFBED3D9B8");

        let (coin, den) = coin_mine(&mut rng, &miner, &block_hash, 12);

        assert!(den >= 12);
    }

    #[test]
    fn test_coin_get_name() {
        assert_eq!(coin_get_name(35), "D32");
        assert_eq!(coin_get_name(74), "H16");
        assert_eq!(coin_get_name(130), "N1");
    }

    #[bench]
    fn bench_coin_hash(bencher: &mut Bencher) {
        let coin = U256::from_hex("6D788362D32CB65D518B301C0685E1F9BAA7DBCD935DA3F150D629CAE12D8E3B");
        let miner = U256::from_hex("F4341BD686F0F4985B6A74EF5CA1C1DC7C8880C7DF7F4CDABF917084D058C3BC");
        let block_hash = U256::from_hex("59475E1B6C3C729B1BD5A34486AD423E2EE3EBE7DEAE316A71FEE1AFBED3D9B8");

        bencher.iter(|| {
            let _coin_hash = coin_get_hash(&coin, &miner, &block_hash);
        });
    }

    #[bench]
    fn bench_coin_den(bencher: &mut Bencher) {
        let coin_hash = U256::from_hex("0000D026076CAF7D890376B0270DAB9FB0E70AB9F96926234C211B0D315B0B32");

        bencher.iter(|| {
            let _den = coin_get_denomination(&coin_hash);
        });
    }

    #[bench]
    fn bench_coin_mine(bencher: &mut Bencher) {
        let mut rng = rand::rng();

        let miner = U256::from_hex("F4341BD686F0F4985B6A74EF5CA1C1DC7C8880C7DF7F4CDABF917084D058C3BC");
        let block_hash = U256::from_hex("59475E1B6C3C729B1BD5A34486AD423E2EE3EBE7DEAE316A71FEE1AFBED3D9B8");

        bencher.iter(|| {
            let (_coin, _den) = coin_mine(&mut rng, &miner, &block_hash, 8);
        });
    }
}
