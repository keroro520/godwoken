use ckb_types::H256;
use gw_jsonrpc_types::ckb_jsonrpc_types::{Uint128, Uint32};
use std::{u128, u32};

type AccountID = Uint32;

pub struct GodwokenRpcClient {
    url: reqwest::Url,
    client: reqwest::blocking::Client,
    id: u64,
}

impl GodwokenRpcClient {
    pub fn new(url: &str) -> GodwokenRpcClient {
        let url = reqwest::Url::parse(url).expect("godwoken uri, e.g. \"http://127.0.0.1:8119\"");
        GodwokenRpcClient {
            url,
            id: 0,
            client: reqwest::blocking::Client::new(),
        }
    }
}

impl GodwokenRpcClient {
    pub fn get_tip_block_hash(&mut self) -> Result<Option<H256>, String> {
        let params = serde_json::Value::Null;
        self.rpc::<Option<H256>>("get_tip_block_hash", params)
            .map(|opt| opt.map(Into::into))
    }

    pub fn get_balance(&mut self, account_id: u32, sudt_id: u32) -> Result<u128, String> {
        let params = serde_json::to_value((AccountID::from(account_id), AccountID::from(sudt_id)))
            .map_err(|err| err.to_string())?;
        self.rpc::<Uint128>("get_balance", params).map(Into::into)
    }

    pub fn get_account_id_by_script_hash(
        &mut self,
        script_hash: H256,
    ) -> Result<Option<u32>, String> {
        let params = serde_json::to_value((script_hash,)).map_err(|err| err.to_string())?;
        self.rpc::<Option<Uint32>>("get_account_id_by_script_hash", params)
            .map(|opt| opt.map(Into::into))
    }

    fn rpc<SuccessResponse: serde::de::DeserializeOwned>(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<SuccessResponse, String> {
        self.id += 1;
        let mut req_json = serde_json::Map::new();
        req_json.insert("id".to_owned(), serde_json::to_value(&self.id).unwrap());
        req_json.insert("jsonrpc".to_owned(), serde_json::to_value(&"2.0").unwrap());
        req_json.insert("method".to_owned(), serde_json::to_value(method).unwrap());
        req_json.insert("params".to_owned(), params);

        let resp = self
            .client
            .post(self.url.clone())
            .json(&req_json)
            .send()
            .map_err(|err| err.to_string())?;
        let output = resp
            .json::<ckb_jsonrpc_types::response::Output>()
            .map_err(|err| err.to_string())?;
        match output {
            ckb_jsonrpc_types::response::Output::Success(success) => {
                serde_json::from_value(success.result).map_err(|err| err.to_string())
            }
            ckb_jsonrpc_types::response::Output::Failure(failure) => Err(failure.error.to_string()),
        }
    }
}
