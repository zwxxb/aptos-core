// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

// Verifies that `resource_access` preserves the resource instantiation
// (`Box<T>` vs `Box<Marker>`) while `acquires_inferred` reports the bare
// storage key (`Box`). These two channels must not be conflated.
#[tokio::test]
async fn move_package_query_facts_generic_resource() {
    let pkg = common::make_package("facts_generic_resource", &[(
        "store",
        "module 0xCAFE::store {
    struct Marker has copy, drop, store {}

    struct Box<phantom T> has key { value: u64 }

    public fun put<T>(account: &signer, value: u64) {
        move_to(account, Box<T> { value });
    }

    public fun get<T>(addr: address): u64 acquires Box {
        borrow_global<Box<T>>(addr).value
    }

    public fun get_marker(addr: address): u64 acquires Box {
        borrow_global<Box<Marker>>(addr).value
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
