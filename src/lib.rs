//! # uqoin-core
//! 
//! **uqoin-core** is the foundational library for the Uqoin cryptocurrency   
//! protocol. It provides all essential components for managing coins, 
//! transactions, blocks, and blockchain state in a secure, efficient, and
//! deterministic way.
//! 
//! ---
//! 
//! ## Features
//! 
//! - **Elliptic Curve Cryptography** (Ed25519 signatures and key operations)
//! - **Deterministic Key Generation** (BIP-39 style mnemonic seeds)
//! - **Coin Structure and Mining** (unique order-based mining validation)
//! - **Transaction System** (transfer, fee, split, and merge types)
//! - **Block Management** (validation, linking, and complexity proofs)
//! - **State Management** (dynamic tracking of coin ownership and counters)
//! - **Asynchronous Storage** (disk-based persistence with `Lbasedb`)
//! - **Transaction Pool** (preparation of transactions for new blocks)
//! 
//! ---
//! 
//! ## Components
//! 
//! | Module         | Responsibility                             |
//! |:---------------|:-------------------------------------------|
//! | `utils`        | Utility functions and helpers             |
//! | `error`        | Unified error types                       |
//! | `edwards`      | Cryptographic curve operations            |
//! | `schema`       | Signature schemes and key validation      |
//! | `coin`         | Coin format, mining, and validation        |
//! | `transaction`  | Transaction types and verification         |
//! | `block`        | Block structure and hash validation        |
//! | `state`        | Real-time blockchain state management      |
//! | `pool`         | Transaction pooling before block creation |
//! | `seed`         | Mnemonic generation and deterministic keys |
//! | `blockchain`   | Persistent blockchain storage              |
//! 
//! ---
//! 
//! ## Philosophy
//! 
//! - **Minimalistic** and protocol-focused design
//! - **Deterministic** and reproducible operations
//! - **High-performance** and scalable storage
//! - **Secure** cryptographic foundations
//! 
//! ---
//! 
//! > **uqoin-core** — powering the future of simple, fair, and efficient 
//! blockchain systems.

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
