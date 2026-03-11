use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};
use tracing::debug;

const SERVICE_NAME: &str = "tunnel-master";

/// Retrieve an SSH key passphrase from macOS Keychain.
/// Returns None if no passphrase is stored.
pub fn get_passphrase(key_path: &str) -> Option<String> {
    match get_generic_password(SERVICE_NAME, key_path) {
        Ok(bytes) => {
            let passphrase = String::from_utf8(bytes).ok()?;
            debug!("Retrieved passphrase from Keychain for {}", key_path);
            Some(passphrase)
        }
        Err(e) => {
            debug!("No passphrase in Keychain for {}: {}", key_path, e);
            None
        }
    }
}

/// Store an SSH key passphrase in macOS Keychain.
pub fn set_passphrase(key_path: &str, passphrase: &str) -> Result<(), String> {
    // Delete existing entry if any (set_generic_password fails on duplicates)
    let _ = delete_generic_password(SERVICE_NAME, key_path);

    set_generic_password(SERVICE_NAME, key_path, passphrase.as_bytes())
        .map_err(|e| format!("Failed to store passphrase: {}", e))?;

    debug!("Stored passphrase in Keychain for {}", key_path);
    Ok(())
}

/// Delete an SSH key passphrase from macOS Keychain.
pub fn delete_passphrase(key_path: &str) -> Result<(), String> {
    delete_generic_password(SERVICE_NAME, key_path)
        .map_err(|e| format!("Failed to delete passphrase: {}", e))?;
    debug!("Deleted passphrase from Keychain for {}", key_path);
    Ok(())
}
