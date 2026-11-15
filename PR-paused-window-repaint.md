# Repaint on paused window transitions

Request a frame after resize, fullscreen, focus and occlusion changes. When an outdated or lost surface is reconfigured, request the follow-up frame needed by the waiting event loop. Preserve the existing pause flags, clock resets, zero-size guard, surface lifecycle and World/input rules. Do not resume simulation to force a repaint or introduce a continuous paused redraw loop.

The original native Windows run selected Intel Iris Xe / Vulkan and produced a valid windowed ship/ring scene. Its paused fullscreen capture failed with stale borders. Root independently inspected both frames and verified the live executable identity. The native owner retains that first failure separately from the corrected run.

Before accepting the correction, run workspace check, all existing 33 tests and strict Clippy on the changed source. On the Dell, pause and hold a stable tick, enter and leave fullscreen, resize to different aspect ratios, minimize/restore and switch focus. Capture fresh full-window frames with no stale borders and confirm paused ticks stay fixed. Resume and verify rendering and normal simulation without paused-time catch-up. Check clean Escape and normal close. Original tests and assertions remain unchanged; actual held-key controls require their own input evidence.

Formatting and Markdown checks are prepared separately from these pending native results. Actual Mac backend, complete gameplay, docking, receiving and save acceptance remain open.

See [problem](ISSUE-paused-window-repaint.md).
