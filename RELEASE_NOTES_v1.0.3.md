# Foldown 1.0.3

Released 29 August 2026 by Zeeka Limited.

Foldown 1.0.3 is a maintenance release fixing a sidebar bug reported right after 1.0.2 shipped, plus a related gap in how much of the workspace Interactive Mode is aware of. It upgrades an existing Foldown 1.0.2 installation in place.

## Fixed

- Fixed newly created folders not appearing in the sidebar tree. In the default Markdown-only view, Foldown hides any folder whose contents don't include a Markdown file — useful for hiding folders full of images or other attachments, but it also hid a folder the instant you created it, since it's empty until you add something to it. A folder with nothing in it at all is no longer pruned; a folder that has content but none of it is Markdown is still hidden, as before.
- Interactive Mode now tells the model the relative path of every Markdown file in the workspace, including files in subfolders, with every request — not just the files whose content happened to match the current message. Previously the model only ever learned a file existed if its content was retrieved as a relevant excerpt, so it had no way to answer questions like "what files do I have" or reference a file by name unless you'd already mentioned enough about its content to surface it. Full file *content* is still limited to the most relevant excerpts, so this doesn't change how many tokens a typical request uses.

## Privacy note

Because Interactive Mode now includes the full list of workspace file paths with every request, that list — file and folder names, not file content — is sent to your configured AI server on every request, not only the paths of files whose content was retrieved. See the updated Privacy considerations section in the README.

## Upgrade behaviour

- `Foldown-1.0.3-Windows-x64-Setup.exe` uses the same application identifier, product name, publisher, per-machine install mode, uninstall registry key, and `C:\Program Files\Foldown` destination as prior releases.
- Running the 1.0.3 installer upgrades and overwrites the existing Foldown installation rather than creating a second application instance.
- User workspaces and Markdown files are not part of the installation directory and are not removed or duplicated by the upgrade.

## Downloads

- `Foldown-1.0.3-Windows-x64-Setup.exe` — per-machine NSIS installer and in-place upgrader.
- `Foldown-1.0.3-Windows-x64-Standalone.exe` — portable application executable.
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

The release was verified with the complete Rust and frontend test suites (93 backend, 35 frontend), a production frontend build, Authenticode signature checks on both binaries, SHA-256 checksums, and an end-to-end pass — driven via Tauri's WebDriver automation — that reproduced the reported steps (create a folder, confirm it appears immediately) and confirmed the model correctly listed a nested file whose content shares no keywords with the test query, proving the new workspace-wide file listing is what supplied it.

## Licence and support

This release is provided under the [MIT License](LICENSE.md). Contact [support@zeeka.nz](mailto:support@zeeka.nz) for support.
