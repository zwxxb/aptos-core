// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[tokio::test]
async fn move_package_query_facts_spec() {
    let pkg = common::make_package("facts_spec", &[(
        "ledger",
        "module 0xCAFE::ledger {
    struct Account has key { balance: u64 }
    spec Account {
        invariant balance >= 0;
    }

    public fun deposit(addr: address, amount: u64) acquires Account {
        let a = borrow_global_mut<Account>(addr);
        a.balance = a.balance + amount;
    }
    spec deposit {
        ensures global<Account>(addr).balance == old(global<Account>(addr).balance) + amount;
    }

    public fun pure_fn(): u64 { 42 }
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
