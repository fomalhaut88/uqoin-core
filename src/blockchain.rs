//! Blockchain is driver to store blocks and transactions on disk. It is done
//! with the help of `Lbasedb`. It supports the structures and raw bytes on
//! extranct and update.

//! Provides asynchronous storage and retrieval of blocks and transactions in
//! the Uqoin blockchain.
//!
//! This module uses `Lbasedb` to persist blockchain data on disk,
//! supporting structured access to individual transactions, blocks, and raw 
//! bytes. It enables adding new blocks, reading block and transaction history,
//! and low-level updates of serialized blockchain data.

use tokio::io::{Result as TokioResult, ErrorKind};
use tokio::sync::Mutex;
use lbasedb::col::Col;
use lbasedb::path_concat;

use crate::transaction::Transaction;
use crate::block::{Block, BlockInfo, BlockData};


/// A driver for storing and retrieving blocks and transactions on disk.
///
/// `Blockchain` uses `Lbasedb` columns internally and ensures thread-safe 
/// asynchronous access. It supports structured access to blocks and 
/// transactions, as well as raw byte-level operations for advanced use cases.
pub struct Blockchain {
    transaction_col: Mutex<Col<Transaction>>,
    block_col: Mutex<Col<Block>>,
}


impl Blockchain {
    /// Creates a new blockchain instance by opening transaction and block 
    /// storage at the given path.
    pub async fn new(path: &str) -> TokioResult<Self> {
        let transaction_col = Mutex::new(Col::<Transaction>::new(
            path_concat!(path, "transactions.col")
        ).await?);
        let block_col = Mutex::new(Col::<Block>::new(
            path_concat!(path, "blocks.col")
        ).await?);
        Ok(Self { transaction_col, block_col })
    }

    /// Checks whether the blockchain contains any blocks.
    pub async fn is_empty(&self) -> TokioResult<bool> {
        let count = self.get_block_count().await?;
        Ok(count == 0)
    }

    /// Retrieves the total number of blocks stored in the blockchain.
    pub async fn get_block_count(&self) -> TokioResult<u64> {
        let size = self.block_col.lock().await.size().await?;
        Ok(size as u64)
    }

    /// Retrieves the total number of transactions stored in the blockchain.
    pub async fn get_transaction_count(&self) -> TokioResult<u64> {
        let size = self.transaction_col.lock().await.size().await?;
        Ok(size as u64)
    }

    /// Retrieves a block by its index (1-based).
    pub async fn get_block(&self, bix: u64) -> TokioResult<Block> {
        if bix == 0 {
            Err(ErrorKind::NotFound.into())
        } else {
            self.block_col.lock().await.get(bix as usize - 1).await
        }
    }

    /// Retrieves a transaction by its index (1-based).
    pub async fn get_transaction(&self, tix: u64) -> 
                                 TokioResult<Transaction> {
        if tix == 0 {
            Err(ErrorKind::NotFound.into())
        } else {
            self.transaction_col.lock().await.get(tix as usize - 1).await
        }
    }

    /// Retrieves basic information (`BlockInfo`) about a block by its index
    /// (1-based). 
    pub async fn get_block_info(&self, bix: u64) -> TokioResult<BlockInfo> {
        if bix == 0 {
            Ok(BlockInfo::genesis())
        } else {
            let block = self.get_block(bix).await?;
            Ok(BlockInfo {
                bix,
                offset: block.offset + block.size,
                hash: block.hash,
            })
        }
    }

    /// Retrieves the full `BlockData`, including the block and its associated
    /// transactions  by its index (1-based).
    pub async fn get_block_data(&self, bix: u64) -> TokioResult<BlockData> {
        if bix == 0 {
            Ok(BlockData::genesis())
        } else {
            let block = self.get_block(bix).await?;
            let transactions = self.get_transactions_of_block(&block).await?;
            Ok(BlockData { bix, block, transactions })
        }
    }

    /// Retrieves multiple consecutive `BlockData` entries starting from a given
    /// block index (1-based).
    pub async fn get_block_data_many(&self, bix: u64, count: u64) -> 
                                     TokioResult<Vec<BlockData>> {
        if (bix > 0) && (count > 0) {
            // Get all blocks
            let blocks: Vec<Block> = self.block_col.lock().await
                .get_many((bix - 1) as usize, count as usize).await?;

            // Calculate transaction offset and count
            let block_first = blocks.first().unwrap();
            let block_last = blocks.last().unwrap();
            let transaction_offset = block_first.offset;
            let transaction_count = 
                block_last.offset + block_last.size - transaction_offset;

            // Get all transactions
            let transactions: Vec<Transaction> = self.transaction_col
                .lock().await.get_many(transaction_offset as usize, 
                                       transaction_count as usize).await?;

            // Gather block data vector
            Ok(blocks.into_iter().enumerate().map(|(ix, block)| {
                let trs = &transactions[
                    (block.offset - transaction_offset) as usize
                    ..
                    (block.offset - transaction_offset + block.size) as usize
                ];
                BlockData {
                    bix: bix + ix as u64,
                    block,
                    transactions: trs.to_vec(),
                }
            }).collect())
        } else {
            Err(ErrorKind::NotFound.into())
        }
    }

    /// Retrieves the last (most recent) block stored in the blockchain.
    pub async fn get_last_block(&self) -> TokioResult<Block> {
        let bix = self.get_block_count().await?;
        self.get_block(bix).await
    }

    /// Retrieves all transactions associated with a specific block.
    pub async fn get_transactions_of_block(&self, block: &Block) -> 
                                           TokioResult<Vec<Transaction>> {
        self.transaction_col.lock().await
            .get_many(block.offset as usize, block.size as usize).await
    }

    /// Pushes a new block along with its associated transactions into the
    /// blockchain. It returns the 1-based block number (`bix`) of the inserted
    /// block.
    pub async fn push_new_block(&self, block: &Block,
                                transactions: &[Transaction]) -> 
                                TokioResult<u64> {
        self.transaction_col.lock().await.update_many(block.offset as usize, 
                                                      transactions).await?;
        let bix = self.block_col.lock().await.push(&block).await? as u64 + 1;
        Ok(bix)
    }

    /// Truncates the blockchain to retain only a specified number of blocks.
    pub async fn truncate(&self, block_count: u64) -> TokioResult<()> {
        if block_count > 0 {
            let block = self.get_block(block_count).await?;
            let transaction_count = block.offset + block.size;
            self.block_col.lock().await.resize(block_count as usize).await?;
            self.transaction_col.lock().await
                .resize(transaction_count as usize).await?;
        } else {
            self.block_col.lock().await.resize(0).await?;
            self.transaction_col.lock().await.resize(0).await?;
        }
        Ok(())
    }

    /// Retrieves multiple consecutive blocks by offset and count.
    pub async fn get_block_many(&self, offset: usize, 
                                count: usize) -> TokioResult<Vec<Block>> {
        self.block_col.lock().await.get_many(offset, count).await
    }

    /// Retrieves multiple consecutive transactions by offset and count.
    pub async fn get_transaction_many(&self, offset: usize, 
                                      count: usize) -> 
                                      TokioResult<Vec<Transaction>> {
        self.transaction_col.lock().await.get_many(offset, count).await
    }

    /// Retrieves the raw serialized bytes of a range of blocks.
    pub async fn get_block_raw(&self, offset: usize, 
                               count: usize) -> TokioResult<Vec<u8>> {
        self.block_col.lock().await.get_raw(offset, count).await
    }

    /// Retrieves the raw serialized bytes of a range of transactions.
    pub async fn get_transaction_raw(&self, offset: usize, 
                                     count: usize) -> TokioResult<Vec<u8>> {
        self.transaction_col.lock().await.get_raw(offset, count).await
    }

    /// Updates the raw serialized bytes of blocks starting at the given offset.
    pub async fn update_block_raw(&self, offset: usize, 
                                  bytes: &[u8]) -> TokioResult<()> {
        self.block_col.lock().await.update_raw(offset, bytes).await
    }

    /// Updates the raw serialized bytes of transactions starting at the given
    /// offset.
    pub async fn update_transaction_raw(&self, offset: usize, 
                                        bytes: &[u8]) -> TokioResult<()> {
        self.transaction_col.lock().await.update_raw(offset, bytes).await
    }
}
