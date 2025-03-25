use std::mem;
use std::hash::Hash;
use std::collections::HashSet;

use sha3::{Sha3_256, Digest};
use finitelib::prelude::*;

use crate::errors::UqoinError;


/// 256-bit unsigned integer datatype
pub type U256 = bigi_of_bits!(256);

/// Ring for 256-bit unsigned integers that supports +, -, *, /.
pub type R256 = bigi_ring_for_bigi!(U256);

/// Alias for ECDSA signature that is a pair of U256.
pub type Signature = (U256, U256);

/// Result to manage Uqoin errors.
pub type UqoinResult<T> = Result<T, UqoinError>;


/// Get SHA3 hash of a slice of U256.
pub fn hash_of_u256<'a, I: Iterator<Item = &'a U256>>(elems: I) -> U256 {
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


/// Cut first elements from a vector.
pub fn vec_split_left<T>(v: &mut Vec<T>, ix: usize) -> Vec<T> {
    let mut u = v.split_off(ix);
    mem::swap(v, &mut u);
    u
}


/// Check unique values.
pub fn check_unique<T: Eq + Hash, I: Iterator<Item = T>>(it: I) -> bool {
    let mut set = HashSet::<T>::new();
    for elem in it {
        if set.contains(&elem) {
            return false;
        }
        set.insert(elem);
    }
    true
}


/// Check if all values are the same.
pub fn check_same<T: PartialEq, I: Iterator<Item = T>>(it: I) -> bool {
    let mut value: Option<T> = None;
    for elem in it {
        if value.is_none() {
            value = Some(elem);
        } else if value != Some(elem) {
            return false;
        }
    }
    true
}
