use std::{collections::HashMap, sync::RwLock};

use anyhow::Result;
use gw_common::{smt::SMT, H256};
use gw_db::{
    error::Error,
    schema::{
        Col, COLUMNS, COLUMN_ACCOUNT_SMT_BRANCH, COLUMN_ACCOUNT_SMT_LEAF, COLUMN_BLOCK,
        COLUMN_META, META_TIP_BLOCK_HASH_KEY,
    },
    RocksDBSnapshot,
};
use gw_types::{
    packed::{self, L2Block},
    prelude::{Entity, FromSliceShouldBeOk, Pack, Reader, Unpack},
};

use crate::{smt::smt_store::SMTStore, state::mem_state_db::MemStateTree, traits::KVStore};

pub const META_MEM_BLOCK_INFO: &[u8] = b"MEM_BLOCK_INFO";
/// account SMT root
pub const META_MEM_SMT_ROOT_KEY: &[u8] = b"MEM_ACCOUNT_SMT_ROOT_KEY";
/// account SMT count
pub const META_MEM_SMT_COUNT_KEY: &[u8] = b"MEM_ACCOUNT_SMT_COUNT_KEY";

enum Value<T> {
    Exist(T),
    Deleted,
}

type MemColumn = HashMap<Vec<u8>, Value<Vec<u8>>>;

pub struct StoreSnapshot {
    inner: RocksDBSnapshot,
    mem: Vec<RwLock<MemColumn>>,
}

impl StoreSnapshot {
    pub(crate) fn new(inner: RocksDBSnapshot) -> Self {
        let mut mem = Vec::with_capacity(COLUMNS as usize);
        mem.resize_with(COLUMNS as usize, || RwLock::new(HashMap::default()));

        Self { inner, mem }
    }

    pub fn state(&self) -> Result<MemStateTree<'_>> {
        let merkle_root = {
            let block = self.get_tip_block()?;
            block.raw().post_account()
        };
        let root = self
            .get_mem_block_account_smt_root()?
            .unwrap_or_else(|| merkle_root.merkle_root().unpack());
        let account_count = self
            .get_mem_block_account_count()?
            .unwrap_or_else(|| merkle_root.count().unpack());
        let mem_smt_store = SMTStore::new(COLUMN_ACCOUNT_SMT_LEAF, COLUMN_ACCOUNT_SMT_BRANCH, self);
        let tree = SMT::new(root, mem_smt_store);
        Ok(MemStateTree::new(tree, account_count))
    }

    pub fn get_tip_block_hash(&self) -> Result<H256, Error> {
        let slice = self
            .get(COLUMN_META, META_TIP_BLOCK_HASH_KEY)
            .expect("get tip block hash");
        Ok(
            packed::Byte32Reader::from_slice_should_be_ok(slice.as_ref())
                .to_entity()
                .unpack(),
        )
    }

    pub fn get_tip_block(&self) -> Result<L2Block, Error> {
        let tip_block_hash = self.get_tip_block_hash()?;
        Ok(self.get_block(&tip_block_hash)?.expect("get tip block"))
    }

    pub fn get_mem_block_account_smt_root(&self) -> Result<Option<H256>, Error> {
        match self.get(COLUMN_META, META_MEM_SMT_ROOT_KEY) {
            Some(slice) => {
                debug_assert_eq!(slice.len(), 32);
                let mut root = [0u8; 32];
                root.copy_from_slice(&slice);
                Ok(Some(root.into()))
            }
            None => Ok(None),
        }
    }

    pub fn get_mem_block_account_count(&self) -> Result<Option<u32>, Error> {
        match self.get(COLUMN_META, META_MEM_SMT_COUNT_KEY) {
            Some(slice) => {
                let count =
                    packed::Uint32Reader::from_slice_should_be_ok(slice.as_ref()).to_entity();
                Ok(Some(count.unpack()))
            }
            None => Ok(None),
        }
    }

    pub fn get_block(&self, block_hash: &H256) -> Result<Option<L2Block>, Error> {
        match self.get(COLUMN_BLOCK, block_hash.as_slice()) {
            Some(slice) => Ok(Some(
                packed::L2BlockReader::from_slice_should_be_ok(slice.as_ref()).to_entity(),
            )),
            None => Ok(None),
        }
    }

    pub fn set_mem_block_account_smt_root(&self, root: H256) -> Result<(), Error> {
        self.insert_raw(COLUMN_META, META_MEM_SMT_ROOT_KEY, root.as_slice())?;
        Ok(())
    }

    pub fn set_mem_block_account_count(&self, count: u32) -> Result<(), Error> {
        let count: packed::Uint32 = count.pack();
        self.insert_raw(COLUMN_META, META_MEM_SMT_COUNT_KEY, count.as_slice())
            .expect("insert");
        Ok(())
    }

    pub fn update_mem_pool_block_info(&self, block_info: &packed::BlockInfo) -> Result<(), Error> {
        self.insert_raw(COLUMN_META, META_MEM_BLOCK_INFO, block_info.as_slice())
            .expect("insert");
        Ok(())
    }

    pub fn get_mem_pool_block_info(&self) -> Result<Option<packed::BlockInfo>, Error> {
        match self.get(COLUMN_META, META_MEM_BLOCK_INFO) {
            Some(slice) => Ok(Some(
                packed::BlockInfoReader::from_slice_should_be_ok(slice.as_ref()).to_entity(),
            )),
            None => Ok(None),
        }
    }
}

impl KVStore for StoreSnapshot {
    fn get(&self, col: Col, key: &[u8]) -> Option<Box<[u8]>> {
        match self
            .mem
            .get(col as usize)
            .expect("can't found column")
            .read()
            .expect("get read lock failed")
            .get(key)
        {
            Some(Value::Exist(v)) => Some(v.clone().into_boxed_slice()),
            Some(Value::Deleted) => None,
            None => self
                .inner
                .get_pinned(col, key)
                .expect("db operation should be ok")
                .map(|v| Box::<[u8]>::from(v.as_ref())),
        }
    }

    fn insert_raw(&self, col: Col, key: &[u8], value: &[u8]) -> Result<(), Error> {
        self.mem
            .get(col as usize)
            .expect("can't found column")
            .write()
            .expect("get write lock failed")
            .insert(key.to_vec(), Value::Exist(value.to_vec()));
        Ok(())
    }

    fn delete(&self, col: Col, key: &[u8]) -> Result<(), Error> {
        self.mem
            .get(col as usize)
            .expect("can't found column")
            .write()
            .expect("get write lock failed")
            .insert(key.to_vec(), Value::Deleted);
        Ok(())
    }
}