//! Framing: a u32 big-endian body length, then that many bytes of JSON. The same shape
//! as `control.sock`. [`encode_frame`] and [`decode_len`] are I/O-free so the app's tokio
//! server can reuse them with its own async reads; [`read_frame`] and [`write_frame`] are
//! the blocking std versions the client uses.

use std::io::{self, Read, Write};

use serde::{Serialize, de::DeserializeOwned};

#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    #[error("frame of {len} bytes exceeds the {max}-byte limit")]
    TooLarge { len: usize, max: usize },
    #[error("connection closed mid-frame")]
    Truncated,
    #[error("frame is not valid JSON for this message: {0}")]
    Json(#[from] serde_json::Error),
    #[error("socket I/O failed: {0}")]
    Io(#[source] io::Error),
}

impl FrameError {
    /// Whether the failure was a read or write deadline passing.
    pub fn is_timeout(&self) -> bool {
        matches!(self, FrameError::Io(e) if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut))
    }
}

impl From<io::Error> for FrameError {
    fn from(e: io::Error) -> Self {
        if e.kind() == io::ErrorKind::UnexpectedEof {
            FrameError::Truncated
        } else {
            FrameError::Io(e)
        }
    }
}

/// The length prefix and JSON body of `value`, refusing a body over `max` bytes.
pub fn encode_frame<T: Serialize>(value: &T, max: usize) -> Result<Vec<u8>, FrameError> {
    let body = serde_json::to_vec(value)?;
    let len = u32::try_from(body.len())
        .ok()
        .filter(|_| body.len() <= max)
        .ok_or(FrameError::TooLarge {
            len: body.len(),
            max,
        })?;
    let mut frame = Vec::with_capacity(4 + body.len());
    frame.extend_from_slice(&len.to_be_bytes());
    frame.extend_from_slice(&body);
    Ok(frame)
}

/// The body length a prefix announces, refusing one over `max` before anything is
/// allocated for it.
pub fn decode_len(prefix: [u8; 4], max: usize) -> Result<usize, FrameError> {
    let len = u32::from_be_bytes(prefix) as usize;
    if len > max {
        return Err(FrameError::TooLarge { len, max });
    }
    Ok(len)
}

/// Reads one frame and decodes its body.
pub fn read_frame<R: Read, T: DeserializeOwned>(
    reader: &mut R,
    max: usize,
) -> Result<T, FrameError> {
    let mut prefix = [0u8; 4];
    reader.read_exact(&mut prefix)?;
    let len = decode_len(prefix, max)?;
    let mut body = vec![0u8; len];
    reader.read_exact(&mut body)?;
    Ok(serde_json::from_slice(&body)?)
}

/// Encodes `value` and writes it as one frame.
pub fn write_frame<W: Write, T: Serialize>(
    writer: &mut W,
    value: &T,
    max: usize,
) -> Result<(), FrameError> {
    let frame = encode_frame(value, max)?;
    writer.write_all(&frame)?;
    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MAX_REQUEST_BYTES, Op, QueryRequest};

    #[test]
    fn round_trips_a_request() {
        let req = QueryRequest::new(Op::Hosts);
        let mut buf = Vec::new();
        write_frame(&mut buf, &req, MAX_REQUEST_BYTES).unwrap();
        assert_eq!(&buf[..4], &((buf.len() - 4) as u32).to_be_bytes());
        let back: QueryRequest = read_frame(&mut buf.as_slice(), MAX_REQUEST_BYTES).unwrap();
        assert_eq!(back, req);
    }

    #[test]
    fn refuses_to_encode_an_oversize_body() {
        let req = QueryRequest::new(Op::History {
            host: None,
            series: vec!["x".repeat(MAX_REQUEST_BYTES)],
            from_ms: 0,
            to_ms: 1,
            points: None,
        });
        assert!(matches!(
            encode_frame(&req, MAX_REQUEST_BYTES),
            Err(FrameError::TooLarge {
                max: MAX_REQUEST_BYTES,
                ..
            })
        ));
    }

    #[test]
    fn refuses_an_oversize_prefix_before_reading_the_body() {
        let prefix = ((MAX_REQUEST_BYTES + 1) as u32).to_be_bytes();
        assert!(matches!(
            decode_len(prefix, MAX_REQUEST_BYTES),
            Err(FrameError::TooLarge { len, .. }) if len == MAX_REQUEST_BYTES + 1
        ));
        assert_eq!(decode_len(16u32.to_be_bytes(), 16).unwrap(), 16);
        // Only the prefix is present: the oversize check must fire, not a body read.
        let r: Result<QueryRequest, _> = read_frame(&mut prefix.as_slice(), MAX_REQUEST_BYTES);
        assert!(matches!(r, Err(FrameError::TooLarge { .. })));
    }

    #[test]
    fn reports_truncated_prefix_and_body() {
        let r: Result<QueryRequest, _> = read_frame(&mut [0u8, 0].as_slice(), 64);
        assert!(matches!(r, Err(FrameError::Truncated)));
        let mut buf = encode_frame(&QueryRequest::new(Op::Status), 64).unwrap();
        buf.truncate(buf.len() - 3);
        let r: Result<QueryRequest, _> = read_frame(&mut buf.as_slice(), 64);
        assert!(matches!(r, Err(FrameError::Truncated)));
    }

    #[test]
    fn reports_bad_json() {
        let mut buf = 3u32.to_be_bytes().to_vec();
        buf.extend_from_slice(b"{{{");
        let r: Result<QueryRequest, _> = read_frame(&mut buf.as_slice(), 64);
        assert!(matches!(r, Err(FrameError::Json(_))));
    }
}
