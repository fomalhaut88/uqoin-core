use tokio::io::{Result as TokioResult, ErrorKind};
use tokio::sync::Mutex;
use lbasedb::col::Col;
use lbasedb::path_concat;

use crate::utils::*;
use crate::transaction::Transaction;
use crate::block::Block;


/// Basic blockchain information: transactions and blocks.
pub struct Blockchain {
    transaction_col: Mutex<Col<Transaction>>,
    block_col: Mutex<Col<Block>>,
}


impl Blockchain {
    /// Create a blockchain instance.
    pub async fn new(path: &str) -> TokioResult<Self> {
        let transaction_col = Mutex::new(Col::<Transaction>::new(
            path_concat!(path, "transactions.col")
        ).await?);
        let block_col = Mutex::new(Col::<Block>::new(
            path_concat!(path, "blocks.col")
        ).await?);
        Ok(Self { transaction_col, block_col })
    }

    /// Check blockchain is empty.
    pub async fn is_empty(&self) -> TokioResult<bool> {
        let count = self.get_block_count().await?;
        Ok(count == 0)
    }

    /// Get block count.
    pub async fn get_block_count(&self) -> TokioResult<usize> {
        self.block_col.lock().await.size().await
    }

    /// Get transaction count.
    pub async fn get_transaction_count(&self) -> TokioResult<usize> {
        self.transaction_col.lock().await.size().await
    }

    /// Get block by number.
    pub async fn get_block(&self, bix: usize) -> TokioResult<Block> {
        self.block_col.lock().await.get(bix - 1).await
    }

    /// Get transaction by number.
    pub async fn get_transaction(&self, tix: usize) -> 
                                 TokioResult<Transaction> {
        self.transaction_col.lock().await.get(tix - 1).await
    }

    /// Get last block.
    pub async fn get_last_block(&self) -> TokioResult<Block> {
        let bix = self.get_block_count().await?;
        if bix > 0 {
            self.get_block(bix).await
        } else {
            Err(ErrorKind::NotFound.into())
        }
    }

    /// Get transactions of a block by number.
    pub async fn get_transactions_of_block(&self, bix: usize) -> 
                                           TokioResult<Vec<Transaction>> {
        let block = self.get_block(bix).await?;
        self.transaction_col.lock().await
            .get_many(block.tix as usize - 1, block.size as usize).await
    }

    /// Push new block with transactions. The function returns the number of 
    /// the inserted block.
    pub async fn push_new_block(&self, transactions: &[Transaction], 
                                hash_prev: &U256, validator: &U256, 
                                nonce: &U256, hash: &U256) -> 
                                TokioResult<usize> {
        let tix = self.transaction_col.lock().await
            .push_many(transactions).await? + 1;
        let block = Block::new(tix as u64, transactions.len() as u64,
                               hash_prev.clone(), validator.clone(), 
                               nonce.clone(), hash.clone());
        let bix = self.block_col.lock().await.push(&block).await? + 1;
        Ok(bix)
    }

    /// Truncate the blockchain until the necessary block count.
    pub async fn truncate(&self, block_count: usize) -> TokioResult<()> {
        if block_count > 0 {
            let block = self.get_block(block_count).await?;
            let transaction_count = (block.tix + block.size) as usize - 1;
            self.block_col.lock().await.resize(block_count).await?;
            self.transaction_col.lock().await.resize(transaction_count).await?;
        } else {
            self.block_col.lock().await.resize(0).await?;
            self.transaction_col.lock().await.resize(0).await?;
        }
        Ok(())
    }
}
