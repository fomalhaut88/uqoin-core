//! Provides functionality for generating and handling mnemonic phrases and 
//! seeds, facilitating deterministic key generation for wallets.
//!
//! This module utilizes the BIP-39 standard to create 12-word English mnemonic 
//! phrases and derive corresponding 128-bit seeds. These seeds can 
//! deterministically generate sequences of cryptographic keys compatible with
//! the Uqoin protocol.
//!
//! Note: While this implementation follows BIP-39, it is not a formal part of 
//! the Uqoin specification and should be considered a recommended approach.

use rand::Rng;
use rand::distr::{Distribution, StandardUniform};
use bip39::{Mnemonic as Bip39Mnemonic, Language};
use finitelib::group::Group;

use crate::utils::*;
use crate::schema::Schema;


/// Represents a 12-word English mnemonic phrase used for seed generation.
pub type Mnemonic = [String; 12];


/// Encapsulates a 128-bit seed derived from a BIP-39 mnemonic phrase.
/// Provides methods for seed creation, retrieval, and key generation.
pub struct Seed(Bip39Mnemonic);


impl Seed {
    /// Generates a new random seed using the provided random number generator.
    pub fn random<R: Rng>(rng: &mut R) -> Self {
        rng.random()
    }

    /// Creates a seed from a given 256-bit value, utilizing only the first 128
    /// bits.
    pub fn from_value(value: &U256) -> Self {
        let entropy: [u8; 16] = value.to_bytes()[..16].try_into().unwrap();
        Self::from_entropy(&entropy)
    }

    /// Constructs a seed from a provided 12-word mnemonic phrase.
    pub fn from_mnemonic(mnemonic: &Mnemonic) -> Self {
        let phrase = mnemonic.join(" ");
        let bip93_mnemonic = Bip39Mnemonic::parse_normalized(&phrase).unwrap();
        Self(bip93_mnemonic)
    }

    /// Retrieves the 128-bit seed value as a `U256` type.
    pub fn value(&self) -> U256 {
        // TODO: Maybe I need a different way to generate 256-bit of the seed.
        let entropy: [u8; 16] = self.0.to_entropy().try_into().unwrap();
        u128::from_ne_bytes(entropy).into()
    }

    /// Returns the 12-word mnemonic phrase associated with the seed.
    pub fn mnemonic(&self) -> Mnemonic {
        // Take 12 words only
        self.0.words().take(12).map(|w| w.to_string())
            .collect::<Vec<String>>().try_into().unwrap()
    }

    /// Generates an infinite, deterministic sequence of private keys from the
    /// seed.
    ///
    /// Each key is uniquely derived from the seed and a sequential index,
    /// allowing reproducible generation of multiple keys from a single seed.
    /// This method is ideal for creating hierarchical deterministic (HD) 
    /// wallets or for generating predictable test key sets.
    ///
    /// Since the iterator is infinite, it is recommended to combine it with 
    /// methods like `.take(count)` if you need a fixed number of keys
    pub fn gen_keys(&self, schema: &Schema) -> impl Iterator<Item = U256> {
        let curve = schema.curve();
        let value = self.value();
        let mut j = curve.generator.clone();
        std::iter::from_fn(move || {
            curve.mul_scalar_assign(&mut j, value.bit_iter());
            let p = curve.convert_from(&j);
            let key = &schema.point_to_number(&p) % &curve.base.order;
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
        assert_eq!(seed_from_value.gen_keys(&schema).nth(3),
                   seed.gen_keys(&schema).nth(3));

        let seed_from_mnemonic = Seed::from_mnemonic(&mnemonic);
        assert_eq!(seed_from_mnemonic.value(), value);
        assert_eq!(seed_from_mnemonic.mnemonic(), mnemonic);
        assert_eq!(seed_from_mnemonic.gen_keys(&schema).nth(3),
                   seed.gen_keys(&schema).nth(3));
    }
}
