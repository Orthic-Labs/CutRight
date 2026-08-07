//! Untrusted-media validation limits.
//!
//! A worker requests to decode a piece of media, model output or skill
//! asset. Before the harness spends any expensive decode cycle, it asks
//! [`validate_media`] to confirm the supplied metadata is within the
//! hard release-policy limits.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaMetadata {
    pub width: u32,
    pub height: u32,
    pub duration_ms: u32,
    pub stream_count: u32,
    pub compressed_bytes: u64,
    pub decompressed_bytes: u64,
    pub metadata_size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaLimits {
    pub max_width: u32,
    pub max_height: u32,
    pub max_duration_ms: u32,
    pub max_stream_count: u32,
    pub max_compressed_bytes: u64,
    pub max_decompressed_bytes: u64,
    pub max_metadata_size_bytes: u64,
    pub max_decompression_ratio: u32,
}

impl Default for MediaLimits {
    fn default() -> Self {
        Self {
            max_width: 7680,
            max_height: 4320,
            max_duration_ms: 4 * 60 * 60 * 1000,
            max_stream_count: 8,
            max_compressed_bytes: 32 * 1024 * 1024 * 1024,
            max_decompressed_bytes: 256 * 1024 * 1024 * 1024,
            max_metadata_size_bytes: 32 * 1024 * 1024,
            max_decompression_ratio: 64,
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MediaLimitError {
    #[error("dim {width}x{height} exceeds policy floor")]
    DimensionTooLarge {
        width: u32,
        height: u32,
        limits: MediaLimits,
    },
    #[error("duration {0}ms exceeds limit {1}ms")]
    DurationTooLong(u32, u32),
    #[error("stream count {0} exceeds limit {1}")]
    TooManyStreams(u32, u32),
    #[error("compressed bytes {0} exceeds limit {1}")]
    CompressedTooLarge(u64, u64),
    #[error("decompressed bytes {0} exceeds limit {1}")]
    DecompressedTooLarge(u64, u64),
    #[error("metadata size {0} bytes exceeds limit {1}")]
    MetadataTooLarge(u64, u64),
    #[error("decompression ratio exceeds limit {0}")]
    DecompressionRatio(u32),
}

/// Validate [`MediaMetadata`] against the supplied [`MediaLimits`].
pub fn validate_media(
    metadata: &MediaMetadata,
    limits: &MediaLimits,
) -> Result<(), MediaLimitError> {
    if metadata.width > limits.max_width || metadata.height > limits.max_height {
        return Err(MediaLimitError::DimensionTooLarge {
            width: metadata.width,
            height: metadata.height,
            limits: limits.clone(),
        });
    }
    if metadata.duration_ms > limits.max_duration_ms {
        return Err(MediaLimitError::DurationTooLong(
            metadata.duration_ms,
            limits.max_duration_ms,
        ));
    }
    if metadata.stream_count > limits.max_stream_count {
        return Err(MediaLimitError::TooManyStreams(
            metadata.stream_count,
            limits.max_stream_count,
        ));
    }
    if metadata.compressed_bytes > limits.max_compressed_bytes {
        return Err(MediaLimitError::CompressedTooLarge(
            metadata.compressed_bytes,
            limits.max_compressed_bytes,
        ));
    }
    if metadata.decompressed_bytes > limits.max_decompressed_bytes {
        return Err(MediaLimitError::DecompressedTooLarge(
            metadata.decompressed_bytes,
            limits.max_decompressed_bytes,
        ));
    }
    if metadata.metadata_size_bytes > limits.max_metadata_size_bytes {
        return Err(MediaLimitError::MetadataTooLarge(
            metadata.metadata_size_bytes,
            limits.max_metadata_size_bytes,
        ));
    }
    if metadata.compressed_bytes > 0 {
        let ratio = (metadata.decompressed_bytes / metadata.compressed_bytes) as u32;
        if ratio > limits.max_decompression_ratio {
            return Err(MediaLimitError::DecompressionRatio(ratio));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn within_bounds() -> MediaMetadata {
        MediaMetadata {
            width: 1920,
            height: 1080,
            duration_ms: 60_000,
            stream_count: 2,
            compressed_bytes: 1024,
            decompressed_bytes: 4096,
            metadata_size_bytes: 1024,
        }
    }

    #[test]
    fn within_bounds_is_accepted() {
        let l = MediaLimits::default();
        assert!(validate_media(&within_bounds(), &l).is_ok());
    }

    #[test]
    fn dimension_overflow_is_rejected() {
        let l = MediaLimits::default();
        let mut m = within_bounds();
        m.width = 8192;
        m.height = 8192;
        assert!(matches!(
            validate_media(&m, &l),
            Err(MediaLimitError::DimensionTooLarge { .. })
        ));
    }

    #[test]
    fn decompression_bomb_is_rejected() {
        let l = MediaLimits::default();
        let mut m = within_bounds();
        m.compressed_bytes = 1024;
        m.decompressed_bytes = 1024 * 1024;
        assert!(matches!(
            validate_media(&m, &l),
            Err(MediaLimitError::DecompressionRatio(_))
        ));
    }
}
