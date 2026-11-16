# Paid equipment and durable fitting

## Result

A commander can purchase the original SH025 Large Cargo Bay, store it without losing paid ownership, and refit the same item. The actual fitted hold changes between 92 and 127 tonnes. The market, freight admission, save validation and HUD use the same authoritative capacity.

## Implementation

The content importer binds the original cost, dedicated slot, technology and mount/budget rules to immutable source. World owns the paid item, account transaction and fitting state. The desktop forwards typed commands using the current World-issued item and tick. O opens the panel; K purchases; U stores; Y refits. Opening another panel clears hidden mission confirmations.

The current save format records equipment provenance and fitting state. Older formats refuse atomically. Existing installed previews and real profiles remain untouched during candidate validation.

## Validation

Content-component author checks passed 48 importer tests, documentation lint and immutable-source reproduction. These are not joined Rust execution evidence. Root authored two desktop regressions for panel state, hidden reviews, bounded text and insufficient-funds/missing-item atomicity. Joined compilation, Rust tests, Clippy, independent review and actual paid purchase/store/refit/cold Continue are pending.

## Compatibility and limits

No equipment refund, free device, inferred slot authority, automatic migration or cargo-preservation fallback is added. The prior memory failure, unsupported window edge checks, performance/race evidence and commercial-distribution gates remain open. This packet does not claim a new playable delivery or expanded fleet acceptance.

Related: [problem](ISSUE-paid-equipment.md), [source contract](PAID-CARGO-BAY-CONTENT.md), [architecture](ARCHITECTURE.md).
