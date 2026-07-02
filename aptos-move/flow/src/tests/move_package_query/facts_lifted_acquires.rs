// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

/// Pins the lifted-function acquires recompute for both the direct and
/// same-module-callee propagation paths, using invoked lambdas.
///
/// The compiler's acquires fixpoint runs before lambda lifting, so lifted
/// functions start with `acquiresInferred: []` even when their bodies read
/// same-module resources. After the fix:
///   - `__lambda__1__setup`               → direct `borrow_global<Config>` →
///     acquiresInferred: ["0xcafe::stored::Config"]
///   - `__lambda__1__lifted_calls_helper` → calls `read_config` (same-module,
///     non-lifted) → acquiresInferred: ["0xcafe::stored::Config"] via fixpoint
#[tokio::test]
async fn move_package_query_facts_lifted_acquires() {
    let pkg = common::make_package("facts_lifted_acquires", &[(
        "stored",
        "module 0xCAFE::stored {
    struct Config has key { v: u64 }

    fun apply(f: |u64|u64, x: u64): u64 { f(x) }

    // Direct: lambda body reads Config itself.
    public fun setup(x: u64): u64 {
        apply(|v| v + borrow_global<Config>(@0xCAFE).v, x)
    }

    // Indirect: lambda body calls a same-module helper that reads Config.
    public fun lifted_calls_helper(x: u64): u64 {
        apply(|v| v + read_config(), x)
    }

    fun read_config(): u64 {
        borrow_global<Config>(@0xCAFE).v
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
