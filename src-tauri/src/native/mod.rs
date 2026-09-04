pub mod credentials;

use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// A file path handed to the app at launch (via "Open with Foldown" or a
/// second-instance relaunch) that the frontend hasn't asked for yet — it
/// isn't ready to receive an event the instant the Rust side starts up, so
/// this is pulled once via a command instead of relying on emit timing.
pub struct PendingOpen(pub Mutex<Option<String>>);

impl PendingOpen {
    pub fn new() -> Self {
        Self(Mutex::new(None))
    }
}

impl Default for PendingOpen {
    fn default() -> Self {
        Self::new()
    }
}

/// Finds the first `.md` file among `args` that actually exists on disk —
/// used both for this process's own launch arguments and for the argv a
/// second instance hands over when the user opens another file while
/// Foldown is already running.
pub fn extract_md_arg(args: &[String]) -> Option<PathBuf> {
    args.iter().skip(1).find_map(|arg| {
        let path = PathBuf::from(arg);
        let is_md = path
            .extension()
            .map(|e| e.eq_ignore_ascii_case("md"))
            .unwrap_or(false);
        (is_md && path.is_file()).then_some(path)
    })
}

/// Adds `path` to Windows' "Recent" jump-list category. Best-effort — a
/// failure here should never interrupt opening a file.
///
/// `SHAddToRecentDocs` is a shell/COM-backed call; Tauri commands run on a
/// worker thread with no COM apartment initialized, so it silently no-ops
/// without this — `CoInitializeEx` is safe to call redundantly (it just
/// returns `S_FALSE`) if some other code on this thread already did it.
pub fn add_to_recent_docs(path: &Path) {
    use std::os::windows::ffi::OsStrExt;

    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // Run on a dedicated, freshly-spawned OS thread rather than whatever
    // Tauri worker thread happens to be handling this command — the shell
    // call is COM-backed and Tauri's async runtime can reuse/tear down its
    // pool threads before a call like this finishes its work.
    let _ = std::thread::spawn(move || {
        use std::ffi::c_void;
        use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
        use windows::Win32::UI::Shell::{SHAddToRecentDocs, SHARD_PATHW};
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            SHAddToRecentDocs(SHARD_PATHW.0 as u32, Some(wide.as_ptr() as *const c_void));
        }
    })
    .join();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_file(name: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("foldown-native-test-{}-{}", std::process::id(), n));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        fs::write(&path, "content").unwrap();
        path
    }

    #[test]
    fn extracts_existing_md_arg() {
        let path = temp_file("note.md");
        let args = vec![
            "foldown.exe".to_string(),
            path.to_string_lossy().into_owned(),
        ];
        assert_eq!(extract_md_arg(&args), Some(path));
    }

    #[test]
    fn ignores_non_md_and_missing_files() {
        let png = temp_file("image.png");
        let args = vec![
            "foldown.exe".to_string(),
            png.to_string_lossy().into_owned(),
            "C:\\does\\not\\exist.md".to_string(),
        ];
        assert_eq!(extract_md_arg(&args), None);
    }

    #[test]
    fn no_args_returns_none() {
        assert_eq!(extract_md_arg(&["foldown.exe".to_string()]), None);
    }
}
