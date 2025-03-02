#![feature(test)]

extern crate test;

pub mod utils;
pub mod hash;
pub mod edwards;
pub mod crypto;
pub mod coin;
pub mod transaction;


#[cfg(test)]
mod tests {
    // use super::*;

    #[test]
    fn test() {}
}
