//! What the shell is told about a source's connection.
use crate::{RemoteError, Route};
use kelvo_schema::HostId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ConnectionStatus {
    pub connected: bool,
    pub route: Option<Route>,
    pub retrying: bool,
    pub error: Option<String>,
    pub history_error: Option<String>,
    pub last_seen_ms: Option<i64>,
    pub catching_up: bool,
    pub clock_offset_ms: i64,
    /// The connected peer's `Hello.app_version`, kept after it disconnects so the
    /// shell can offer an agent update while the host is offline.
    pub remote_version: Option<String>,
    /// The last attempt failed because the peer's protocol is incompatible
    /// ([`RemoteError::Incompatible`]); only a coordinated upgrade reconnects it.
    pub incompatible: bool,
    /// The id a host answered with when it is not the paired one
    /// ([`RemoteError::IdentityMismatch`]), so the shell can offer re-pairing.
    pub identity_changed: Option<HostId>,
}
impl ConnectionStatus {
    pub(super) fn record_failure(&mut self, e: &RemoteError) {
        self.connected = false;
        self.error = Some(e.to_string());
        self.identity_changed = match e {
            RemoteError::IdentityMismatch { actual, .. } => Some(*actual),
            _ => None,
        };
        self.incompatible = matches!(
            e,
            RemoteError::Incompatible(_) | RemoteError::IncompatiblePeer { .. }
        );
        if let RemoteError::IncompatiblePeer { version, .. } = e {
            self.remote_version = Some(version.clone());
        }
        self.catching_up = false;
        self.retrying = e.retryable();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn an_incompatible_peer_keeps_its_version_until_it_reconnects() {
        let mut status = super::ConnectionStatus {
            remote_version: Some("0.2.0".into()),
            ..Default::default()
        };
        status.record_failure(&crate::RemoteError::IncompatiblePeer {
            version: "0.1.0".into(),
            reason: "protocol 1 is below 2".into(),
        });
        assert!(status.incompatible && !status.retrying);
        assert_eq!(status.remote_version.as_deref(), Some("0.1.0"));
        status.record_failure(&crate::RemoteError::Transport("offline".into()));
        assert!(!status.incompatible && status.retrying);
        assert_eq!(status.remote_version.as_deref(), Some("0.1.0"));
    }
}
