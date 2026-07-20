# Ground Station Release Instructions

## Required release workflow

- Every completed application-code or UI change in this repository must produce a new versioned GitHub Release; a local build alone is not complete.
- Run the frontend tests, Svelte/TypeScript checks, production frontend build, Rust tests/check, and Windows Tauri no-bundle release build before publishing.
- Copy `src-tauri/target/release/app.exe` to `artifacts/GroundStation_<version>_Portable_<YYYY-MM-DD_HHMMSS>.exe`, create the matching `.json` manifest and `.sha256`, and update `artifacts/LATEST.txt`.
- Keep `artifacts/*.exe` out of Git. Commit source and release metadata, push the intended branch, and upload the `.exe`, manifest, and checksum to the matching GitHub Release.
- Download all published assets into an isolated temporary directory and verify the release state, asset names, byte size, and SHA-256 before declaring completion.
