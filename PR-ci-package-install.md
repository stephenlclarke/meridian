# Fix CI package installation

Install the pinned Markdown checker with npm lifecycle scripts disabled and invoke its installed binary directly. Require a binary wheel for the pinned Python coverage package. The Sonar gate and source coverage remain unchanged.

Validation: final hosted checks and exact-main Sonar analysis remain pending. No runtime code or assets change.
