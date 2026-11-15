# CI package installation

The first exact-main Sonar analysis failed the security rating because the documentation and coverage installers permitted package build or lifecycle scripts. All platform tests and coverage jobs passed.

Require script-free installation while retaining the same pinned checker versions, tests, coverage scope and quality gate.
