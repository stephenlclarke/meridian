# feat(content): extract the original SH025 paid cargo bay

Large Cargo Bay now has a reproducible 107-byte content contract with the original 28,000-tenth cost, one-based TL4, SH025 92+35t hold, dedicated slot, actual budget usage and Avenor/Renas TL12/13. The extractor authenticates original blobs and reconciles authored/runtime rules; the manifest retains exact source lines, hashes and pending statuses. No Rust or desktop path changes.

Eleven new extraction tests pass, including independent binary decoding and refusal cases. Focused importer line coverage is 98.2%. Independent source-derived fixtures cover real starter admission, payment, stored ownership, TL and overfull unfit boundaries; they are not native execution evidence. The fixed-source reproduction passes. Root owns combined review and the exact-head Dell/player checks; G3 owns version3 persistence and actual command authority. No separate PR or merge is requested by this component.

Compatibility: additive offline content only; existing binaries, profile versions and installed app remain untouched. Retained notices and original project provenance do not establish commercial clearance. The [problem](ISSUE-paid-cargo-bay-content.md) and [complete source/consumer contract](PAID-CARGO-BAY-CONTENT.md) define the bounded handoff and remaining gates.
