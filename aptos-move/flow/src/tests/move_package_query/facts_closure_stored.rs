// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

/// Stored callbacks that are never invoked.
///
/// `setup` stores a closure into `Holder.cb` but never calls it, so:
///   - `setup.resourceAccess` must be empty (no direct reads/writes)
///   - `setup.invokesFunctionValues` must be false
///   - `setup.createsClosures` provides the creator→target link
///
/// On this compiler version, storable closures must be reducible to partial
/// applications of `#[persistent]` or `public` functions — block-body lambdas
/// cannot acquire `store`. Here `|x| read_config_add(x)` is a partial
/// application of `read_config_add`, which reads `Config` directly.
/// The target function's facts (reads [Config]) carry the closure's effects;
/// whatever `setup.acquiresInferred` the compiler attributes is pinned as-is.
#[tokio::test]
async fn move_package_query_facts_closure_stored() {
    let pkg = common::make_package("facts_closure_stored", &[(
        "stored",
        "module 0xCAFE::stored {
    struct Config has key { v: u64 }
    struct Holder has key { cb: |u64|u64 has store + copy + drop }

    fun register_callback(cb: |u64|u64 has store + copy + drop) {
        borrow_global_mut<Holder>(@0xCAFE).cb = cb;
    }

    #[persistent]
    fun read_config_add(x: u64): u64 {
        x + borrow_global<Config>(@0xCAFE).v
    }

    public fun setup() {
        register_callback(|x| read_config_add(x))
    }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;
    let result = common::call_tool(
        &client,
        "move_package_query",
        serde_json::json!({ "package_path": dir, "query": "facts" }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    common::check_baseline(file!(), &formatted);
}
