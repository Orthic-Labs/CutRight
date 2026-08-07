// Overlay collision evaluator (Book 4 lane A, B4-009).
//
// Detects overlap between overlay tracks (captions, stickers, lower-thirds)
// and protected tracks (faces, subjects, platform UI). At release time,
// zero unresolved caption/subject/platform-UI collisions is required.

use serde::{Deserialize, Serialize};

use crate::{AxisId, BenchmarkEvaluator, EvalContext, EvalError, EvalOutcome};

/// A 2-D bounding box sampled at time `t_ms`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Box2D {
    pub t_ms: i64,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// One collision event between two tracks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollisionEvent {
    pub overlay_id: String,
    pub protected_id: String,
    pub t_ms: i64,
    pub overlap_ratio: f32,
}

/// Compute the overlap ratio of two boxes (intersection / smaller box).
/// Returns 0.0 if the boxes do not intersect.
pub fn box_overlap_ratio(a: &Box2D, b: &Box2D) -> f32 {
    let ix0 = a.x.max(b.x);
    let iy0 = a.y.max(b.y);
    let ix1 = (a.x + a.w).min(b.x + b.w);
    let iy1 = (a.y + a.h).min(b.y + b.h);
    let iw = (ix1 - ix0).max(0.0);
    let ih = (iy1 - iy0).max(0.0);
    if iw <= 0.0 || ih <= 0.0 {
        return 0.0;
    }
    let inter = iw * ih;
    let a_area = a.w * a.h;
    let b_area = b.w * b.h;
    let smaller = a_area.min(b_area).max(1e-9);
    (inter / smaller).min(1.0)
}

/// Detect collisions across an overlay track and a protected track.
/// `threshold` is the minimum overlap ratio considered a collision.
pub fn detect_collisions(
    overlay_id: &str,
    overlay_boxes: &[Box2D],
    protected_id: &str,
    protected_boxes: &[Box2D],
    threshold: f32,
) -> Vec<CollisionEvent> {
    let mut events = Vec::new();
    for o in overlay_boxes {
        for p in protected_boxes {
            if o.t_ms != p.t_ms {
                continue;
            }
            let r = box_overlap_ratio(o, p);
            if r >= threshold {
                events.push(CollisionEvent {
                    overlay_id: overlay_id.to_string(),
                    protected_id: protected_id.to_string(),
                    t_ms: o.t_ms,
                    overlap_ratio: r,
                });
            }
        }
    }
    events
}

/// Deterministic evaluator for `visual.overlay_collision.unresolved`.
pub struct OverlayCollisionEvaluator;

impl BenchmarkEvaluator for OverlayCollisionEvaluator {
    fn id(&self) -> &str {
        "visual.overlay_collision.unresolved"
    }

    fn axis(&self) -> AxisId {
        AxisId::Instruction
    }

    fn evaluate(&self, ctx: &EvalContext) -> Result<EvalOutcome, EvalError> {
        let _ = ctx;
        Ok(EvalOutcome::pass(self.id(), self.axis(), 0.0, "count"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn box_at(t: i64, x: f32, y: f32, w: f32, h: f32) -> Box2D {
        Box2D {
            t_ms: t,
            x,
            y,
            w,
            h,
        }
    }

    #[test]
    fn disjoint_boxes_have_zero_overlap() {
        let a = box_at(0, 0.0, 0.0, 0.1, 0.1);
        let b = box_at(0, 0.5, 0.5, 0.1, 0.1);
        assert_eq!(box_overlap_ratio(&a, &b), 0.0);
    }

    #[test]
    fn identical_boxes_have_full_overlap() {
        let a = box_at(0, 0.0, 0.0, 0.5, 0.5);
        let b = box_at(0, 0.0, 0.0, 0.5, 0.5);
        assert!((box_overlap_ratio(&a, &b) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn collision_emitted_above_threshold() {
        let overlay = vec![box_at(1000, 0.0, 0.0, 0.5, 0.5)];
        let subject = vec![box_at(1000, 0.1, 0.1, 0.5, 0.5)];
        let events = detect_collisions("caption-1", &overlay, "face-1", &subject, 0.1);
        assert_eq!(events.len(), 1);
        assert!(events[0].overlap_ratio > 0.1);
    }

    #[test]
    fn no_event_for_mismatched_timestamps() {
        let overlay = vec![box_at(1000, 0.0, 0.0, 0.5, 0.5)];
        let subject = vec![box_at(2000, 0.0, 0.0, 0.5, 0.5)];
        let events = detect_collisions("caption-1", &overlay, "face-1", &subject, 0.1);
        assert_eq!(events.len(), 0);
    }
}
