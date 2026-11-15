# Pinned wgpu instance constructor

The first Dell Linux workspace check failed with E0599: wgpu 29.0.4 does not implement Default for InstanceDescriptor. The desktop shell cannot compile until it uses the pinned API.

Use the explicit constructor with winit's owned display handle. The same event loop supplies both the display handle and window; this also meets the GLES presentation requirement. Keep all dependency versions and gameplay unchanged.

See [change details](PR-wgpu-instance-constructor.md).
