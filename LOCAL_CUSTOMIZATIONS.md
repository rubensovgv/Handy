# Local dictionary shortcut (macOS)

Select a corrected word or short phrase in another application and press
**Control + Shift + Space**. Handy adds it to its existing custom words and
shows a brief status without focusing its settings window. The shortcut can
be changed under General. Option + Space still dictates; Option + Shift +
Space remains the optional post-processing shortcut.

The action uses the macOS Accessibility selected-text attribute only when
invoked. It does not monitor typing, inspect the whole field, or use the
clipboard. Applications must expose their selection to Accessibility. No
selection, unsupported applications, missing permission, duplicates, and
selections over 50 characters or containing paragraphs produce a status
instead of adding data. Existing spelling is kept for case-insensitive
duplicates. Remove an unwanted word under Advanced > Custom words.

A local rebuild has a different code signature from the official download.
macOS may require renewing Accessibility permission for the installed app.
Do not reset unrelated permissions. Installing an official update replaces
this customization; keep the source commit to rebuild it.

## Build and validate

Use the SDK matching Xcode if Command Line Tools point at a newer SDK:

```sh
export SDKROOT=/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX26.4.sdk
export CMAKE_POLICY_VERSION_MINIMUM=3.5
bun install --frozen-lockfile
bun run lint
bun run build
(cd src-tauri && cargo test --release dictionary_shortcut --lib)
(cd src-tauri && cargo clippy --release --lib)
bun run tauri build --bundles app --config '{"bundle":{"createUpdaterArtifacts":false}}'
```

This only builds a local app. No publishing or deployment is required.
