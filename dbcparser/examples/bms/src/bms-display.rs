// examples/bms/src/bms-display.rs

/*
 * Copyright (C) 2015-2023 IoT.bzh
 * SPDX-License-Identifier: MIT
 */

extern crate serde;
extern crate sockcan;

include!("./__bms-dbcgen.rs");
use crate::DbcSimple::CanMsgPool;

use clap::Parser;
use log::Level;
use log::{debug, error, info, warn};
use sockcan::prelude::*;

/// Read CAN messages and decode them with the generated DBC parser (BCM mode).
///
/// Examples:
///   bms-display                                   # defaults: --iface vcan0 --rate 500 --watchdog 500
///   bms-display --iface can0                      # pick real interface
///   bms-display -i vcan0 -r 200 -w 1000           # custom timers (ms)
///   bms-display -f 0x101                          # filter a single CAN ID (hex)
///   bms-display -f 257                            # filter a single CAN ID (decimal)
///   bms-display --name Voltage                # show only the 'Voltage' signal values
///   bms-display -f 0x101 --name Voltage       # combine CAN ID + signal name filters
#[derive(Debug, Parser)]
#[command(name = "bms-display", version, about, author)]
struct Args {
    /// CAN interface name
    #[arg(short = 'i', long = "iface", default_value = "vcan0")]
    iface: String,

    /// BCM receive timer period in milliseconds (SET_TIMER)
    #[arg(short = 'r', long = "rate", default_value_t = 500, value_parser = clap::value_parser!(u64).range(1..=60_000))]
    rate_ms: u64,

    /// BCM watchdog timeout in milliseconds (START_TIMER)
    #[arg(short = 'w', long = "watchdog", default_value_t = 500, value_parser = clap::value_parser!(u64).range(1..=300_000))]
    watchdog_ms: u64,

    /// Filter to a single CAN ID (accepts decimal like 257 or hex like 0x101)
    #[arg(short = 'f', long = "filter", value_parser = parse_canid)]
    filter: Option<u32>,

    /// Show only the given DBC signal name (exact match, e.g. "Voltage")
    #[arg(long = "name")]
    name: Option<String>,

    /// Increase verbosity (can be repeated: -v, -vv)
    #[arg(short = 'v', action = clap::ArgAction::Count)]
    verbose: u8,
}

/// Parse CAN ID as decimal or hex with 0x/0X prefix.
fn parse_canid(s: &str) -> Result<u32, String> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).map_err(|e| e.to_string())
    } else {
        s.parse::<u32>().map_err(|e| e.to_string())
    }
}

fn init_logging(verbosity: u8) {
    // Map -v levels to env_logger filters
    let level = match verbosity {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };
    let env = env_logger::Env::default().default_filter_or(level);
    let _ = env_logger::Builder::from_env(env).format_timestamp_millis().try_init();
}

/// Register BCM RX filters for either the whole pool or a single CAN ID.
///
/// If `only_canid` is provided, the function checks that it exists in the DBC pool
/// and subscribes to it only.
fn register_pool_filters(
    sock: &SockCanHandle,
    pool: &CanMsgPool,
    rate_ms: u64,
    watchdog_ms: u64,
    only_canid: Option<u32>,
) -> Result<(), CanError> {
    // Choose the subscription set
    let ids: Vec<u32> = if let Some(wanted) = only_canid {
        let exists = pool.get_ids().contains(&wanted);
        if !exists {
            return Err(CanError::new(
                "dbc-filter-not-found",
                format!("requested canid 0x{wanted:03X} is not present in the DBC pool"),
            ));
        }
        vec![wanted]
    } else {
        pool.get_ids().to_vec()
    };

    // Install one BCM subscription per CAN ID
    for &canid in ids.iter() {
        SockBcmCmd::new(
            CanBcmOpCode::RxSetup,
            CanBcmFlag::RX_FILTER_ID
                | CanBcmFlag::SET_TIMER
                | CanBcmFlag::START_TIMER
                | CanBcmFlag::RX_ANNOUNCE_RESUME,
            canid,
        )
        .set_timers(rate_ms, watchdog_ms)
        .apply(sock)?;
        info!(
            "Subscribed canid=0x{canid:03X} rate={}ms watchdog={}ms{}",
            rate_ms,
            watchdog_ms,
            if only_canid.is_some() { " (single filter)" } else { "" }
        );
    }
    Ok(())
}

fn main() -> Result<(), CanError> {
    let args = Args::parse();
    init_logging(args.verbose);

    info!("Opening BCM socket on iface {}", args.iface);
    let sock = SockCanHandle::open_bcm(args.iface.as_str(), CanTimeStamp::CLASSIC)?;

    let pool = CanMsgPool::new("dbc-demo");
    if pool.get_ids().is_empty() {
        warn!("DBC pool returned no IDs — nothing to subscribe.");
    }

    // Try to install filters (either full pool or a single filtered ID)
    if let Err(e) = register_pool_filters(&sock, &pool, args.rate_ms, args.watchdog_ms, args.filter)
    {
        error!("{e}");
        // Stop early if the requested filter doesn't exist
        return Err(e);
    }

    let mut count: u64 = 0;
    loop {
        count = count.saturating_add(1);

        // Read a BCM message (only filtered CAN IDs should arrive)
        let bcm_msg = sock.get_bcm_frame();

        // Prepare message for DBC parsing
        let msg_data = CanMsgData {
            canid: bcm_msg.get_id()?,
            stamp: bcm_msg.get_stamp(),
            opcode: bcm_msg.get_opcode(),
            len: bcm_msg.get_len()?,
            data: bcm_msg.get_data()?,
        };

        // Feed into the parser pool
        let msg = pool.update(&msg_data)?;
        debug!(
            "({count}) canid=0x{:03X} opcode={:?} stamp={}",
            msg_data.canid, msg_data.opcode, msg_data.stamp
        );

        for sig_ref in msg.get_signals() {
            let signal = sig_ref.borrow();

            // Apply optional name filter (exact match on DBC signal name)
            if let Some(ref wanted) = args.name {
                if signal.get_name() != wanted {
                    continue; // skip non-matching signals
                }
            }

            let age_ms = if signal.get_stamp() > 0 {
                (msg_data.stamp.saturating_sub(signal.get_stamp())) / 1000
            } else {
                0
            };

            let json = if cfg!(feature = "serde") {
                signal.to_json()
            } else {
                "serde-disabled".to_owned()
            };

            if log::log_enabled!(Level::Debug) {
                debug!(
                    "  -- {:<20} value:{:<12?} status:{:<8?} age:{:>6} ms | json:{}",
                    signal.get_name(),
                    signal.get_value(),
                    signal.get_status(),
                    age_ms,
                    json
                );
            } else {
                info!("  -- {:<20} value:{:<12}", signal.get_name(), signal.get_value());
            }
        }
    }
}
