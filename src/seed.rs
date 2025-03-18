use rand::Rng;
use rand::distr::{Distribution, StandardUniform};
use bip39::{Mnemonic as Bip39Mnemonic, Language};
use finitelib::group::Group;

use crate::utils::*;
use crate::schema::Schema;


/// Type for the mnemonic phrase that is 12 English words.
pub type Mnemonic = [String; 12];


/// 128-bit seed object that can be represented as 256-bit value and 12-words
/// mnemonic phrase (according to BIP39). Seed may generate wallet sequence.
pub struct Seed(Bip39Mnemonic);


impl Seed {
    /// Generate a random seed.
    pub fn random<R: Rng>(rng: &mut R) -> Self {
        rng.random()
    }

    /// Create seed from value (only first 128-bit matter).
    pub fn from_value(value: &U256) -> Self {
        let entropy: [u8; 16] = value.to_bytes()[..16].try_into().unwrap();
        Self::from_entropy(&entropy)
    }

    /// Create seed from mnemonic phrase.
    pub fn from_mnemonic(mnemonic: &Mnemonic) -> Self {
        let phrase = mnemonic.join(" ");
        let bip93_mnemonic = Bip39Mnemonic::parse_normalized(&phrase).unwrap();
        Self(bip93_mnemonic)
    }

    /// Get value of the seed.
    pub fn value(&self) -> U256 {
        // TODO: Maybe I need a different way to generate 256-bit of the seed.
        let entropy: [u8; 16] = self.0.to_entropy().try_into().unwrap();
        u128::from_ne_bytes(entropy).into()
    }

    /// Get the mnemonic phrase.
    pub fn mnemonic(&self) -> Mnemonic {
        // Take 12 words only
        self.0.words().take(12).map(|w| w.to_string())
            .collect::<Vec<String>>().try_into().unwrap()
    }

    /// Iterate a sequence of private keys for wallets generated from the seed.
    /// Since the iterator is infinite, use it as `seed.gen_wallet_keys.take(...)`
    /// or `seed.gen_wallet_keys.nth(...)`. The wallet keys are always the same
    /// for same seed.
    pub fn gen_wallet_keys(&self, schema: &Schema) -> impl Iterator<Item = U256> {
        let curve = schema.curve();
        let value = self.value();
        let mut s = curve.generator.clone();
        std::iter::from_fn(move || {
            curve.mul_scalar_assign(&mut s, value.bit_iter());
            let key = schema.point_to_number(&s);
            Some(key)
        })
    }

    fn from_entropy(entropy: &[u8; 16]) -> Self {
        // 128-bit (16 bytes) entropy for exactly 12 words
        let bip93_mnemonic = Bip39Mnemonic
            ::from_entropy_in(Language::English, entropy).unwrap();
        Self(bip93_mnemonic)
    }
}


impl Distribution<Seed> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Seed {
        let entropy: [u8; 16] = rng.random();
        Seed::from_entropy(&entropy)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed() {
        let schema = Schema::new();
        let mut rng = rand::rng();

        let seed: Seed = rng.random();
        let mnemonic = seed.mnemonic();
        let value = seed.value();

        let seed_from_value = Seed::from_value(&value);
        assert_eq!(seed_from_value.value(), value);
        assert_eq!(seed_from_value.mnemonic(), mnemonic);
        assert_eq!(seed_from_value.gen_wallet_keys(&schema).nth(3),
                   seed.gen_wallet_keys(&schema).nth(3));

        let seed_from_mnemonic = Seed::from_mnemonic(&mnemonic);
        assert_eq!(seed_from_mnemonic.value(), value);
        assert_eq!(seed_from_mnemonic.mnemonic(), mnemonic);
        assert_eq!(seed_from_mnemonic.gen_wallet_keys(&schema).nth(3),
                   seed.gen_wallet_keys(&schema).nth(3));
    }
}
