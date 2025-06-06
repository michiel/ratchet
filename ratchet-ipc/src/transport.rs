//! IPC transport implementations

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

use crate::error::IpcError;
use crate::protocol::MessageEnvelope;

/// IPC transport trait for different communication mechanisms
#[async_trait]
pub trait IpcTransport: Send + Sync {
    /// Send a message to the other end
    async fn send<T: Serialize + Send + Sync>(
        &mut self,
        message: &MessageEnvelope<T>,
    ) -> Result<(), IpcError>;

    /// Receive a message from the other end
    async fn receive<T: for<'de> Deserialize<'de> + Send>(
        &mut self,
    ) -> Result<MessageEnvelope<T>, IpcError>;

    /// Close the transport
    async fn close(&mut self) -> Result<(), IpcError>;
}

/// Stdin/Stdout IPC transport for process communication
pub struct StdioTransport {
    stdin: tokio::io::Stdin,
    stdout: tokio::io::Stdout,
}

impl StdioTransport {
    /// Create a new stdio transport
    pub fn new() -> Self {
        Self {
            stdin: tokio::io::stdin(),
            stdout: tokio::io::stdout(),
        }
    }
}

impl Default for StdioTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl IpcTransport for StdioTransport {
    async fn send<T: Serialize + Send + Sync>(
        &mut self,
        message: &MessageEnvelope<T>,
    ) -> Result<(), IpcError> {
        let json = serde_json::to_string(message)
            .map_err(|e| IpcError::SerializationError(e.to_string()))?;

        // Send with newline delimiter
        let message_with_newline = format!("{}\n", json);
        self.stdout
            .write_all(message_with_newline.as_bytes())
            .await
            .map_err(|e| IpcError::IoError(e.to_string()))?;

        self.stdout
            .flush()
            .await
            .map_err(|e| IpcError::IoError(e.to_string()))?;

        Ok(())
    }

    async fn receive<T: for<'de> Deserialize<'de> + Send>(
        &mut self,
    ) -> Result<MessageEnvelope<T>, IpcError> {
        let mut reader = tokio::io::BufReader::new(&mut self.stdin);
        let mut line = String::new();

        reader
            .read_line(&mut line)
            .await
            .map_err(|e| IpcError::IoError(e.to_string()))?;

        if line.is_empty() {
            return Err(IpcError::ConnectionClosed);
        }

        // Remove newline
        line.truncate(line.trim_end().len());

        let envelope: MessageEnvelope<T> = serde_json::from_str(&line)
            .map_err(|e| IpcError::DeserializationError(e.to_string()))?;

        // Check protocol version compatibility
        if envelope.protocol_version != crate::protocol::IPC_PROTOCOL_VERSION {
            return Err(IpcError::ProtocolVersionMismatch {
                expected: crate::protocol::IPC_PROTOCOL_VERSION,
                actual: envelope.protocol_version,
            });
        }

        Ok(envelope)
    }

    async fn close(&mut self) -> Result<(), IpcError> {
        // Stdin/stdout don't need explicit closing
        Ok(())
    }
}

/// Child process transport for parent-child communication
pub struct ChildProcessTransport {
    stdin: Option<tokio::process::ChildStdin>,
    stdout: Option<tokio::process::ChildStdout>,
}

impl ChildProcessTransport {
    /// Create a new child process transport
    pub fn new(stdin: tokio::process::ChildStdin, stdout: tokio::process::ChildStdout) -> Self {
        Self {
            stdin: Some(stdin),
            stdout: Some(stdout),
        }
    }
}

#[async_trait]
impl IpcTransport for ChildProcessTransport {
    async fn send<T: Serialize + Send + Sync>(
        &mut self,
        message: &MessageEnvelope<T>,
    ) -> Result<(), IpcError> {
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| IpcError::IoError("stdin already closed".to_string()))?;

        let json = serde_json::to_string(message)
            .map_err(|e| IpcError::SerializationError(e.to_string()))?;

        let message_with_newline = format!("{}\n", json);
        stdin
            .write_all(message_with_newline.as_bytes())
            .await
            .map_err(|e| IpcError::IoError(e.to_string()))?;

        stdin
            .flush()
            .await
            .map_err(|e| IpcError::IoError(e.to_string()))?;

        Ok(())
    }

    async fn receive<T: for<'de> Deserialize<'de> + Send>(
        &mut self,
    ) -> Result<MessageEnvelope<T>, IpcError> {
        let stdout = self
            .stdout
            .as_mut()
            .ok_or_else(|| IpcError::IoError("stdout already closed".to_string()))?;

        let mut reader = tokio::io::BufReader::new(stdout);
        let mut line = String::new();

        reader
            .read_line(&mut line)
            .await
            .map_err(|e| IpcError::IoError(e.to_string()))?;

        if line.is_empty() {
            return Err(IpcError::ConnectionClosed);
        }

        line.truncate(line.trim_end().len());

        let envelope: MessageEnvelope<T> = serde_json::from_str(&line)
            .map_err(|e| IpcError::DeserializationError(e.to_string()))?;

        if envelope.protocol_version != crate::protocol::IPC_PROTOCOL_VERSION {
            return Err(IpcError::ProtocolVersionMismatch {
                expected: crate::protocol::IPC_PROTOCOL_VERSION,
                actual: envelope.protocol_version,
            });
        }

        Ok(envelope)
    }

    async fn close(&mut self) -> Result<(), IpcError> {
        // Take ownership and drop to close
        let _ = self.stdin.take();
        let _ = self.stdout.take();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::WorkerMessage;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_message_envelope_serialization() {
        let message = WorkerMessage::Ping {
            correlation_id: Uuid::new_v4(),
        };

        let envelope = MessageEnvelope::new(message);
        let json = serde_json::to_string(&envelope).unwrap();

        // Verify it's valid JSON and can be deserialized
        let deserialized: MessageEnvelope<WorkerMessage> = serde_json::from_str(&json).unwrap();
        assert_eq!(
            deserialized.protocol_version,
            crate::protocol::IPC_PROTOCOL_VERSION
        );
    }
}
