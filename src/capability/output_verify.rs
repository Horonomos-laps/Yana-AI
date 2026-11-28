//! Output verification: "the file is the one we wrote" (a hash) is not the same as "the file is
//! a valid PNG/PDF/ZIP/...". An external program can exit 0 and leave an empty, truncated or
//! wrong-format file. This module checks the file a capability says it produced, by structure:
//! size bounds, header bytes, trailer bytes, and for ZIP the central directory.
//!
//! It proves a file is well-formed at the container level. It does not decode pixels, frames or
//! text, so a corrupt payload inside a well-formed container can still pass.

use super::error::CapabilityError;
use super::evidence::ToolEvidence;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// Bytes read from the start of the file to recognise a header.
const HEAD_BYTES: usize = 16;
/// Bytes read from the end of the file to recognise a trailer. PDF allows trailing
/// whitespace after `%%EOF`, so this is larger than any fixed trailer.
const TAIL_BYTES: usize = 1024;
/// Hard cap on an output we agree to hash and verify.
pub const MAX_OUTPUT_BYTES: u64 = 512 * 1024 * 1024;

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
const PNG_IEND_TRAILER: [u8; 12] = [0, 0, 0, 0, b'I', b'E', b'N', b'D', 0xAE, 0x42, 0x60, 0x82];
const WAV_HEADER_BYTES: u64 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputKind {
    Png,
    Jpeg,
    Gif,
    Pdf,
    Zip,
    Mp4,
    Wav,
}

/// What the caller expects the program to have produced.
#[derive(Debug, Clone, Copy)]
pub struct OutputExpectation {
    pub kind: OutputKind,
    /// Smallest size that can be a real file of this kind; at least 1.
    pub min_bytes: u64,
    /// Largest size accepted; capped at [`MAX_OUTPUT_BYTES`].
    pub max_bytes: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct OutputReport {
    pub kind: OutputKind,
    pub bytes: u64,
    pub evidence: ToolEvidence,
}

fn invalid(detail: String) -> CapabilityError {
    CapabilityError::InvalidInput { detail }
}

/// Reads the first `HEAD_BYTES` and last `TAIL_BYTES` of the file.
fn read_ends(path: &Path, len: u64) -> Result<(Vec<u8>, Vec<u8>), CapabilityError> {
    let io = |what: &str, e: std::io::Error| CapabilityError::Io {
        detail: format!("verify output: {what} {}: {e}", path.display()),
    };
    let mut file = File::open(path).map_err(|e| io("open", e))?;
    let mut head = vec![0u8; HEAD_BYTES.min(len as usize)];
    file.read_exact(&mut head).map_err(|e| io("read head of", e))?;
    let tail_len = TAIL_BYTES.min(len as usize);
    file.seek(SeekFrom::Start(len - tail_len as u64))
        .map_err(|e| io("seek in", e))?;
    let mut tail = vec![0u8; tail_len];
    file.read_exact(&mut tail).map_err(|e| io("read tail of", e))?;
    Ok((head, tail))
}

fn check_png(head: &[u8], tail: &[u8]) -> Result<(), String> {
    if !head.starts_with(&PNG_SIGNATURE) {
        return Err("PNG signature missing".into());
    }
    if !tail.ends_with(&PNG_IEND_TRAILER) {
        return Err("PNG has no IEND chunk at the end (truncated?)".into());
    }
    Ok(())
}

fn check_jpeg(head: &[u8], tail: &[u8]) -> Result<(), String> {
    if !head.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Err("JPEG start-of-image marker missing".into());
    }
    let end = tail.iter().rposition(|b| *b != 0).map_or(0, |i| i + 1);
    if !tail[..end].ends_with(&[0xFF, 0xD9]) {
        return Err("JPEG has no end-of-image marker (truncated?)".into());
    }
    Ok(())
}

fn check_gif(head: &[u8], tail: &[u8]) -> Result<(), String> {
    if !(head.starts_with(b"GIF87a") || head.starts_with(b"GIF89a")) {
        return Err("GIF header missing".into());
    }
    if tail.last() != Some(&0x3B) {
        return Err("GIF has no trailer byte (truncated?)".into());
    }
    Ok(())
}

fn check_pdf(head: &[u8], tail: &[u8]) -> Result<(), String> {
    if !head.starts_with(b"%PDF-") {
        return Err("PDF header missing".into());
    }
    if !tail.windows(5).any(|w| w == b"%%EOF") {
        return Err("PDF has no %%EOF marker near the end (truncated?)".into());
    }
    Ok(())
}

fn check_mp4(head: &[u8]) -> Result<(), String> {
    if head.len() < 8 || &head[4..8] != b"ftyp" {
        return Err("MP4 'ftyp' box missing at the start".into());
    }
    Ok(())
}

fn check_wav(head: &[u8], len: u64) -> Result<(), String> {
    if head.len() < 12 || &head[0..4] != b"RIFF" || &head[8..12] != b"WAVE" {
        return Err("WAV RIFF/WAVE header missing".into());
    }
    let declared = u64::from(u32::from_le_bytes([head[4], head[5], head[6], head[7]]));
    if declared + WAV_HEADER_BYTES != len {
        return Err(format!(
            "WAV declares {} bytes but the file is {len} (truncated or padded?)",
            declared + WAV_HEADER_BYTES
        ));
    }
    Ok(())
}

fn check_zip(path: &Path) -> Result<(), String> {
    let archive = super::archive::open_archive(path).map_err(|e| e.to_string())?;
    if archive.is_empty() {
        return Err("ZIP has no entries".into());
    }
    Ok(())
}

/// Verifies that `path` holds a well-formed file of the expected kind. Refuses a symlink or
/// anything that is not a regular file, so a program cannot point the check at a different file.
pub fn verify_output(
    path: &Path,
    expect: OutputExpectation,
    session_id: Option<String>,
) -> Result<OutputReport, CapabilityError> {
    let meta = fs::symlink_metadata(path).map_err(|_| CapabilityError::NotFound {
        requested: path.display().to_string(),
    })?;
    if !meta.file_type().is_file() {
        return Err(CapabilityError::NotAFile {
            requested: path.display().to_string(),
        });
    }
    let len = meta.len();
    let floor = expect.min_bytes.max(1);
    let ceiling = expect.max_bytes.min(MAX_OUTPUT_BYTES);
    if len < floor {
        return Err(invalid(format!(
            "output is {len} bytes, below the {floor}-byte minimum for {:?}",
            expect.kind
        )));
    }
    if len > ceiling {
        return Err(CapabilityError::TooLarge { bytes: len, limit: ceiling });
    }
    let (head, tail) = read_ends(path, len)?;
    let verdict = match expect.kind {
        OutputKind::Png => check_png(&head, &tail),
        OutputKind::Jpeg => check_jpeg(&head, &tail),
        OutputKind::Gif => check_gif(&head, &tail),
        OutputKind::Pdf => check_pdf(&head, &tail),
        OutputKind::Mp4 => check_mp4(&head),
        OutputKind::Wav => check_wav(&head, len),
        OutputKind::Zip => check_zip(path),
    };
    verdict.map_err(|why| invalid(format!("output {} is not a valid {:?}: {why}", path.display(), expect.kind)))?;
    Ok(OutputReport {
        kind: expect.kind,
        bytes: len,
        evidence: ToolEvidence::for_file(path, session_id)?,
    })
}

#[cfg(test)]
#[path = "output_verify_tests.rs"]
mod tests;
