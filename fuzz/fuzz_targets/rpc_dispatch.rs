//! Fuzz target: RPC host dispatch.
//!
//! Goal: ensure arbitrary packed input to `Host::dispatch` never panics and that
//! protocol lookup/header decoding are exercised.

#![no_main]

mod common;

use libfuzzer_sys::fuzz_target;
use sproto::rpc::Host;

fuzz_target!(|data: &[u8]| {
    let schema = common::build_rpc_schema();
    let mut host = Host::new(schema);
    let _ = host.dispatch(data);
});
