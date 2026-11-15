# Save bytes need host file custody

The in-progress core continuation codec cannot perform filesystem work. The desktop needs a single-writer save lease, bounded reads and same-directory staged publication that never truncates the current save before a replacement is ready. A write or rename failure must preserve the previous file; a directory-sync failure after publication must be reported as already published, rather than pretending rollback occurred.
