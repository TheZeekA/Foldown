use std::sync::Mutex;

use windows::core::HSTRING;
use windows::core::PWSTR;
use windows::Win32::Foundation::{ERROR_NOT_FOUND, FILETIME};
use windows::Win32::Security::Credentials::{
    CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_FLAGS,
    CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
};

use crate::error::{AppError, AppResult};

/// Serializes every call into Windows Credential Manager from this process.
/// Concurrent `CredWriteW`/`CredReadW`/`CredDeleteW` calls from multiple
/// threads were observed to intermittently lose a just-written value to an
/// immediately following read on the *same* thread (reproduced reliably
/// under `cargo test`'s default multi-threaded runner; 100% reliable with
/// `--test-threads=1`) — this is cheap insurance since settings load/save is
/// not a hot path.
static CREDENTIAL_LOCK: Mutex<()> = Mutex::new(());

fn target_name(provider: &str) -> String {
    format!("Foldown/ai-key/{provider}")
}

/// Stores (or overwrites) `key` under the Credential Manager target for
/// `provider`, persisted at the local-machine level (survives reboots, tied
/// to this Windows login via DPAPI under the hood).
pub fn store_api_key(provider: &str, key: &str) -> AppResult<()> {
    let _guard = CREDENTIAL_LOCK.lock().unwrap();
    let target = target_name(provider);
    let mut target_buf: Vec<u16> = target.encode_utf16().chain(std::iter::once(0)).collect();
    let mut username_buf: Vec<u16> = "Foldown\0".encode_utf16().collect();
    let mut blob = key.as_bytes().to_vec();

    let credential = CREDENTIALW {
        Flags: CRED_FLAGS(0),
        Type: CRED_TYPE_GENERIC,
        TargetName: PWSTR(target_buf.as_mut_ptr()),
        Comment: PWSTR::null(),
        LastWritten: FILETIME::default(),
        CredentialBlobSize: blob.len() as u32,
        CredentialBlob: blob.as_mut_ptr(),
        Persist: CRED_PERSIST_LOCAL_MACHINE,
        AttributeCount: 0,
        Attributes: std::ptr::null_mut(),
        TargetAlias: PWSTR::null(),
        UserName: PWSTR(username_buf.as_mut_ptr()),
    };

    unsafe { CredWriteW(&credential, 0) }.map_err(|e| {
        AppError::Message(format!(
            "Could not save API key to Windows Credential Manager: {e}"
        ))
    })
}

/// Returns `None` (not an error) when no credential is stored under this
/// provider's target name yet — the normal state for a provider the user
/// hasn't configured.
pub fn read_api_key(provider: &str) -> AppResult<Option<String>> {
    let _guard = CREDENTIAL_LOCK.lock().unwrap();
    let target = HSTRING::from(target_name(provider));
    let mut credential: *mut CREDENTIALW = std::ptr::null_mut();
    match unsafe { CredReadW(&target, CRED_TYPE_GENERIC, None, &mut credential) } {
        Ok(()) => {
            let value = unsafe {
                let cred = &*credential;
                let bytes = std::slice::from_raw_parts(
                    cred.CredentialBlob,
                    cred.CredentialBlobSize as usize,
                );
                String::from_utf8_lossy(bytes).into_owned()
            };
            unsafe { CredFree(credential as *const _) };
            Ok(Some(value))
        }
        Err(e) if e.code() == ERROR_NOT_FOUND.to_hresult() => Ok(None),
        Err(e) => Err(AppError::Message(format!(
            "Could not read API key from Windows Credential Manager: {e}"
        ))),
    }
}

/// Deleting a credential that doesn't exist is treated as success, not an
/// error — callers (e.g. `SettingsStore::set_ai_settings`) call this
/// unconditionally whenever a provider's key field is empty.
pub fn delete_api_key(provider: &str) -> AppResult<()> {
    let _guard = CREDENTIAL_LOCK.lock().unwrap();
    let target = HSTRING::from(target_name(provider));
    match unsafe { CredDeleteW(&target, CRED_TYPE_GENERIC, None) } {
        Ok(()) => Ok(()),
        Err(e) if e.code() == ERROR_NOT_FOUND.to_hresult() => Ok(()),
        Err(e) => Err(AppError::Message(format!(
            "Could not delete API key from Windows Credential Manager: {e}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    /// A unique provider name per test invocation so these tests exercise a
    /// real Windows Credential Manager round-trip without ever touching (or
    /// colliding with) the app's real `Foldown/ai-key/{local,openai,...}`
    /// entries on the developer's machine.
    fn unique_provider() -> String {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        format!("test-{}-{}", std::process::id(), n)
    }

    #[test]
    fn stores_reads_and_deletes_a_real_windows_credential() {
        let provider = unique_provider();
        assert_eq!(read_api_key(&provider).unwrap(), None);

        store_api_key(&provider, "sk-test-12345").unwrap();
        assert_eq!(
            read_api_key(&provider).unwrap(),
            Some("sk-test-12345".to_string())
        );

        // Overwriting an existing credential must replace, not fail.
        store_api_key(&provider, "sk-test-67890").unwrap();
        assert_eq!(
            read_api_key(&provider).unwrap(),
            Some("sk-test-67890".to_string())
        );

        delete_api_key(&provider).unwrap();
        assert_eq!(read_api_key(&provider).unwrap(), None);
    }

    #[test]
    fn deleting_a_credential_that_was_never_stored_is_not_an_error() {
        let provider = unique_provider();
        delete_api_key(&provider).unwrap();
    }
}
