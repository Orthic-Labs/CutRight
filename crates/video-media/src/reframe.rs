//! Anchor-driven crop/reframe filter construction for vertical/square
//! delivery presets.

use crate::probe::MediaMetadata;
use crate::RenderError;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReframeAnchor {
    pub output_start_ms: i64,
    pub center_x: f64,
    pub center_y: f64,
}

pub(crate) fn reframe_filter(
    metadata: &MediaMetadata,
    width: u32,
    height: u32,
    anchors: Option<&[ReframeAnchor]>,
) -> Result<String, RenderError> {
    let Some(anchors) = anchors.filter(|anchors| !anchors.is_empty()) else {
        return Ok(format!(
            "scale={width}:{height}:force_original_aspect_ratio=increase,crop={width}:{height},setsar=1"
        ));
    };
    let input_width = metadata
        .width
        .ok_or_else(|| RenderError::Failed("reframe input has no width".into()))?
        as f64;
    let input_height = metadata
        .height
        .ok_or_else(|| RenderError::Failed("reframe input has no height".into()))?
        as f64;
    let scale = (width as f64 / input_width).max(height as f64 / input_height);
    let scaled_width = (input_width * scale).round().max(width as f64) as u32;
    let scaled_height = (input_height * scale).round().max(height as f64) as u32;
    let crop_x = |anchor: &ReframeAnchor| {
        ((scaled_width as f64 * anchor.center_x.clamp(0.0, 1.0) - width as f64 / 2.0)
            .clamp(0.0, (scaled_width - width) as f64))
        .round() as u32
    };
    let crop_y = |anchor: &ReframeAnchor| {
        ((scaled_height as f64 * anchor.center_y.clamp(0.0, 1.0) - height as f64 / 2.0)
            .clamp(0.0, (scaled_height - height) as f64))
        .round() as u32
    };
    let initial = anchors[0];
    let commands = anchors
        .iter()
        .map(|anchor| {
            format!(
                "{:.3} crop@reframe x {};{:.3} crop@reframe y {}",
                anchor.output_start_ms.max(0) as f64 / 1_000.0,
                crop_x(anchor),
                anchor.output_start_ms.max(0) as f64 / 1_000.0,
                crop_y(anchor)
            )
        })
        .collect::<Vec<_>>()
        .join(";");
    Ok(format!(
        "scale={scaled_width}:{scaled_height},sendcmd=c='{commands}',crop@reframe={width}:{height}:x={}:y={},setsar=1",
        crop_x(&initial),
        crop_y(&initial)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reframe_filter_schedules_each_anchor() {
        let metadata = MediaMetadata {
            duration_ms: Some(2_000),
            has_video: true,
            has_audio: true,
            width: Some(640),
            height: Some(360),
            rotation_degrees: None,
            is_hdr: Some(false),
            timebase: None,
        };
        let anchors = [
            ReframeAnchor {
                output_start_ms: 0,
                center_x: 0.25,
                center_y: 0.5,
            },
            ReframeAnchor {
                output_start_ms: 1_000,
                center_x: 0.75,
                center_y: 0.5,
            },
        ];
        let filter = reframe_filter(&metadata, 360, 640, Some(&anchors)).expect("filter");
        assert!(filter.contains("sendcmd"));
        assert!(filter.contains("0.000 crop@reframe x"));
        assert!(filter.contains("1.000 crop@reframe x"));
        assert!(filter.contains("crop@reframe=360:640"));
    }
}
