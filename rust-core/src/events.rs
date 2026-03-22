use crate::types::{TrafficSample, TunnelStatus};

/// Entry for a keyboard-interactive prompt.
#[derive(Debug, Clone, uniffi::Record)]
pub struct KiPromptEntry {
    pub text: String,
    pub echo: bool,
}

/// Callback trait implemented by the foreign (Swift) side.
/// Rust calls these methods to push state changes, auth prompts, and traffic updates.
///
/// IMPORTANT: All methods are called from background tokio tasks.
/// The Swift implementation MUST use `Task { @MainActor in ... }` for UI updates
/// and MUST NOT synchronously call back into Rust (deadlock risk).
#[uniffi::export(with_foreign)]
pub trait TunnelEventHandler: Send + Sync {
    fn on_tunnel_state_changed(&self, id: String, status: TunnelStatus, error_message: Option<String>);
    fn on_passphrase_requested(&self, id: String, key_path: String);
    fn on_password_requested(&self, id: String);
    fn on_host_key_verification(&self, id: String, host: String, port: u16, key_type: String, fingerprint: String);
    fn on_keyboard_interactive(
        &self,
        id: String,
        name: String,
        instructions: String,
        prompts: Vec<KiPromptEntry>,
    );
    fn on_traffic_update(&self, id: String, sample: TrafficSample);
    fn on_error(&self, id: String, message: String);
}
