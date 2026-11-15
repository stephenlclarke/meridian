# Fix pinned wgpu instance construction

Replace the unavailable InstanceDescriptor::default call with new_with_display_handle and pass the active winit event loop's owned display handle into Renderer::new. This follows the pinned wgpu 29.0.4 instance API and preserves the compositor connection for surface creation.

The original Linux workspace check failed at caab7db. Formatting and Markdown checks pass for this correction. Exact corrected-head workspace check, tests and Clippy must run on the Dell; actual GPU presentation remains unverified. No dependency, shader or gameplay change is included.

See [problem](ISSUE-wgpu-instance-constructor.md) and [implementation status](docs/IMPLEMENTATION-STATUS.md).
