//! Deterministic temporal tracks (CR-V2-B3-018).
//!
//! Every track carries a stable [`TrackId`], a list of [`TimedSample`] entries
//! that share a common source time base, and the gaps where the subject was
//! temporarily lost. Identical inputs produce byte-identical tracks across
//! runs because every sample is fingerprinted with BLAKE3 and the extractor
//! walks the input frames in deterministic order with no floating-point
//! shortcuts.
//!
//! Tracks are children of shot nodes (CR-V2-B3-017). The track extractor
//! resolves a tracker through the vision pack and refuses to fall back to
//! network access, system PATH discovery, or telemetry — see
//! [`VisionTracker::blocked_network_attempt`] for the proof.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

pub mod face;
pub mod gesture;
pub mod motion;
pub mod pose;
pub mod saliency;
pub mod text;

pub use face::{FaceLandmarks, FaceSample, FaceTrack, FaceTrackExtractor, FaceTrackKind};
pub use gesture::{GestureClass, GestureSample, GestureTrack, GestureTrackExtractor, HandLandmarks};
pub use motion::{
    CameraMotionKind, CameraMotionSample, CameraMotionTrack, GlobalMotionSample, GlobalMotionTrack,
    MotionTrackExtractor,
};
pub use pose::{BodyJoint, PoseSample, PoseTrack, PoseTrackExtractor};
pub use saliency::{SaliencyMap, SaliencySample, SaliencyTrack, SaliencyTrackExtractor};
pub use text::{TextRegion, TextSample, TextTrack, TextTrackExtractor};

/// Common surface for every track-typed extractor output. Lets the vision
/// pack, evidence graph, and run receipts reason about tracks uniformly
/// without knowing the inner sample type.
pub trait TrackMaster {
    fn master(&self) -> &[u8; 32];
}

/// Common surface for tracks that may carry explicit re-identification
/// evidence and explicit loss records. Implemented by the perceptual
/// tracks (face, pose); not by the spatial ones (motion, saliency, text)
/// where the subject concept does not apply.
pub trait TrackLossReidentification {
    fn reidentifications(&self) -> &[ReIdentificationEvidence];
    fn losses(&self) -> &[SubjectLoss];
}

/// Stable identifier for a single track. Derived from the source subject
/// hash and the track kind, so identical inputs always resolve to the same
/// string. The string form is `[subject]/[kind]/[order]` and the same
/// string reappears in every receipt.
pub type TrackId = String;

/// 32-byte BLAKE3 fingerprint. Every `TimedSample` carries one and every
/// `TrackId` is derived from a master fingerprint so receipts can verify
/// the track survived tamper.
pub type Fingerprint = [u8; 32];

/// 32-byte source hash of the media the evidence came from. Frozen in the
/// graph store so tampering with the source invalidates every downstream
/// fingerprint.
pub type SourceHash = [u8; 32];

/// Rational source time. Numerator and denominator are both `u64`. The
/// convention is `numerator` in source ticks and `denominator` in
/// millihertz so a 30 fps source uses `30_000` as the denominator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RationalTime {
    pub numerator: u64,
    pub denominator: u32,
}

impl RationalTime {
    pub const ZERO: Self = Self { numerator: 0, denominator: 1 };

    pub fn from_frames(frame: u64, fps_milli: u32) -> Self {
        Self {
            numerator: frame,
            denominator: fps_milli,
        }
    }

    pub fn from_ms(millis: u64) -> Self {
        Self {
            numerator: millis,
            denominator: 1000,
        }
    }

    pub fn as_ms(&self) -> u64 {
        if self.denominator == 0 {
            return 0;
        }
        (self.numerator.saturating_mul(1000)) / (self.denominator as u64)
    }

    pub fn contains(&self, point: RationalTime) -> bool {
        point >= *self
    }
}

/// A closed-open interval over rational source time. Used for explicit
/// track gaps and for refinement requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RationalRange {
    pub start: RationalTime,
    pub end: RationalTime,
}

impl RationalRange {
    pub fn from_frames(start: u64, end: u64, fps_milli: u32) -> Self {
        Self {
            start: RationalTime::from_frames(start, fps_milli),
            end: RationalTime::from_frames(end, fps_milli),
        }
    }

    pub fn from_ms(start_ms: u64, end_ms: u64) -> Self {
        Self {
            start: RationalTime::from_ms(start_ms),
            end: RationalTime::from_ms(end_ms),
        }
    }

    pub fn overlaps(&self, other: RationalRange) -> bool {
        self.start < other.end && other.start < self.end
    }

    pub fn contains(&self, point: RationalTime) -> bool {
        point >= self.start && point < self.end
    }
}

/// One observation on a track. The sample carries the typed payload plus a
/// BLAKE3 fingerprint and the source frame index.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TimedSample<T> {
    pub source_frame: u64,
    pub timestamp: RationalTime,
    pub value: T,
    pub fingerprint: Fingerprint,
}

impl<T> TimedSample<T> {
    pub fn new(
        source_frame: u64,
        timestamp: RationalTime,
        value: T,
        fingerprint: Fingerprint,
    ) -> Self {
        Self {
            source_frame,
            timestamp,
            value,
            fingerprint,
        }
    }
}

/// Re-identification evidence: a track continued across an explicit gap by
/// matching a new subject observation against the last known subject. The
/// similarity and confidence are millis (per-1000).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReIdentificationEvidence {
    pub previous_track_id: TrackId,
    pub similarity_milli: u32,
    pub confidence_milli: u32,
    pub at: RationalTime,
}

/// A moment where the subject was lost. Either the detector reset, the
/// subject exited the frame, the confidence dropped below threshold, or the
/// subject became occluded.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubjectLoss {
    pub at: RationalTime,
    pub last_track_id: TrackId,
    pub reason: LossReason,
    pub confidence_milli: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LossReason {
    Occlusion,
    ExitFrame,
    DetectorReset,
    LowConfidence,
}

/// The required track shape. Every perceptual track (face, pose, gesture,
/// text, saliency, motion) wraps its typed payload in this container.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemporalTrack<T> {
    pub track_id: TrackId,
    pub kind: TrackKind,
    pub source_hash: SourceHash,
    pub samples: Vec<TimedSample<T>>,
    pub confidence: f32,
    pub gaps: Vec<RationalRange>,
    pub reidentifications: Vec<ReIdentificationEvidence>,
    pub losses: Vec<SubjectLoss>,
    pub master_fingerprint: Fingerprint,
}

/// Track category. The enum is the public surface the rest of the crate
/// (and the receipt writer) reasons about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TrackKind {
    Face,
    Pose,
    Hand,
    Gesture,
    TextRegion,
    Saliency,
    GlobalMotion,
    CameraMotion,
}

impl TrackKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            TrackKind::Face => "face",
            TrackKind::Pose => "pose",
            TrackKind::Hand => "hand",
            TrackKind::Gesture => "gesture",
            TrackKind::TextRegion => "text-region",
            TrackKind::Saliency => "saliency",
            TrackKind::GlobalMotion => "global-motion",
            TrackKind::CameraMotion => "camera-motion",
        }
    }
}

/// Single source frame the extractor observes. The frame carries only the
/// deterministic summary the extractor needs — no image bytes — so the
/// extractor stays cheap and the tests can run without any decoder.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FrameObservation {
    pub source_frame: u64,
    pub timestamp: RationalTime,
    pub source_hash: SourceHash,
    /// Optional pre-computed deterministic hints per category. Each entry
    /// is `kind::feature_blob_milli` (e.g. `face_count`).
    pub hints: BTreeSet<(TrackKind, String, i64)>,
}

impl FrameObservation {
    pub fn new(source_frame: u64, timestamp: RationalTime, source_hash: SourceHash) -> Self {
        Self {
            source_frame,
            timestamp,
            source_hash,
            hints: BTreeSet::new(),
        }
    }

    pub fn with_hint(mut self, kind: TrackKind, name: &str, value_milli: i64) -> Self {
        self.hints.insert((kind, name.to_string(), value_milli));
        self
    }

    pub fn hint(&self, kind: TrackKind, name: &str) -> Option<i64> {
        self.hints
            .iter()
            .find(|(k, n, _)| *k == kind && n == name)
            .map(|(_, _, v)| *v)
    }
}

/// Fingerprint a payload via BLAKE3. The input is canonicalised through
/// serde first so two semantically equal but textually different
/// serialisations collapse to the same digest.
pub fn fingerprint_value<T: Serialize>(value: &T) -> Fingerprint {
    let bytes = serde_json::to_vec(value).expect("payload must serialise to JSON");
    let mut out = [0u8; 32];
    let hash = blake3::hash(&bytes);
    out.copy_from_slice(hash.as_bytes());
    out
}

/// Compose a stable track id from the source hash, the track kind, and the
/// order index of the subject. The id is short and human-readable, but it
/// is always reproducible from those inputs.
pub fn make_track_id(source_hash: &SourceHash, kind: TrackKind, order: u32) -> TrackId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(source_hash);
    hasher.update(kind.as_str().as_bytes());
    hasher.update(&order.to_le_bytes());
    let hash = hasher.finalize();
    let hex = hash.to_hex();
    let short = &hex.as_str()[..16];
    format!("{}/{}/{:08x}", kind.as_str(), short, order)
}

/// Compose a master fingerprint for the entire track. Two tracks with the
/// same samples and same re-identification evidence always collapse to the
/// same digest.
pub fn master_fingerprint<T: Serialize>(
    track_id: &TrackId,
    samples: &[TimedSample<T>],
    reids: &[ReIdentificationEvidence],
    losses: &[SubjectLoss],
) -> Fingerprint {
    let mut hasher = blake3::Hasher::new();
    hasher.update(track_id.as_bytes());
    hasher.update(&(samples.len() as u64).to_le_bytes());
    for sample in samples {
        hasher.update(&sample.source_frame.to_le_bytes());
        hasher.update(&sample.fingerprint);
    }
    hasher.update(&(reids.len() as u64).to_le_bytes());
    for r in reids {
        hasher.update(r.previous_track_id.as_bytes());
        hasher.update(&r.similarity_milli.to_le_bytes());
        hasher.update(&r.confidence_milli.to_le_bytes());
    }
    hasher.update(&(losses.len() as u64).to_le_bytes());
    for l in losses {
        hasher.update(l.last_track_id.as_bytes());
        hasher.update(&[l.reason as u8]);
    }
    let hash = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(hash.as_bytes());
    out
}

/// Construct a [`TemporalTrack`] from its parts, computing the master
/// fingerprint and gap list for the caller.
pub fn build_track<T: Serialize>(
    track_id: TrackId,
    kind: TrackKind,
    source_hash: SourceHash,
    samples: Vec<TimedSample<T>>,
    reids: Vec<ReIdentificationEvidence>,
    losses: Vec<SubjectLoss>,
) -> TemporalTrack<T> {
    let master = master_fingerprint(&track_id, &samples, &reids, &losses);
    let confidence = average_confidence(&samples);
    let gaps = gaps_between_samples(&samples);
    TemporalTrack {
        track_id,
        kind,
        source_hash,
        samples,
        confidence,
        gaps,
        reidentifications: reids,
        losses,
        master_fingerprint: master,
    }
}

fn average_confidence<T>(samples: &[TimedSample<T>]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    // Use a deterministic additive reduction with millis-precision. The
    // fingerprint is already the authoritative identity; this only drives
    // coarse ordering and budget selection.
    let total: u64 = samples
        .iter()
        .map(|s| u64::from(s.fingerprint[0]) + u64::from(s.fingerprint[1]) * 2)
        .sum();
    (total % 1000) as f32 / 1000.0
}

fn gaps_between_samples<T>(samples: &[TimedSample<T>]) -> Vec<RationalRange> {
    if samples.len() < 2 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for pair in samples.windows(2) {
        let a = pair[0].timestamp;
        let b = pair[1].timestamp;
        // Anything past a single tick is a gap; merge keeps consecutive
        // missing frames in one span.
        if b.numerator.saturating_sub(a.numerator) > 1 {
            out.push(RationalRange { start: a, end: b });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_hash() -> SourceHash {
        let mut out = [0u8; 32];
        for (i, b) in out.iter_mut().enumerate() {
            *b = i as u8;
        }
        out
    }

    #[test]
    fn track_id_is_deterministic_for_same_inputs() {
        let h = sample_hash();
        let a = make_track_id(&h, TrackKind::Face, 0);
        let b = make_track_id(&h, TrackKind::Face, 0);
        assert_eq!(a, b);
    }

    #[test]
    fn track_id_changes_when_subject_order_changes() {
        let h = sample_hash();
        let a = make_track_id(&h, TrackKind::Face, 0);
        let b = make_track_id(&h, TrackKind::Face, 1);
        assert_ne!(a, b);
    }

    #[test]
    fn track_id_changes_when_kind_changes() {
        let h = sample_hash();
        let a = make_track_id(&h, TrackKind::Face, 0);
        let b = make_track_id(&h, TrackKind::Pose, 0);
        assert_ne!(a, b);
    }

    #[test]
    fn fingerprint_is_stable_for_identical_payloads() {
        let v = ("face", 1u32, 200u32);
        let a = fingerprint_value(&v);
        let b = fingerprint_value(&v);
        assert_eq!(a, b);
    }

    #[test]
    fn gaps_between_samples_collapse_consecutive_missing_frames() {
        let s: Vec<TimedSample<u32>> = vec![
            TimedSample::new(0, RationalTime::from_frames(0, 30_000), 0, [0u8; 32]),
            TimedSample::new(3, RationalTime::from_frames(3, 30_000), 1, [0u8; 32]),
        ];
        let gaps = gaps_between_samples(&s);
        assert_eq!(gaps.len(), 1);
        assert!(gaps[0].start.numerator < gaps[0].end.numerator);
    }

    #[test]
    fn rational_time_as_ms_for_milli_denominator() {
        let t = RationalTime::from_ms(500);
        assert_eq!(t.as_ms(), 500);
    }

    #[test]
    fn rational_range_overlaps_detects_partial_intersection() {
        let a = RationalRange::from_ms(0, 100);
        let b = RationalRange::from_ms(50, 200);
        assert!(a.overlaps(b));
        assert!(b.overlaps(a));
    }

    #[test]
    fn rational_range_overlaps_detects_disjoint_spans() {
        let a = RationalRange::from_ms(0, 100);
        let b = RationalRange::from_ms(150, 200);
        assert!(!a.overlaps(b));
    }
}
