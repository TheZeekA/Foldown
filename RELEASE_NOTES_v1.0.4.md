# Foldown 1.0.4

Released 29 August 2026 by Zeeka Limited.

Foldown 1.0.4 is a maintenance release fixing a sidebar drag-and-drop bug reported right after 1.0.3 shipped. It upgrades an existing Foldown 1.0.3 installation in place.

## Fixed

- Fixed dragging a file or folder onto another folder in the sidebar not working. The app window had Tauri's native OS-level drag-and-drop handling enabled, which on Windows takes over drag events before they reach the page — silently blocking the sidebar's own in-app drag-and-drop (used to move files and folders around the workspace tree). Native window drag-and-drop is now disabled so the sidebar's drag-and-drop works as expected.
- Dropping an external `.md` file from Explorer onto the Foldown window to open it still works — that feature now uses the same in-page drag-and-drop mechanism instead of the OS-level handler it relied on before.

## Upgrade behaviour

- `Foldown-1.0.4-Windows-x64-Setup.exe` uses the same application identifier, product name, publisher, per-machine install mode, uninstall registry key, and `C:\Program Files\Foldown` destination as prior releases.
- Running the 1.0.4 installer upgrades and overwrites the existing Foldown installation rather than creating a second application instance.
- User workspaces and Markdown files are not part of the installation directory and are not removed or duplicated by the upgrade.

## Downloads

- `Foldown-1.0.4-Windows-x64-Setup.exe` — per-machine NSIS installer and in-place upgrader.
- `Foldown-1.0.4-Windows-x64-Standalone.exe` — portable application executable.
- `Zeeka-Limited-Foldown-Self-Signed.cer` — public signing certificate.
- `SHA256SUMS.txt` — SHA-256 checksums for every published binary and certificate.

Foldown requires 64-bit Windows and Microsoft Edge WebView2.

## Signing information

The installer and standalone executable are Authenticode-signed with a self-signed certificate whose subject is `CN=Zeeka Limited` and whose SHA-1 thumbprint is:

```text
356487DAB123E0A290FD454EEB20613497B7E7DF
```

The certificate expires on 29 August 2031. Compare the downloaded certificate's thumbprint with the value above before trusting it. A self-signed certificate provides integrity after it is explicitly trusted, but it does not provide third-party identity validation or Microsoft SmartScreen reputation.

## Verification

The release was verified with the complete Rust and frontend test suites (93 backend, 35 frontend), a production frontend build, and Authenticode signature checks on both binaries. The sidebar drag-and-drop fix has no automated test coverage — it was root-caused to Tauri's window-level drag-and-drop configuration conflicting with in-page HTML5 drag-and-drop, and the reporting user confirmed the fix by testing an unsigned build ahead of this release.

## Licence and support

This release is provided under the [MIT License](LICENSE.md). Contact [support@zeeka.nz](mailto:support@zeeka.nz) for support.
