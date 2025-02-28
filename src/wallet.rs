pub struct Wallet {
    key: U256,
}


impl Wallet {
    pub fn new(key: U256) -> Self {
        Self { key }
    }

    pub fn from_seed(seed: U256, order: usize) -> Self {
        let key = Self::iter_keys_from_seed(seed).nth(order);
        Self::new(key)
    }

    pub fn addr(&self) -> U256 {
        unimplemented!("Algorithm to get address from the private key.")
    }

    pub fn create_signature(&self, hash: U256) -> (U256, U256) {
        unimplemented!("ECDSA algorithm.")
    }

    pub fn check_signature(addr: U256, signature: (U256, U256)) -> bool {
        unimplemented!("ECDSA algorithm.")
    }

    pub fn iter_keys_from_seed(seed: U256) -> impl Iterator<Item = U256> {
        unimplemented!("Algorithm to iterate keys from the seed.")
    }
}
