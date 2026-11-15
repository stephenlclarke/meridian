# Original station launch and physical return

Implement the human-authorized Meridian local launch/manual docking loop using exact original SH028 display/native geometry, SH025 native hull and authored port data. The earlier geometry predicates alone did not mutate World or establish admission.

Keep one World owner, transactional invalid-input refusal and public commands/ticks. A renderer, proximity test, dock key or fabricated receipt must not grant entry. Use source-derived exterior launch, actual plane crossing and conservative continuous whole-hull collision refusal. Preserve all existing flight tests and identify the bounded collision LOD and remaining physical/authority qualification honestly.

See [implementation and criteria](PR-original-station-loop.md). Root owns desktop integration; G2 reviews the joined artifact and Dell runs actual checks/tests/GPU. No MBP compiler/native/GPU process is started for this slice.
