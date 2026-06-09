// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Regression guard: the call graph stays inline-aware.
//!
//! After the inliner expands an `inline fun` into a caller, the caller's
//! post-inlining `called_funs` loses the edge to the inline function but gains
//! the edges pulled in from the inline body. `call_graph` unions the
//! pre-inlining source snapshot (`get_source_called_functions`) with
//! `get_called_functions`, so both survive:
//!
//! `use_peek` calls `inline fun peek`, which calls `helper`. The graph shows
//! `use_peek -> {peek, helper}` (source edge + inlined edge) and
//! `peek -> {helper}`.

use crate::tests::common;

#[tokio::test]
async fn move_package_query_call_graph_inline() {
    let pkg = common::make_package("call_graph_inline", &[(
        "vault",
        "module 0xCAFE::vault {
    fun helper(x: u64): u64 { x + 1 }

    public inline fun peek(x: u64): u64 {
        helper(x)
    }

    public fun use_peek(x: u64): u64 {
        peek(x)
    }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;
    let result = common::call_tool(
        &client,
        "move_package_query",
        serde_json::json!({ "package_path": dir, "query": "call_graph" }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    common::check_baseline(file!(), &formatted);
}
