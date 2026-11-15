# Preserve the actual World across continue

The connected World and paid-services slice publishes memory only. Recreating a fresh profile on Continue would reset source stock, lose partial receipts, refuel without payment and risk repeating a completed reward. The existing account/market state also lacks the producer totals required to detect inconsistent saved balances and stock.

Add a bounded versioned core memory representation, persist the complete actual owned state and validate cross-component consistency before replacing a live World. Record service/travel accounting at the actual producer, not during load. Refuse unsupported, corrupt, foreign, duplicated or inconsistent state without repair or grants. Invalidate UI review/contact state while retaining genuine dock, property, custody and progress. Filesystem publication and real process interaction stay with the host.

[Criteria, format and limits](WORLD-SAVE-CONTINUE.md).
