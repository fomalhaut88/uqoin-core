//! The `utils` module in the `uqoin-core` library provides essential utility 
//! functions and type aliases that support the core operations of the Uqoin 
//! cryptocurrency protocol. These utilities facilitate tasks such as hashing, 
//! vector manipulation, and validation checks, ensuring efficient and reliable 
//! functionality throughout the system.

use std::mem;
use std::hash::Hash;
use std::collections::HashSet;

use sha3::{Sha3_256, Digest};
use finitelib::prelude::*;


/// A 256-bit unsigned integer type, fundamental for representing large 
/// numerical values in cryptographic computations.
pub type U256 = bigi_of_bits!(256);

/// A ring structure over U256, supporting arithmetic operations like addition, 
/// subtraction, multiplication, and division.
pub type R256 = bigi_ring_for_bigi!(U256);

/// A tuple of two U256 values, representing an ECDSA signature.
pub type Signature = (U256, U256);

/// A result type alias for handling errors specific to the Uqoin protocol.
pub type UqoinResult<T> = Result<T, crate::error::Error>;


/// Computes the SHA3-256 hash of an iterator over U256 elements.
pub fn hash_of_u256<'a, I: Iterator<Item = &'a U256>>(elems: I) -> U256 {
    let mut hasher = Sha3_256::new();
    for elem in elems {
        hasher.update(elem.to_bytes());
    }
    let bytes = hasher.finalize();
    U256::from_bytes(&bytes)
}


/// Splits a vector at a specified index, returning the left portion and 
/// modifying the original vector to contain the right portion.
pub fn vec_split_left<T>(v: &mut Vec<T>, ix: usize) -> Vec<T> {
    let mut u = v.split_off(ix);
    mem::swap(v, &mut u);
    u
}


/// Checks whether all elements in an iterator are unique.
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


/// Determines if all elements in an iterator are equal.
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


#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;
    use rand::Rng;

    #[test]
    fn test_hash_of_u256() {
        let values = vec![U256::from(1), U256::from(2), U256::from(3)];
        let hash = hash_of_u256(values.iter());
        assert_eq!(hash, U256::from_hex(
            "E2C3BEC9B8260DAFA4C239753BC9C53919BE9390A8378C28CBCA238516DACFCA"
        ));
    }

    #[test]
    fn test_vec_split_left() {
        let mut vec = vec![1, 2, 3, 4, 5];
        let left = vec_split_left(&mut vec, 2);
        assert_eq!(left, vec![1, 2]);
        assert_eq!(vec, vec![3, 4, 5]);
    }

    #[test]
    fn test_check_unique() {
        assert!(check_unique([1, 2, 3, 4, 5].iter()));
        assert!(!check_unique([1, 2, 3, 2, 5].iter()));
        assert!(check_unique(std::iter::empty::<i32>()));
    }

    #[test]
    fn test_check_same() {
        assert!(check_same([42, 42, 42].iter()));
        assert!(!check_same([42, 43, 42].iter()));
        assert!(check_same(std::iter::empty::<i32>()));
    }

    #[bench]
    fn bench_hash_of_u256_1(bencher: &mut Bencher) {
        let mut rng = rand::rng();
        let arr: [U256; 1] = rng.random();

        bencher.iter(|| {
            let _hash = hash_of_u256(arr.iter());
        });
    }

    #[bench]
    fn bench_hash_of_u256_10(bencher: &mut Bencher) {
        let mut rng = rand::rng();
        let arr: [U256; 10] = rng.random();

        bencher.iter(|| {
            let _hash = hash_of_u256(arr.iter());
        });
    }

    #[bench]
    fn bench_hash_of_u256_100(bencher: &mut Bencher) {
        let mut rng = rand::rng();
        let arr: [U256; 100] = rng.random();

        bencher.iter(|| {
            let _hash = hash_of_u256(arr.iter());
        });
    }
}
