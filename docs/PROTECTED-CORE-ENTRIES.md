# Protected core source entries

The Mule source verifier reconstructs the original bytes of all 115 protected Rust files. Use a feature-owned entry when a hull or equipment change needs to modify those files. Put the entry at `integration/<feature-id>/PROTECTED-CORE-ENTRY.json`. The directory name and the JSON `id` must match. Feature IDs use lowercase letters, digits and internal hyphens.

The verifier discovers these entries automatically. The frozen index records the 32 earlier entries and pins each entry by SHA-256. The entries preserve the earlier removal order, seven equipment manifest pins, immutable Git bases and exact declaration anchors. The older Mule, Goshawk, Renas, Commander and Stag checks still run after those entries. Keep the frozen index, baseline and historical inverse manifests intact.

## Publish a module

A declaration entry removes one exact `pub mod <module>;` line from `lib.rs`. Supply the exact LF-terminated text immediately before and after the line at the point of removal. The matching module must have a new source file. The verifier requires one declaration and one exact anchored span. It rejects a missing, duplicated or relocated declaration.

For example, Rabbit Explorer can adopt this entry when it reconciles onto the Vulture base.

```json
{
  "schema": 1,
  "id": "sh066-rabbit-explorer",
  "undoBefore": ["sh065-vulture-raider"],
  "undoAfter": [],
  "kind": "declaration",
  "module": "rabbit_explorer",
  "before": "pub mod navigation;\n",
  "after": "pub mod saiga;\n"
}
```

Rhinoceros Trader uses its own entry with `undoBefore` set to `["sh066-rabbit-explorer"]`. Its anchors are the Rabbit Explorer and Saiga declarations. Badger uses `["sh067-rhinoceros-trader"]` with the Arna Cutter and Banteng declarations as its anchors. Convert a successor as part of its normal reconciliation. Its geometry, source pins and receipts stay in its existing paths.

## Publish an equipment change

Generate the inverse from a clean worktree with the proposed feature applied, using the exact immutable base commit for the bytes you want to restore.

```sh
python3 -B tools/generate_flight_computer_inverse.py "$feature_base" > "$feature_dir/PROTECTED-CORE-INVERSE.json"
```

Place the manifest in the feature's directory. Use `kind` set to `inverse`, `manifest` set to the repository-relative path, and specify the 40-digit `base`, the 64-digit `sha256` of the manifest and a short `label`. Include the same `schema`, `id`, `undoBefore` and `undoAfter` fields as for a declaration. Review the entry, manifest and source delta together. For every touched file, replay checks the complete input hash, exact hunk offsets and bytes, and complete output hash. The approved final hashes still come from the frozen 115-file baseline.

## Declare undo order

An `undoBefore` edge runs this entry before the other entry. An `undoAfter` edge runs it after the other entry. Add dependencies to the new feature's entry. A whole-file inverse needs an explicit dependency path to every other entry that touches a file it changes. Unordered declarations are allowed when their exact byte contexts are disjoint or share only an outer line. If one declaration consumes another's context, or both use the same gap, declare their undo order. The verifier rejects unresolved overlaps, missing dependencies and cycles.

Run the source verifier and its focused tests before submitting a feature.

```sh
python3 -B tools/verify_mule_envelope_source.py
python3 -B -m unittest discover -s tools -p 'test*protected_core*.py'
python3 -B -m unittest discover -s tools -p 'test_verify_mule_envelope_source.py'
```
