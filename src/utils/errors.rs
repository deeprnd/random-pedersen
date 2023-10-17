#[cfg(feature = "std")]
use std::fmt;

use axum::http::StatusCode;
use bulletproofs::ProofError;

#[derive(Debug)]
pub struct CacheError;

// Custom error type for random generation errors
#[derive(Debug)]
pub struct RandomGenerationError;

// Implement From trait for ring::error::Unspecified for RandomGenerationError
impl From<ring::error::Unspecified> for RandomGenerationError {
    fn from(_: ring::error::Unspecified) -> Self {
        RandomGenerationError
    }
}

// Implement To trait StatusCode for RandomGenerationError
impl Into<StatusCode> for RandomGenerationError {
    fn into(self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

// Implement Display trait for RandomGenerationError
#[cfg(feature = "std")]
impl fmt::Display for RandomGenerationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error generating random bytes")
    }
}

// Custom error type for random generation errors
#[derive(Debug)]
pub struct CommitmentGenerationError;

// Implement From trait ProofError for CommitmentGenerationError
impl From<ProofError> for CommitmentGenerationError {
    fn from(_: ProofError) -> Self {
        CommitmentGenerationError
    }
}

// Implement To trait StatusCode for CommitmentGenerationError
impl Into<StatusCode> for CommitmentGenerationError {
    fn into(self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

// Implement Display trait for RandomGenerationError
#[cfg(feature = "std")]
impl fmt::Display for CommitmentGenerationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error generating random bytes")
    }
}
