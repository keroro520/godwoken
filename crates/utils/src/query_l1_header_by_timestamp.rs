use anyhow::Result;
use gw_rpc_client::rpc_client::RPCClient;
use gw_types::prelude::Unpack;
use std::cmp::Ordering;

pub const L1_CONFIRMATION_COUNT: u64 = 100;

pub async fn get_confirmed_header_timestamp(rpc_client: &RPCClient) -> Result<u64> {
    let tip_number: u64 = rpc_client.get_tip().await?.number().unpack();
    let confirmed_number = tip_number.saturating_sub(L1_CONFIRMATION_COUNT);
    let confirmed_timestamp = rpc_client
        .get_header_by_number(confirmed_number)
        .await?
        .expect("get l1 confirmed block header")
        .inner
        .timestamp
        .value();
    Ok(confirmed_timestamp)
}

// TODO @keroro520 It is early version, need optimization. query_l1_header_by_timestamp
/// Query l1 header by block timestamp
pub async fn query_l1_header_by_timestamp(
    rpc_client: &RPCClient,
    timestamp: u64,
) -> Result<Option<ckb_types::core::HeaderView>> {
    let tip_number: u64 = rpc_client.get_tip().await?.number().unpack();
    let confirmed_number = tip_number.saturating_sub(L1_CONFIRMATION_COUNT);
    for number in (confirmed_number - 100..=confirmed_number + 10).rev() {
        let header = rpc_client
            .get_header_by_number(number)
            .await?
            .expect("get_header_by_number in query_l1_header_by_timestamp");
        match header.inner.timestamp.value().cmp(&timestamp) {
            Ordering::Equal => return Ok(Some(header.into())),
            Ordering::Greater => continue,
            Ordering::Less => {
                log::warn!(
                    "[query_l1_header_by_timestamp] failed, timestamp={}, tip_number={}",
                    timestamp,
                    tip_number
                );
                return Ok(None);
            }
        }
    }
    log::warn!(
        "[query_l1_header_by_timestamp] failed, timestamp={}, tip_number={}",
        timestamp,
        tip_number
    );
    Ok(None)
}
