//! Pose tracks (CR-V2-B3-018).
//!
//! Tracks a small set of canonical body joints (`head`, `neck`, `shoulders`,
//! `elbows`, `wrists`, `hips`, `knees`, `ankles`) for every detected
//! subject. Joints are stored as integer millis in the normalized frame
//! box so the extractor never touches floating-point arithmetic and the
//! output fingerprint stays stable.

use serde::{Deserialize, Serialize};

use super::{
    build_track, fingerprint_value, FrameObservation, LossReason, ReIdentificationEvidence,
    SubjectLoss, TimedSample, TrackKind, TrackLossReidentification, TrackMaster,
};

fn u32_hint(obs: &FrameObservation, kind: TrackKind, name: &str) -> u32 {
    obs.hint(kind, name).unwrap_or(0).max(0) as u32
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BodyJoint {
    Head,
    Neck,
    LeftShoulder,
    RightShoulder,
    LeftElbow,
    RightElbow,
    LeftWrist,
    RightWrist,
    LeftHip,
    RightHip,
    LeftKnee,
    RightKnee,
    LeftAnkle,
    RightAnkle,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PoseSample {
    pub joints: Vec<(BodyJoint, i32, i32, u32)>,
}

impl PoseSample {
    pub fn confidence_milli(&self) -> u32 {
        self.joints
            .iter()
            .map(|(_, _, _, c)| *c)
            .max()
            .unwrap_or(0)
    }
}

pub struct PoseTrackExtractor {
    pub reid_similarity_milli: u32,
}

impl Default for PoseTrackExtractor {
    fn default() -> Self {
        Self {
            reid_similarity_milli: 700,
        }
    }
}

impl PoseTrackExtractor {
    pub fn extract(
        &self,
        observations: &[FrameObservation],
        order: u32,
        source_hash: [u8; 32],
    ) -> Option<PoseTrack> {
        let mut samples = Vec::new();
        let mut reids = Vec::new();
        let mut losses = Vec::new();
        let mut last_pose_id: Option<[u8; 32]> = None;
        let mut absent_streak: u32 = 0;
        for obs in observations {
            let count = obs.hint(TrackKind::Pose, &format!("order{order}_count"));
            let Some(count) = count else {
                absent_streak = absent_streak.saturating_add(1);
                continue;
            };
            if count == 0 {
                absent_streak = absent_streak.saturating_add(1);
                continue;
            }
            let mut joints = Vec::new();
            for joint in [
                BodyJoint::Head,
                BodyJoint::Neck,
                BodyJoint::LeftShoulder,
                BodyJoint::RightShoulder,
                BodyJoint::LeftElbow,
                BodyJoint::RightElbow,
                BodyJoint::LeftWrist,
                BodyJoint::RightWrist,
                BodyJoint::LeftHip,
                BodyJoint::RightHip,
                BodyJoint::LeftKnee,
                BodyJoint::RightKnee,
                BodyJoint::LeftAnkle,
                BodyJoint::RightAnkle,
            ] {
                let x = obs
                    .hint(TrackKind::Pose, &format!("order{order}_{joint:?}_x"))
                    .unwrap_or(0)
                    .clamp(i32::MIN as i64, i32::MAX as i64)
                    as i32;
                let y = obs
                    .hint(TrackKind::Pose, &format!("order{order}_{joint:?}_y"))
                    .unwrap_or(0)
                    .clamp(i32::MIN as i64, i32::MAX as i64)
                    as i32;
                let conf = u32_hint(obs, TrackKind::Pose, &format!("order{order}_{joint:?}_conf"));
                joints.push((joint, x, y, conf));
            }
            let sample = PoseSample { joints };
            let fp = fingerprint_value(&sample);
            if absent_streak > 0 {
                if let Some(prev) = last_pose_id {
                    let sim = similarity_milli(&fp, &prev);
                    if sim >= self.reid_similarity_milli {
                        reids.push(ReIdentificationEvidence {
                            previous_track_id: format!("pose/{order:08x}"),
                            similarity_milli: sim,
                            confidence_milli: sample.confidence_milli(),
                            at: obs.timestamp,
                        });
                    }
                    losses.push(SubjectLoss {
                        at: obs.timestamp,
                        last_track_id: format!("pose/{order:08x}"),
                        reason: LossReason::LowConfidence,
                        confidence_milli: sample.confidence_milli(),
                    });
                }
            }
            absent_streak = 0;
            last_pose_id = Some(fp);
            samples.push(TimedSample::new(obs.source_frame, obs.timestamp, sample, fp));
        }
        if samples.is_empty() {
            return None;
        }
        let track_id = super::make_track_id(&source_hash, TrackKind::Pose, order);
        let master = super::master_fingerprint(&track_id, &samples, &reids, &losses);
        Some(PoseTrack {
            inner: build_track(track_id, TrackKind::Pose, source_hash, samples, reids, losses),
            master_fingerprint: master,
        })
    }
}

fn similarity_milli(a: &[u8; 32], b: &[u8; 32]) -> u32 {
    let matching = a.iter().zip(b.iter()).filter(|(x, y)| x == y).count() as u32;
    (matching.saturating_mul(1000)) / 32
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoseTrack {
    #[serde(flatten)]
    pub inner: super::TemporalTrack<PoseSample>,
    pub master_fingerprint: [u8; 32],
}

impl TrackMaster for PoseTrack {
    fn master(&self) -> &[u8; 32] {
        &self.master_fingerprint
    }
}

impl TrackLossReidentification for PoseTrack {
    fn reidentifications(&self) -> &[ReIdentificationEvidence] {
        &self.inner.reidentifications
    }
    fn losses(&self) -> &[SubjectLoss] {
        &self.inner.losses
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracks::RationalTime;

    fn obs(frame: u64, order: u32) -> FrameObservation {
        let mut o = FrameObservation::new(frame, RationalTime::from_frames(frame, 30_000), [4u8; 32]);
        o = o.with_hint(TrackKind::Pose, &format!("order{order}_count"), 700);
        o
    }

    #[test]
    fn pose_track_is_deterministic_for_same_observations() {
        let ex = PoseTrackExtractor::default();
        let obs = vec![obs(0, 0), obs(30, 0)];
        let a = ex.extract(&obs, 0, [4u8; 32]).unwrap();
        let b = ex.extract(&obs, 0, [4u8; 32]).unwrap();
        assert_eq!(a.inner.samples.len(), b.inner.samples.len());
        assert_eq!(a.master_fingerprint, b.master_fingerprint);
    }

    #[test]
    fn pose_track_is_none_without_subject() {
        let ex = PoseTrackExtractor::default();
        let empty: Vec<FrameObservation> = vec![];
        assert!(ex.extract(&empty, 0, [0u8; 32]).is_none());
    }
}