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

pub fn get_password(tunnel_id: &str) -> Option<String> {
    let key = format!("password/{}", tunnel_id);
    let entry = keyring::Entry::new(SERVICE_NAME, &key).ok()?;
    match entry.get_password() {
        Ok(password) => {
            debug!("Retrieved password from credential store for tunnel {}", tunnel_id);
            Some(password)
        }
        Err(keyring::Error::NoEntry) => {
            debug!("No password stored for tunnel {}", tunnel_id);
            None
        }
        Err(e) => {
            debug!("Credential store error for tunnel {}: {}", tunnel_id, e);
            None
        }
    }
}

pub fn store_password(tunnel_id: &str, password: &str) -> Result<(), String> {
    let key = format!("password/{}", tunnel_id);
    let entry = keyring::Entry::new(SERVICE_NAME, &key).map_err(|e| e.to_string())?;
    entry.set_password(password).map_err(|e| e.to_string())
}

pub fn delete_password(tunnel_id: &str) {
    let key = format!("password/{}", tunnel_id);
    if let Ok(entry) = keyring::Entry::new(SERVICE_NAME, &key) {
        let _ = entry.delete_credential();
        debug!("Deleted password for tunnel {}", tunnel_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_password_returns_none_when_not_stored() {
        let result = get_password("test-nonexistent-tunnel-12345");
        assert!(result.is_none());
    }

    #[test]
    #[ignore = "requires OS credential store (macOS Keychain / Windows Credential Manager / Linux secret service)"]
    fn store_and_get_password_roundtrip() {
        let id = "test-roundtrip-tunnel-keychain";
        delete_password(id);

        store_password(id, "my-secret-pw").expect("store should succeed");
        let retrieved = get_password(id);
        assert_eq!(retrieved, Some("my-secret-pw".to_string()));

        delete_password(id);
        assert!(get_password(id).is_none());
    }

    #[test]
    fn delete_password_is_idempotent() {
        delete_password("test-nonexistent-delete-12345");
    }
}
