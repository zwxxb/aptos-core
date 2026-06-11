// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[tokio::test]
async fn move_package_query_facts_resource_access() {
    let pkg = common::make_package("facts_resource_access", &[(
        "bank",
        "module 0xCAFE::bank {
    struct Coin has key { value: u64 }

    public fun publish(account: &signer, value: u64) {
        move_to(account, Coin { value });
    }

    public fun peek(addr: address): u64 acquires Coin {
        borrow_global<Coin>(addr).value
    }

    public fun bump(addr: address) acquires Coin {
        borrow_global_mut<Coin>(addr).value = borrow_global<Coin>(addr).value + 1;
    }

    public fun has_coin(addr: address): bool {
        exists<Coin>(addr)
    }

    native public fun mystery_take(addr: address): Coin;
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
