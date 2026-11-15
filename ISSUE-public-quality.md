# Public repository, licensing and quality visibility

Meridian needs an accessible public repository, explicit proprietary licensing for its new code, and reproducible cross-platform checks with public quality metrics. Automatic Sonar analysis could not import test coverage or provide the required exact main-commit version.

## Acceptance

Keep the repository and Sonar project public, retain applicable reused-file notices, and publish no credentials. Run Rust checks/tests/Clippy on Linux, Windows and macOS plus documentation and importer checks. Produce coverage and external Clippy reports. Analyze committed main only, verify Previous version and use its exact SHA. Link live CI and Sonar badges without inventing passing results. Keep actual native UI and asset-origin limits explicit.
