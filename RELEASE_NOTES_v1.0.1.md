# Foldown 1.0.1

Released 29 August 2026 by Zeeka Limited.

Foldown 1.0.1 is a maintenance release focused on safe, reliable Interactive Mode file editing. It upgrades an existing Foldown 1.0.0 installation in place.

## Fixed

- Fixed normal model responses containing fenced JSON examples being misclassified as Foldown file actions.
- Fixed valid-looking file actions failing with **The model returned malformed Foldown actions** when a model emitted an unescaped quotation mark inside Markdown content.
- Added narrowly scoped recovery for unescaped quotes inside an action's `content` field while preserving strict validation of action types, paths, workspace boundaries, duplicate targets, and Markdown-only operations.
- Fixed AI edits such as “remove section 4” deleting later untouched sections. Foldown previously supplied only search-matched excerpts while requiring the model to return a complete replacement file, allowing the model to reconstruct an incomplete document.
- Foldown now saves the active editor document before an AI request and supplies its complete content as authoritative edit context.
- Replacement actions are refused unless the complete target document was supplied to the model, preventing partial-context whole-file overwrites.
- Generic JSON examples remain visible in chat, while genuine `foldown-actions` and compatible generic action blocks remain hidden and processed normally.

## Upgrade behaviour

- `Foldown-1.0.1-Windows-x64-Setup.exe` uses the same application identifier, product name, publisher, per-machine install mode, uninstall registry key, and `C:\Program Files\Foldown` destination as 1.0.0.
- Running the 1.0.1 installer upgrades and overwrites the existing Foldown installation rather than creating a second application instance.
- User workspaces and Markdown files are not part of the installation directory and are not removed or duplicated by the upgrade.

## Downloads

- `Foldown-1.0.1-Windows-x64-Setup.exe` — per-machine NSIS installer and in-place upgrader.
- `Foldown-1.0.1-Windows-x64-Standalone.exe` — portable application executable.
- `Zeeka-Limited-Foldown-Self-Signed.cer` — public signing certificate.
- `SHA256SUMS.txt` — SHA-256 checksums for every published binary and certificate.

Foldown requires 64-bit Windows and Microsoft Edge WebView2.

## Signing information

The installer and standalone executable are Authenticode-signed with a self-signed certificate whose subject is `CN=Zeeka Limited` and whose SHA-1 thumbprint is:

```text
356487DAB123E0A290FD454EEB20613497B7E7DF
```

The certificate expires on 29 August 2031. A self-signed certificate provides integrity after it is explicitly trusted, but it does not provide third-party identity validation or Microsoft SmartScreen reputation.

## Verification

The release was verified with the complete Rust and frontend test suites, a production frontend build, generated NSIS metadata inspection, Authenticode signature checks, SHA-256 checksums, and an in-place installation test over Foldown 1.0.0.

## Licence and support

This release is provided under the [MIT License](LICENSE.md). Contact [support@zeeka.nz](mailto:support@zeeka.nz) for support.
