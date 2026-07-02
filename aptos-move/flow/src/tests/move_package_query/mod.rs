// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

mod call_graph;
mod call_graph_closure;
mod call_graph_leaf;
mod dep_graph;
mod dep_graph_no_deps;
mod facts;
mod facts_closure;
mod facts_closure_stored;
mod facts_lifted_acquires;
mod facts_acquires_transitive;
mod facts_compile_error;
mod facts_function_value;
mod facts_lambda_tag;
mod facts_nested;
mod facts_spec_block;
mod function_usage;
mod function_usage_bad_format;
mod function_usage_bad_function;
mod function_usage_bad_module;
mod function_usage_closure;
mod function_usage_leaf;
mod invalid_path;
mod invalid_query;
mod module_summary;
mod module_summary_empty;
