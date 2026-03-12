use tracing::debug;

const SERVICE_NAME: &str = "tunnel-master";

pub fn get_passphrase(key_path: &str) -> Option<String> {
    let entry = keyring::Entry::new(SERVICE_NAME, key_path).ok()?;
    match entry.get_password() {
        Ok(passphrase) => {
            debug!("Retrieved passphrase from credential store");
            Some(passphrase)
        }
        Err(keyring::Error::NoEntry) => {
            debug!("No passphrase stored for this key");
            None
        }
        Err(e) => {
            debug!("Credential store error: {}", e);
            None
        }
    }
}

pub fn set_passphrase(key_path: &str, passphrase: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE_NAME, key_path)
        .map_err(|e| format!("Failed to access credential store: {}", e))?;
    entry
        .set_password(passphrase)
        .map_err(|e| format!("Failed to store passphrase: {}", e))?;
    debug!("Stored passphrase in credential store");
    Ok(())
}
