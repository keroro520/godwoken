use anyhow::Result;
use gw_rpc_client::rpc_client::RPCClient;
use gw_types::prelude::Unpack;
use lazy_static::lazy_static;
use std::cmp::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

lazy_static! {
    // TODO @keroro520 need to discuss about L1_CONFIRMATIONS
    pub static ref L1_CONFIRMATIONS: u64 = 1000;

    // L1_HEADER_CURSOR is initialized and updated by `query_l1_header_by_timestamp` which should
    // only be used by the block producer to search for a matching L1 header by timestamp.
    pub static ref L1_HEADER_CURSOR: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));
}

/// Returns timestamp of the latest confirmed block on L1
pub async fn get_confirmed_header(
    rpc_client: &RPCClient,
) -> Result<gw_jsonrpc_types::ckb_jsonrpc_types::HeaderView> {
    let tip_number: u64 = rpc_client.get_tip().await?.number().unpack();
    let confirmed_number = tip_number.saturating_sub(*L1_CONFIRMATIONS);
    let confirmed_header = rpc_client
        .get_header_by_number(confirmed_number)
        .await?
        .expect("get l1 confirmed block header");
    Ok(confirmed_header)
}

/// Query l1 header by block timestamp.
//
// This function should only be used by the block producer to search for a matching L1 header by
// timestamp.
pub async fn query_l1_header_by_timestamp(
    rpc_client: &RPCClient,
    target_timestamp: u64,
) -> Result<Option<ckb_types::core::HeaderView>> {
    let mut cursor = {
        *L1_HEADER_CURSOR
            .lock()
            .expect("acquire lock of L1_HEADER_CURSOR")
    };

    // Initialize cursor if it has not uninitialized
    if cursor == 0 {
        let tip_number: u64 = rpc_client.get_tip().await?.number().unpack();
        let confirmed_number = tip_number.saturating_sub(*L1_CONFIRMATIONS);
        cursor = confirmed_number;
    }

    let mut last_ordering: Option<Ordering> = None;
    let start_time = Instant::now();
    let mut last_warning_time = Instant::now();
    let ret = loop {
        if start_time.elapsed() > Duration::from_secs(60)
            && last_warning_time.elapsed() > Duration::from_secs(5)
        {
            last_warning_time = Instant::now();
            log::warn!(
                    "[query_l1_header_by_timestamp] has been a long time, target_timestamp: {}, cursor_number: {}, elapsed: {:?}",
                    target_timestamp, cursor, start_time.elapsed()
                );
        }

        let header = match rpc_client.get_header_by_number(cursor).await? {
            Some(header) => header,
            None => {
                log::error!(
                    "[query_l1_header_by_timestamp] get_header_by_number({}) responses None, target_timestamp: {}",
                    cursor, target_timestamp
                );
                break Ok(None);
            }
        };
        let ordering = header.inner.timestamp.value().cmp(&target_timestamp);
        match ordering {
            Ordering::Equal => break Ok(Some(header.into())),
            Ordering::Greater => {
                cursor = cursor.saturating_add(1);
            }
            Ordering::Less => {
                cursor = cursor.saturating_sub(1);
            }
        }

        if last_ordering.is_none() {
            last_ordering = Some(ordering);
        } else if last_ordering != Some(ordering) {
            log::error!(
                "[query_l1_header_by_timestamp] No matching block found, target_timestamp: {}",
                target_timestamp,
            );
            break Ok(None);
        }
    };

    // Restore the cursor
    {
        *L1_HEADER_CURSOR
            .lock()
            .expect("acquire lock of L1_HEADER_CURSOR") = cursor;
    }

    ret
}
