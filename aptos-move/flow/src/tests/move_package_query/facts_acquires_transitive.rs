// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

/// Acquires propagation vs direct access: `caller` inherits `acquiresInferred`
/// from `helper` but must NOT report a direct read; `acquiresDeclared` reflects
/// only source annotations. Mutual recursion pins fixpoint stability.
#[tokio::test]
async fn move_package_query_facts_acquires_transitive() {
    let pkg = common::make_package("facts_acquires_transitive", &[(
        "acq",
        "module 0xCAFE::acq {
    struct R has key { v: u64 }
    struct S has key { v: u64 }

    public fun helper(): u64 acquires R {
        borrow_global<R>(@0xCAFE).v
    }

    public fun caller(): u64 acquires R {
        helper()
    }

    public fun undeclared_caller(): u64 {
        helper()
    }

    fun ping(n: u64): u64 acquires R, S {
        if (n == 0) { borrow_global<R>(@0xCAFE).v } else { pong(n - 1) }
    }

    fun pong(n: u64): u64 acquires R, S {
        if (n == 0) { borrow_global<S>(@0xCAFE).v } else { ping(n - 1) }
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
