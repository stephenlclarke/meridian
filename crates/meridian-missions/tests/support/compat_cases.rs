// Generated only from the frozen case IDs; no expectations are changed.
#[test]
fn explicit_state_fresh_vm() {
    run("explicit-state-fresh-vm");
}
#[test]
fn lossless_scalars_and_types() {
    run("lossless-scalars-and-types");
}
#[test]
fn canonical_map_key_order() {
    run("canonical-map-key-order");
}
#[test]
fn shared_dag_is_not_cycle() {
    run("shared-dag-is-not-cycle");
}
#[test]
fn empty_array_becomes_empty_map() {
    run("empty-array-becomes-empty-map");
}
#[test]
fn null_map_member_disappears() {
    run("null-map-member-disappears");
}
#[test]
fn null_array_tail_disappears() {
    run("null-array-tail-disappears");
}
#[test]
fn null_array_hole_is_sparse() {
    run("null-array-hole-is-sparse");
}
#[test]
fn scalar_tostring() {
    run("scalar-tostring");
}
#[test]
fn tostring_table_refused() {
    run("tostring-table-refused");
}
#[test]
fn numeric_integral_key_dense() {
    run("numeric-integral-key-dense");
}
#[test]
fn ordered_ipairs_and_math() {
    run("ordered-ipairs-and-math");
}
#[test]
fn argument_order_and_snapshot_copy() {
    run("argument-order-and-snapshot-copy");
}
#[test]
fn commands_remain_untrusted_values() {
    run("commands-remain-untrusted-values");
}
#[test]
fn result_nan() {
    run("result-nan");
}
#[test]
fn result_infinity() {
    run("result-infinity");
}
#[test]
fn result_function() {
    run("result-function");
}
#[test]
fn cyclic_result() {
    run("cyclic-result");
}
#[test]
fn result_key_zero() {
    run("result-key-zero");
}
#[test]
fn result_key_negative() {
    run("result-key-negative");
}
#[test]
fn result_key_fraction() {
    run("result-key-fraction");
}
#[test]
fn result_key_boolean() {
    run("result-key-boolean");
}
#[test]
fn result_key_too_large() {
    run("result-key-too-large");
}
#[test]
fn sparse_table() {
    run("sparse-table");
}
#[test]
fn mixed_table() {
    run("mixed-table");
}
#[test]
fn shape_extra_field() {
    run("shape-extra-field");
}
#[test]
fn shape_missing_data() {
    run("shape-missing-data");
}
#[test]
fn shape_missing_commands() {
    run("shape-missing-commands");
}
#[test]
fn shape_array_data() {
    run("shape-array-data");
}
#[test]
fn shape_scalar_data() {
    run("shape-scalar-data");
}
#[test]
fn shape_map_commands() {
    run("shape-map-commands");
}
#[test]
fn shape_scalar_return() {
    run("shape-scalar-return");
}
#[test]
fn shape_nil_return() {
    run("shape-nil-return");
}
#[test]
fn empty_commands_map_coerces_array() {
    run("empty-commands-map-coerces-array");
}
#[test]
fn commands_256() {
    run("commands-256");
}
#[test]
fn commands_257() {
    run("commands-257");
}
#[test]
fn output_depth() {
    run("output-depth");
}
#[test]
fn output_collection() {
    run("output-collection");
}
#[test]
fn state_byte_budget() {
    run("state-byte-budget");
}
#[test]
fn output_budget_32() {
    run("output-budget-32");
}
#[test]
fn instruction_default_top_level() {
    run("instruction-default-top-level");
}
#[test]
fn instruction_default_callback() {
    run("instruction-default-callback");
}
#[test]
fn instruction_hook_rounds_up() {
    run("instruction-hook-rounds-up");
}
#[test]
fn recursive_runtime_failure() {
    run("recursive-runtime-failure");
}
#[test]
fn allocation_bound_refusal() {
    run("allocation-bound-refusal");
}
#[test]
fn cumulative_churn_default_allocation_cap() {
    run("cumulative-churn-default-allocation-cap");
}
#[test]
fn cumulative_churn_source_allowed_64m() {
    run("cumulative-churn-source-allowed-64m");
}
#[test]
fn explicit_error() {
    run("explicit-error");
}
#[test]
fn nonstring_error_fallback() {
    run("nonstring-error-fallback");
}
#[test]
fn module_scalar() {
    run("module-scalar");
}
#[test]
fn module_missing_handler() {
    run("module-missing-handler");
}
#[test]
fn module_nonfunction_handler() {
    run("module-nonfunction-handler");
}
#[test]
fn syntax_error() {
    run("syntax-error");
}
#[test]
fn binary_chunk_refused() {
    run("binary-chunk-refused");
}
#[test]
fn invalid_handler_name() {
    run("invalid-handler-name");
}
#[test]
fn handler_length_81() {
    run("handler-length-81");
}
#[test]
fn limits_heap_small() {
    run("limits-heap-small");
}
#[test]
fn limits_allocation_below_heap() {
    run("limits-allocation-below-heap");
}
#[test]
fn limits_output_zero() {
    run("limits-output-zero");
}
#[test]
fn limits_instructions_small() {
    run("limits-instructions-small");
}
#[test]
fn limits_instructions_large() {
    run("limits-instructions-large");
}
#[test]
fn limits_heap_large() {
    run("limits-heap-large");
}
#[test]
fn limits_allocation_large() {
    run("limits-allocation-large");
}
#[test]
fn limits_output_large() {
    run("limits-output-large");
}
#[test]
fn source_over_256k() {
    run("source-over-256k");
}
#[test]
fn validate_present() {
    run("validate-present");
}
#[test]
fn validate_missing() {
    run("validate-missing");
}
#[test]
fn validate_64() {
    run("validate-64");
}
#[test]
fn validate_65() {
    run("validate-65");
}
#[test]
fn sandbox__G() {
    run("sandbox-_G");
}
#[test]
fn sandbox_collectgarbage() {
    run("sandbox-collectgarbage");
}
#[test]
fn sandbox_dofile() {
    run("sandbox-dofile");
}
#[test]
fn sandbox_loadfile() {
    run("sandbox-loadfile");
}
#[test]
fn sandbox_load() {
    run("sandbox-load");
}
#[test]
fn sandbox_print() {
    run("sandbox-print");
}
#[test]
fn sandbox_warn() {
    run("sandbox-warn");
}
#[test]
fn sandbox_pcall() {
    run("sandbox-pcall");
}
#[test]
fn sandbox_xpcall() {
    run("sandbox-xpcall");
}
#[test]
fn sandbox_getmetatable() {
    run("sandbox-getmetatable");
}
#[test]
fn sandbox_setmetatable() {
    run("sandbox-setmetatable");
}
#[test]
fn sandbox_rawget() {
    run("sandbox-rawget");
}
#[test]
fn sandbox_rawset() {
    run("sandbox-rawset");
}
#[test]
fn sandbox_rawequal() {
    run("sandbox-rawequal");
}
#[test]
fn sandbox_pairs() {
    run("sandbox-pairs");
}
#[test]
fn sandbox_next() {
    run("sandbox-next");
}
#[test]
fn sandbox_os() {
    run("sandbox-os");
}
#[test]
fn sandbox_io() {
    run("sandbox-io");
}
#[test]
fn sandbox_package() {
    run("sandbox-package");
}
#[test]
fn sandbox_debug() {
    run("sandbox-debug");
}
#[test]
fn sandbox_require() {
    run("sandbox-require");
}
#[test]
fn sandbox_coroutine() {
    run("sandbox-coroutine");
}
#[test]
fn sandbox_string() {
    run("sandbox-string");
}
#[test]
fn sandbox_table() {
    run("sandbox-table");
}
#[test]
fn sandbox_utf8() {
    run("sandbox-utf8");
}
#[test]
fn sandbox_math_random() {
    run("sandbox-math-random");
}
#[test]
fn sandbox_math_randomseed() {
    run("sandbox-math-randomseed");
}
#[test]
fn sandbox_base_allowed() {
    run("sandbox-base-allowed");
}
#[test]
fn pack_t01_accepted() {
    run("pack-t01-accepted");
}
#[test]
fn pack_navigation_contact_unconfirmed() {
    run("pack-navigation-contact-unconfirmed");
}
#[test]
fn pack_navigation_contact_sold() {
    run("pack-navigation-contact-sold");
}
#[test]
fn pack_navigation_contact_lost() {
    run("pack-navigation-contact-lost");
}
#[test]
fn pack_navigation_contact_flying() {
    run("pack-navigation-contact-flying");
}
#[test]
fn pack_navigation_confirm_cancel() {
    run("pack-navigation-confirm-cancel");
}
#[test]
fn pack_navigation_confirm_receive() {
    run("pack-navigation-confirm-receive");
}
#[test]
fn pack_navigation_confirm_lost() {
    run("pack-navigation-confirm-lost");
}
#[test]
fn pack_navigation_confirm_fit_before_cancel_branch() {
    run("pack-navigation-confirm-fit-before-cancel-branch");
}
#[test]
fn pack_courier_contact_destination() {
    run("pack-courier-contact-destination");
}
#[test]
fn pack_courier_contact_origin() {
    run("pack-courier-contact-origin");
}
#[test]
fn pack_courier_contact_in_flight() {
    run("pack-courier-contact-in-flight");
}
#[test]
fn pack_courier_generate_empty() {
    run("pack-courier-generate-empty");
}
#[test]
fn pack_guiding_light_ordered_receipts() {
    run("pack-guiding-light-ordered-receipts");
}
#[test]
fn pack_guiding_light_betrayed_suppresses_cues() {
    run("pack-guiding-light-betrayed-suppresses-cues");
}
#[test]
fn modules_closed_cache() {
    run("modules-closed-cache");
}
#[test]
fn modules_undeclared() {
    run("modules-undeclared");
}
#[test]
fn modules_cyclic() {
    run("modules-cyclic");
}
#[test]
fn modules_nontable() {
    run("modules-nontable");
}
#[test]
fn modules_bad_name_type() {
    run("modules-bad-name-type");
}
#[test]
fn wire_invalid_tag() {
    run("wire-invalid-tag");
}
#[test]
fn wire_trailing() {
    run("wire-trailing");
}
#[test]
fn wire_truncated_integer() {
    run("wire-truncated-integer");
}
#[test]
fn wire_truncated_string() {
    run("wire-truncated-string");
}
#[test]
fn wire_invalid_utf8() {
    run("wire-invalid-utf8");
}
#[test]
fn wire_infinite_double() {
    run("wire-infinite-double");
}
#[test]
fn wire_negative_zero() {
    run("wire-negative-zero");
}
#[test]
fn wire_int64_min() {
    run("wire-int64-min");
}
#[test]
fn wire_empty_array() {
    run("wire-empty-array");
}
#[test]
fn wire_empty_map() {
    run("wire-empty-map");
}
#[test]
fn wire_duplicate_key() {
    run("wire-duplicate-key");
}
#[test]
fn wire_collection_overflow() {
    run("wire-collection-overflow");
}
#[test]
fn wire_depth31() {
    run("wire-depth31");
}
#[test]
fn wire_depth32() {
    run("wire-depth32");
}
#[test]
fn input_depth32() {
    run("input-depth32");
}
#[test]
fn input_nonfinite() {
    run("input-nonfinite");
}
#[test]
fn input_string_over_limit() {
    run("input-string-over-limit");
}
#[test]
fn stored_extra_field() {
    run("stored-extra-field");
}
#[test]
fn stored_bad_number_length() {
    run("stored-bad-number-length");
}
#[test]
fn stored_uppercase_bits() {
    run("stored-uppercase-bits");
}
#[test]
fn stored_nonfinite() {
    run("stored-nonfinite");
}
#[test]
fn stored_unknown_type() {
    run("stored-unknown-type");
}
