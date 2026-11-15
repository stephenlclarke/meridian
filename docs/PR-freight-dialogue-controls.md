# Explicit freight choices and original dialogue

Root owns the desktop caller; the existing mission owner supplies a pure original G02 presentation projection on the same current base. No new mission framework or Lua runtime is introduced.

## Desktop implementation

Add V for Receive-all review and X for Abandon review. Each requires the actual current actor and an existing active job, and still needs Enter to commit. Select the review choice from the exact requested command rather than inferring acceptance from the presence of an item. Display the explicit pending choice. Clear the ephemeral local job only after genuine abandonment. B, focus/pause and other invalidation behavior remain.

## Validation

Root ran Rust formatting and whitespace checks. Three new desktop regressions are authored, not executed: actual public flight and physical entry followed by cancellable Receive-all and once-only reward; and explicit cancellable abandonment through two finite allocations, including cold continuation refusing a third; and full readable original terms with read-only rendering, plus mission-details navigation that cannot confirm a hidden review. The existing single-unit full-journey regression retains its flight and entry assertions in a shared setup helper. Compiler, tests, strict Clippy, independent complete review and native graphics are pending on the joined artifact.

## Remaining work

The desktop now calls the original typed dialogue projection and wraps the complete review body and notice to the renderer column bound. I shows current objective and accepted journal details; opening details discards the local pending confirmation. The mission owner is finishing the exact original-source content and provenance commit. Join that commit, then run one complete review and existing Dell validation. MBP is the primary wgpu Metal and packaged-playable target. No new complete mission, asset or commercial release acceptance is claimed.
