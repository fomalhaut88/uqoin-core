use crate::utils::*;


/// Basic structure for block.
pub struct Block {
    pub num: u64,
    pub ix: u64,
    pub size: u64,
    pub validator: U256,
    pub nonce: U256,
    pub hash: U256,
}
