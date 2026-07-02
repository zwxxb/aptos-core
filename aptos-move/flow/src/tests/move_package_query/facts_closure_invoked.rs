// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

/// Closure invocation vs creation: `run` invokes an unknown function value
/// (flagged incomplete); `immediate` creates and invokes its own closure in
/// place (known target); `chain` passes a closure to a same-module invoker
/// (creation recorded on `chain`, the unknown invoke stays on `run`).
#[tokio::test]
async fn move_package_query_facts_closure_invoked() {
    let pkg = common::make_package("facts_closure_invoked", &[(
        "invoked",
        "module 0xCAFE::invoked {
    struct R has key { v: u64 }

    fun run(f: ||u64): u64 {
        f()
    }

    public fun immediate(): u64 {
        let f = || borrow_global<R>(@0xCAFE).v;
        f()
    }

    public fun chain(): u64 {
        run(|| borrow_global<R>(@0xCAFE).v)
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
