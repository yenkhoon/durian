use error::Error;
use log::debug;
use primitive_types::{H256, U256};
use provider::Provider;
use std::collections::HashMap;
use address::Address;

#[derive(Debug, Clone, PartialEq)]
struct AccountInfo {
    nonce: U256,
    balance: U256,
    code: Vec<u8>,
    storage: HashMap<H256, (H256, bool)>,
}

impl AccountInfo {
    pub fn new(nonce: U256, balance: U256, code: Vec<u8>) -> AccountInfo {
        AccountInfo {
            nonce,
            balance,
            code,
            storage: HashMap::new(),
        }
    }
}

pub struct State<'a> {
    provider: &'a mut dyn Provider,
    accounts: HashMap<Address, (AccountInfo, bool)>,
}

impl<'a> State<'a> {
    pub fn new(provider: &'a mut dyn Provider) -> Self {
        State {
            provider: provider,
            accounts: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn nonce(&mut self, address: &Address) -> Result<U256, Error> {
        let acc = self.account(address)?;
        Ok(acc.nonce)
    }

    pub fn balance(&mut self, address: &Address) -> Result<U256, Error> {
        let acc = self.account(address)?;
        Ok(acc.balance)
    }

    #[allow(dead_code)]
    pub fn exist(&self, address: &Address) -> bool {
        self.provider.exist(address)
    }

    pub fn timestamp(&self) -> u64 {
        self.provider.block_number()
    }

    pub fn block_number(&self) -> u64 {
        self.provider.block_number()
    }

    pub fn block_hash(&self, block_no: u64) -> Result<H256, Error> {
        self.provider.block_hash(block_no)
    }

    pub fn block_author(&self) -> Result<Address, Error> {
        self.provider.block_author()
    }

    pub fn difficulty(&self) -> Result<U256, Error> {
        self.provider.difficulty()
    }

    pub fn gas_limit(&self) -> Result<U256, Error> {
        self.provider.gas_limit()
    }

    pub fn storage_at(&mut self, address: &Address, key: &H256) -> Result<H256, Error> {
        // From parity ethereum
        // If storage root is empty RLP, then early return zero value. Practically, this makes it so that if
        // `original_storage_cache` is used, then `storage_cache` will always remain empty.

        self.fetch_storage(address, key)?;

        let acc = self.account(address)?;

        if let Some(v) = acc.storage.get(key) {
            Ok(v.0)
        } else {
            Ok(H256::zero())
        }
    }

    pub fn set_storage(&mut self, address: &Address, key: &H256, value: &H256) {
        let acc = self.account_mut(address).unwrap();
        acc.0.storage.insert(*key, (*value, true));
    }

    fn account_mut(&mut self, address: &Address) -> Result<&mut (AccountInfo, bool), Error> {
        self.fetch_account(address)?;

        return Ok(self.accounts.get_mut(address).unwrap());
    }

    fn account(&mut self, address: &Address) -> Result<&AccountInfo, Error> {
        self.fetch_account(address)?;

        return Ok(&self.accounts.get(address).unwrap().0);
    }

    pub fn init_code(&mut self, address: &Address, code: Vec<u8>) {
        let mut acc = self.account_mut(address).unwrap();
        acc.0.code = code;
        acc.1 = true;
    }

    pub fn update_state(&mut self) -> Result<(), Error> {
        for (addr, acc) in &self.accounts {
            if acc.1 {
                if !self.provider.exist(addr) {
                    self.provider.create_contract(addr, &acc.0.code)?;
                } else {
                    self.provider
                        .update_account(addr, &acc.0.balance, &acc.0.nonce)?;
                }
            }

            for (key, val) in &acc.0.storage {
                if val.1 {
                    self.provider.set_storage(addr, key, &val.0)?;
                }
            }
        }

        Ok(())
    }

    fn fetch_account(&mut self, address: &Address) -> Result<(), Error> {
        if self.accounts.contains_key(address) {
            return Ok(());
        }

        if let Ok(acc) = self.provider.account(address) {
            let acc = AccountInfo::new(acc.balance, acc.nonce, acc.code);
            self.accounts.insert(*address, (acc, false));
            Ok(())
        } else {
            let acc = AccountInfo::new(U256::zero(), U256::from(1), vec![]);
            self.accounts.insert(*address, (acc, false));
            Ok(())
        }
    }

    fn fetch_storage(&mut self, address: &Address, key: &H256) -> Result<(), Error> {
        let acc = self.account(address)?;
        if acc.storage.contains_key(key) {
            return Ok(());
        }

        if let Ok(value) = self.provider.storage_at(address, key) {
            let acc = self.account_mut(address)?;
            acc.0.storage.insert(*key, (value, false));
            Ok(())
        } else {
            debug!("Not storage at {:?}", key);
            Ok(())
        }
    }
}
