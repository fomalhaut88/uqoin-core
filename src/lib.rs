//! # uqoin-core
//!
//! Core algorithms that implement Uqoin protocol in Rust.
//! 
//! Uqoin is an NFT-based cryptocurrency where money is reprecented as unique and
//! non-fungible coins having denominations as powers of two. There is a number of
//! engineer ideas that make extremely high performance regarding other modern
//! cryptocurrencies.

#![feature(test)]

extern crate test;

pub mod utils;
pub mod error;
pub mod edwards;
pub mod schema;
pub mod coin;
pub mod transaction;
pub mod block;
pub mod state;
pub mod pool;
pub mod seed;

#[cfg(feature = "blockchain")]
pub mod blockchain;
