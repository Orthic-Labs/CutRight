//! Immutable rough-cut and finish contracts shared by project and render lanes.
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RationalRate {
    pub numerator: u32,
    pub denominator: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RationalTime {
    pub numerator: i64,
    pub denominator: u32,
}

impl RationalTime {
    pub fn new(numerator: i64, denominator: u32) -> Result<Self, LockError> {
        if denominator == 0 {
            return Err(LockError::InvalidTime);
        }
        Ok(Self {
            numerator,
            denominator,
        })
    }
    pub fn millis(ms: i64) -> Self {
        Self {
            numerator: ms,
            denominator: 1000,
        }
    }
    pub fn as_f64(self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WordSafeSegment {
    pub source_id: String,
    pub source_in: RationalTime,
    pub source_out: RationalTime,
    pub timeline_in: RationalTime,
    pub timeline_out: RationalTime,
    pub first_word_id: String,
    pub last_word_id: String,
    #[serde(default)]
    pub speech_region_ids: Vec<String>,
    #[serde(default)]
    pub gap: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LockedCut {
    pub schema_version: u32,
    pub cut_plan_sha256: String,
    pub timeline_rate: RationalRate,
    pub segments: Vec<WordSafeSegment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WordSpan {
    pub first_word_id: String,
    pub last_word_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuditionChoice {
    pub variant_id: String,
    pub source_hashes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssetCandidateScore {
    pub asset_id: String,
    pub meaning: f32,
    pub motion_direction: f32,
    pub source_focus: f32,
    pub lifetime: f32,
    pub negative_space: f32,
    pub crop_safety: f32,
    pub license: f32,
    pub availability: f32,
    #[serde(default)]
    pub rejection_reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VariantSpec {
    pub variant_id: String,
    pub seed: u64,
    pub source_hash: String,
}

pub fn deterministic_variants(intervention: &str, source_hash: &str) -> Vec<VariantSpec> {
    (0..4)
        .map(|i| VariantSpec {
            variant_id: format!("{intervention}-v{}", i + 1),
            seed: blake3::hash(format!("{source_hash}:{i}").as_bytes()).as_bytes()[0] as u64,
            source_hash: source_hash.into(),
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditorTakeover {
    pub asset_id: String,
    pub span: WordSpan,
    pub lifetime_start_ms: i64,
    pub lifetime_end_ms: i64,
}

impl EditorTakeover {
    pub fn validate(&self, cut: &LockedCut) -> Result<(), LockError> {
        if self.asset_id.trim().is_empty()
            || self.lifetime_start_ms < 0
            || self.lifetime_end_ms <= self.lifetime_start_ms
        {
            return Err(LockError::InvalidTime);
        }
        let Some(segment) = cut.segments.iter().find(|segment| {
            !segment.gap
                && segment.first_word_id == self.span.first_word_id
                && segment.last_word_id == self.span.last_word_id
        }) else {
            return Err(LockError::InvalidTime);
        };
        let start_ms = (segment.timeline_in.as_f64() * 1000.0).round() as i64;
        let end_ms = (segment.timeline_out.as_f64() * 1000.0).round() as i64;
        if self.lifetime_start_ms < start_ms || self.lifetime_end_ms > end_ms {
            return Err(LockError::InvalidTime);
        }
        Ok(())
    }
}

impl AssetCandidateScore {
    pub fn total(&self) -> f32 {
        [
            self.meaning,
            self.motion_direction,
            self.source_focus,
            self.lifetime,
            self.negative_space,
            self.crop_safety,
            self.license,
            self.availability,
        ]
        .into_iter()
        .sum()
    }

    pub fn is_eligible(&self) -> bool {
        let scores = [
            self.meaning,
            self.motion_direction,
            self.source_focus,
            self.lifetime,
            self.negative_space,
            self.crop_safety,
            self.license,
            self.availability,
        ];
        scores
            .iter()
            .all(|score| score.is_finite() && (0.0..=1.0).contains(score))
            && self.license > 0.0
            && self.availability > 0.0
            && self.rejection_reasons.is_empty()
    }
}

pub fn rank_asset_candidates(mut candidates: Vec<AssetCandidateScore>) -> Vec<AssetCandidateScore> {
    candidates.retain(AssetCandidateScore::is_eligible);
    candidates.sort_by(|left, right| {
        right
            .total()
            .total_cmp(&left.total())
            .then_with(|| left.asset_id.cmp(&right.asset_id))
    });
    candidates
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FinishPlan {
    pub schema_version: u32,
    pub locked_cut_sha256: String,
    pub graph: video_core::FinishRenderGraph,
    #[serde(default)]
    pub audition_choice: Option<AuditionChoice>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LockError {
    #[error("invalid rational time")]
    InvalidTime,
    #[error("cut lock hash mismatch")]
    HashMismatch,
}

impl LockedCut {
    pub fn canonical_hash(&self) -> String {
        // Hash canonical cut content, excluding stored digest to avoid a
        // self-referential value that can never validate after persistence.
        let mut canonical = self.clone();
        canonical.cut_plan_sha256.clear();
        let bytes = serde_json::to_vec(&canonical).expect("locked cut serializes");
        sha256_hex(&bytes)
    }
    pub fn validate(&self) -> Result<(), LockError> {
        if self.schema_version != 1
            || self.segments.is_empty()
            || self.timeline_rate.numerator == 0
            || self.timeline_rate.denominator == 0
        {
            return Err(LockError::InvalidTime);
        }
        for segment in &self.segments {
            let times = [
                segment.source_in,
                segment.source_out,
                segment.timeline_in,
                segment.timeline_out,
            ];
            if times
                .iter()
                .any(|time| time.denominator == 0 || time.numerator < 0)
                || segment.source_id.trim().is_empty()
                || segment.source_out.as_f64() <= segment.source_in.as_f64()
                || segment.timeline_out.as_f64() <= segment.timeline_in.as_f64()
            {
                return Err(LockError::InvalidTime);
            }
            if segment.first_word_id.is_empty() || segment.last_word_id.is_empty() {
                return Err(LockError::InvalidTime);
            }
            if !segment.gap && segment.speech_region_ids.is_empty() {
                return Err(LockError::InvalidTime);
            }
        }
        Ok(())
    }
    pub fn assert_hash(&self, expected: &str) -> Result<(), LockError> {
        if self.canonical_hash() == expected {
            Ok(())
        } else {
            Err(LockError::HashMismatch)
        }
    }
}

pub fn compile_locked_cut(mut cut: LockedCut) -> Result<LockedCut, LockError> {
    cut.validate()?;
    cut.cut_plan_sha256 = cut.canonical_hash();
    Ok(cut)
}

/// Compile only when forced-word IDs and VAD evidence cover every non-gap
/// segment; raw ASR timestamps cannot satisfy this boundary.
pub fn compile_locked_cut_from_evidence(
    mut cut: LockedCut,
    words: &[video_core::Word],
    vad: &[video_core::VadRegion],
) -> Result<LockedCut, LockError> {
    for segment in &cut.segments {
        if segment.gap {
            continue;
        }
        let first = words
            .iter()
            .find(|w| w.id == segment.first_word_id)
            .ok_or(LockError::InvalidTime)?;
        let last = words
            .iter()
            .find(|w| w.id == segment.last_word_id)
            .ok_or(LockError::InvalidTime)?;
        let forced_word = |word: &video_core::Word| {
            word.kind == "word"
                && word
                    .source_word_id
                    .as_deref()
                    .is_some_and(|id| id.starts_with(&format!("{}:", segment.source_id)))
        };
        if !forced_word(first) || !forced_word(last) {
            return Err(LockError::InvalidTime);
        }
        if first.start_ms != (segment.source_in.as_f64() * 1000.0).round() as i64
            || last.end_ms != (segment.source_out.as_f64() * 1000.0).round() as i64
        {
            return Err(LockError::InvalidTime);
        }
        if !vad
            .iter()
            .any(|r| r.start_ms <= first.start_ms && r.end_ms >= last.end_ms)
        {
            return Err(LockError::InvalidTime);
        }
    }
    cut.validate()?;
    cut.cut_plan_sha256 = cut.canonical_hash();
    Ok(cut)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cut() -> LockedCut {
        LockedCut {
            schema_version: 1,
            cut_plan_sha256: String::new(),
            timeline_rate: RationalRate {
                numerator: 30,
                denominator: 1,
            },
            segments: vec![WordSafeSegment {
                source_id: "cam-a".into(),
                source_in: RationalTime::millis(0),
                source_out: RationalTime::millis(1000),
                timeline_in: RationalTime::millis(0),
                timeline_out: RationalTime::millis(1000),
                first_word_id: "w1".into(),
                last_word_id: "w2".into(),
                speech_region_ids: vec!["r1".into()],
                gap: false,
            }],
        }
    }
    #[test]
    fn lock_hash_is_stable_and_detects_mutation() {
        let locked = compile_locked_cut(cut()).unwrap();
        let hash = locked.cut_plan_sha256.clone();
        assert_eq!(locked.canonical_hash(), hash);
        assert!(locked.assert_hash(&hash).is_ok());
        let mut changed = locked;
        changed.segments[0].source_out = RationalTime::millis(900);
        assert!(matches!(
            changed.assert_hash(&hash),
            Err(LockError::HashMismatch)
        ));
    }

    #[test]
    fn canonical_hash_is_sha256() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn rejects_unknown_schema_and_zero_denominator() {
        let mut invalid = cut();
        invalid.schema_version = 2;
        assert_eq!(invalid.validate(), Err(LockError::InvalidTime));
        invalid.schema_version = 1;
        invalid.segments[0].source_in.denominator = 0;
        assert_eq!(invalid.validate(), Err(LockError::InvalidTime));
    }
}

fn sha256_hex(input: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut bytes = input.to_vec();
    let bit_len = (bytes.len() as u64) * 8;
    bytes.push(0x80);
    while bytes.len() % 64 != 56 {
        bytes.push(0);
    }
    bytes.extend_from_slice(&bit_len.to_be_bytes());
    let mut h = [
        0x6a09e667_u32,
        0xbb67ae85,
        0x3c6ef372,
        0xa54ff53a,
        0x510e527f,
        0x9b05688c,
        0x1f83d9ab,
        0x5be0cd19,
    ];
    for chunk in bytes.chunks_exact(64) {
        let mut w = [0_u32; 64];
        for (index, word) in chunk.chunks_exact(4).enumerate() {
            w[index] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for index in 16..64 {
            let s0 = w[index - 15].rotate_right(7)
                ^ w[index - 15].rotate_right(18)
                ^ (w[index - 15] >> 3);
            let s1 = w[index - 2].rotate_right(17)
                ^ w[index - 2].rotate_right(19)
                ^ (w[index - 2] >> 10);
            w[index] = w[index - 16]
                .wrapping_add(s0)
                .wrapping_add(w[index - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choice = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(choice)
                .wrapping_add(K[index])
                .wrapping_add(w[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(majority);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (state, value) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *state = state.wrapping_add(value);
        }
    }
    h.iter().map(|word| format!("{word:08x}")).collect()
}
