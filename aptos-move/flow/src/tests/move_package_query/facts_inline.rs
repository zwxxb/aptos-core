// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

/// Inline functions: the definition keeps its own body facts (`isInline`),
/// and callers absorb the inlined effects (reads AND acquires) because
/// inlining happens before the acquires pass. Consumers must not
/// double-count; see the FunctionFacts docs.
#[tokio::test]
async fn move_package_query_facts_inline() {
    let pkg = common::make_package("facts_inline", &[(
        "inl",
        "module 0xCAFE::inl {
    struct Config has key { v: u64 }

    inline fun inline_read(): u64 {
        borrow_global<Config>(@0xCAFE).v
    }

    public fun uses_inline(): u64 {
        inline_read()
    }

    inline fun apply_inline(f: |u64|u64): u64 {
        f(1)
    }

    public fun uses_inline_with_lambda(): u64 {
        apply_inline(|x| x + borrow_global<Config>(@0xCAFE).v)
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
