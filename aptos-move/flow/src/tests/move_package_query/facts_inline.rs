// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[tokio::test]
async fn move_package_query_facts_inline() {
    let pkg = common::make_package("facts_inline", &[(
        "vault",
        "module 0xCAFE::vault {
    struct Coin has key { value: u64 }

    public inline fun peek(addr: address): u64 {
        borrow_global<Coin>(addr).value
    }

    public inline fun bump(addr: address) {
        borrow_global_mut<Coin>(addr).value = borrow_global<Coin>(addr).value + 1;
    }

    public inline fun pure_add(a: u64, b: u64): u64 {
        a + b
    }

    public fun use_peek(addr: address): u64 acquires Coin {
        peek(addr)
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
