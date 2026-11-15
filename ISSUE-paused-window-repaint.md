# Paused fullscreen keeps a stale frame

The actual native Windows Vulkan run at source 2dc3dd8 renders SH025 and the navigation rings, but entering fullscreen while paused leaves an old-sized frame, white borders and black uncovered space. The captured native-frame-04-fullscreen-paused.png preserves the failure. This is a presentation failure, not a passing fullscreen check.

Paused, unfocused and occluded windows use ControlFlow::Wait. Resize reconfigures the surface and depth buffer without requesting a new frame, and fullscreen, focus and occlusion changes also omit repaint requests. A surface recovery can likewise reconfigure and return without scheduling another frame. These paths need an explicit repaint while simulation remains paused.

See [correction and native regression criteria](PR-paused-window-repaint.md).
