# Releasing lunatui

Public releases must follow the project review policy and CI gates.

## Release text and artifacts

Release text and artifacts must not reference private infrastructure, hostnames, personal paths, or internal tooling. This applies to tag messages, GitHub release titles and bodies, changelogs, release notes, documentation, and the contents of shipped binaries and archives.

Before publishing, CI runs `scripts/check-release-text.sh` against those sources (and optional extra deny patterns from local or CI configuration). Release builds use `RUSTFLAGS` path remapping so shipped executables do not embed builder home directories.
