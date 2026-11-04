#![cfg_attr(test, allow(clippy::too_many_lines))]
//! The public contract between the `statglass` CLI and the app's query socket (D-126,
//! `plan/rfc-agent-cli.md`).
//!
//! | Need | Item |
//! |---|---|
//! | Ask a question | [`QueryRequest`] = [`API_VERSION`] + [`Op`] |
//! | Read the answer | [`QueryResponse`]: `app_version`, `api` and a [`Body`] of [`Answer`] or [`QueryError`] |
//! | Errors and exit codes | [`ErrorTag`] (fixed message, exit code, retryable), [`ErrorDetails`], [`CliErrorEnvelope`] |
//! | Put bytes on the socket | [`frame`]: u32 big-endian length plus JSON, capped at [`MAX_REQUEST_BYTES`] / [`MAX_RESPONSE_BYTES`] |
//! | Talk to the app | [`Client`] (blocking, std only), [`AppPaths`] |
//! | Did-you-mean | [`suggest::closest`] |
//! | Times in and out | [`time`]: RFC 3339 in the system zone, durations, bare times |
//! | `statglass schema` | [`schemas`] |
//!
//! The DTOs are a deliberate second set of types, not a mirror of `statglass-schema` or
//! the app's IPC types. They evolve additively: new fields are `Option` and skipped when
//! absent, new enum values are added, nothing is renamed or removed. Every enum that a
//! newer app can send has an `Unknown` fallback, so an older CLI decodes the answer and
//! treats the value as "other".
//!
//! No tokio, store, engine or remote dependency: the CLI cannot reach storage.

mod answer;
mod client;
mod doctor;
mod error;
pub mod frame;
mod paths;
mod registry;
mod request;
mod response;
pub mod suggest;
pub mod time;

pub use answer::*;
pub use client::{Client, ClientError, REQUEST_TIMEOUT};
pub use doctor::{AccessState, DoctorProblem, DoctorReport};
pub use error::{CliError, CliErrorEnvelope, ErrorDetails, ErrorTag, QueryError};
pub use frame::FrameError;
pub use paths::{AppPaths, DEFAULT_BUNDLE_ID};
pub use registry::{SchemaEntry, schemas};
pub use request::{Op, QueryRequest, UsageBy};
pub use response::{Answer, Body, QueryResponse};

/// The contract version every request and response carries. It increments only for a
/// breaking change; additive changes keep it.
pub const API_VERSION: u16 = 1;

/// Largest request frame body the app accepts.
pub const MAX_REQUEST_BYTES: usize = 16 * 1024;

/// Largest response frame body the app sends and the client reads.
pub const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

#[cfg(test)]
mod tests;
