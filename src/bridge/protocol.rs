//! Wire format of the Yana bridge: one JSON object per line.
//!
//! This module only parses and checks. It opens no socket and trusts nothing
//! in a line: sizes are capped, identifiers are restricted, and a `control`
//! message the bridge does not know is refused instead of passed along.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

pub const PROTOCOL_VERSION: u32 = 1;
/// Same line limit as the headless chat protocol used by both apps.
pub const MAX_LINE_BYTES: usize = 1024 * 1024;
const MAX_NAME_CHARS: usize = 128;
const MAX_OFFERED_VERSIONS: usize = 16;
const MAX_CAPABILITIES: usize = 64;
/// Everything a client may ask for on the `control` stream.
const CONTROL_KINDS: [&str; 2] = ["stop", "approval.decision"];

/// Independent lanes of one connection. A busy lane never delays another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stream {
    Control,
    Chat,
    State,
    Settings,
    System,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Envelope {
    pub v: u32,
    pub id: String,
    pub stream: Stream,
    #[serde(rename = "type")]
    pub kind: String,
    /// The client that made the change, so it is not sent back to it.
    pub origin: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rev: Option<u64>,
    #[serde(default)]
    pub body: Value,
}

/// First message of a connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hello {
    pub app: String,
    pub app_version: String,
    pub protocols: Vec<u32>,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    TooLarge {
        bytes: usize,
    },
    Malformed(String),
    UnsupportedVersion(u32),
    BadName {
        field: &'static str,
    },
    UnknownControl(String),
    TooMany {
        field: &'static str,
    },
    /// No shared protocol version: names the side that must update.
    NoCommonVersion {
        app: String,
        update: Side,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    App,
    Runtime,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge { bytes } => {
                write!(
                    f,
                    "line of {bytes} bytes is over the {MAX_LINE_BYTES} byte limit"
                )
            }
            Self::Malformed(detail) => write!(f, "not a valid message: {detail}"),
            Self::UnsupportedVersion(v) => write!(f, "protocol version {v} is not supported"),
            Self::BadName { field } => write!(
                f,
                "field '{field}' is empty, too long or has unsafe characters"
            ),
            Self::UnknownControl(kind) => write!(f, "unknown control message '{kind}' refused"),
            Self::TooMany { field } => write!(f, "field '{field}' has too many entries"),
            Self::NoCommonVersion {
                app,
                update: Side::App,
            } => {
                write!(f, "no shared protocol version: update the app '{app}'")
            }
            Self::NoCommonVersion {
                app,
                update: Side::Runtime,
            } => {
                write!(f, "no shared protocol version with '{app}': update yana-rt")
            }
        }
    }
}

impl std::error::Error for ProtocolError {}

/// Names become log fields, map keys and later file names.
fn check_name(field: &'static str, value: &str) -> Result<(), ProtocolError> {
    let safe = |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-');
    let dots_only = value.chars().all(|c| c == '.');
    if value.is_empty() || value.len() > MAX_NAME_CHARS || dots_only || !value.chars().all(safe) {
        return Err(ProtocolError::BadName { field });
    }
    Ok(())
}

fn decode<T: serde::de::DeserializeOwned>(line: &str) -> Result<T, ProtocolError> {
    if line.len() > MAX_LINE_BYTES {
        return Err(ProtocolError::TooLarge { bytes: line.len() });
    }
    // The serde_json error names a position, never the content of the line.
    serde_json::from_str(line).map_err(|error| {
        ProtocolError::Malformed(format!(
            "{:?} at column {}",
            error.classify(),
            error.column()
        ))
    })
}

/// Parses one message. Unknown fields are ignored so newer clients keep working.
pub fn parse_envelope(line: &str) -> Result<Envelope, ProtocolError> {
    let envelope: Envelope = decode(line)?;
    if envelope.v != PROTOCOL_VERSION {
        return Err(ProtocolError::UnsupportedVersion(envelope.v));
    }
    check_name("id", &envelope.id)?;
    check_name("type", &envelope.kind)?;
    check_name("origin", &envelope.origin)?;
    if let Some(object) = &envelope.object {
        check_name("object", object)?;
    }
    if envelope.stream == Stream::Control && !CONTROL_KINDS.contains(&envelope.kind.as_str()) {
        return Err(ProtocolError::UnknownControl(envelope.kind));
    }
    Ok(envelope)
}

pub fn parse_hello(line: &str) -> Result<Hello, ProtocolError> {
    let hello: Hello = decode(line)?;
    check_name("app", &hello.app)?;
    check_name("app_version", &hello.app_version)?;
    if hello.protocols.len() > MAX_OFFERED_VERSIONS {
        return Err(ProtocolError::TooMany { field: "protocols" });
    }
    if hello.capabilities.len() > MAX_CAPABILITIES {
        return Err(ProtocolError::TooMany {
            field: "capabilities",
        });
    }
    for capability in &hello.capabilities {
        check_name("capabilities", capability)?;
    }
    Ok(hello)
}

/// Picks the highest version both sides speak, or says which side is too old.
pub fn negotiate(supported: &[u32], hello: &Hello) -> Result<u32, ProtocolError> {
    if let Some(version) = hello
        .protocols
        .iter()
        .copied()
        .filter(|v| supported.contains(v))
        .max()
    {
        return Ok(version);
    }
    let newest_here = supported.iter().copied().max().unwrap_or(0);
    let newest_there = hello.protocols.iter().copied().max().unwrap_or(0);
    let update = if newest_there > newest_here {
        Side::Runtime
    } else {
        Side::App
    };
    Err(ProtocolError::NoCommonVersion {
        app: hello.app.clone(),
        update,
    })
}

#[cfg(test)]
mod tests;
