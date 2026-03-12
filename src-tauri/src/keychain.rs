use tracing::debug;

const SERVICE_NAME: &str = "tunnel-master";

// ── macOS: use Security.framework Keychain ──────────────────────────

#[cfg(target_os = "macos")]
use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};

#[cfg(target_os = "macos")]
pub fn get_passphrase(key_path: &str) -> Option<String> {
    match get_generic_password(SERVICE_NAME, key_path) {
        Ok(bytes) => {
            let passphrase = String::from_utf8(bytes).ok()?;
            debug!("Retrieved passphrase from Keychain");
            Some(passphrase)
        }
        Err(e) => {
            debug!("No passphrase in Keychain: {}", e);
            None
        }
    }
}

#[cfg(target_os = "macos")]
pub fn set_passphrase(key_path: &str, passphrase: &str) -> Result<(), String> {
    let _ = delete_generic_password(SERVICE_NAME, key_path);
    set_generic_password(SERVICE_NAME, key_path, passphrase.as_bytes())
        .map_err(|e| format!("Failed to store passphrase: {}", e))?;
    debug!("Stored passphrase in Keychain");
    Ok(())
}

// ── Linux / Windows: no keychain support yet ────────────────────────

#[cfg(not(target_os = "macos"))]
pub fn get_passphrase(_key_path: &str) -> Option<String> {
    debug!("Keychain not available on this platform");
    None
}

#[cfg(not(target_os = "macos"))]
pub fn set_passphrase(_key_path: &str, _passphrase: &str) -> Result<(), String> {
    Err("Keychain storage is not yet supported on this platform".to_string())
}
