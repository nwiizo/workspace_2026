//! Geometry and safety policy for Maskcam.

use opencv::core::{Rect, Size};

/// The face rectangle reported by the detector.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FaceBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub angle_degrees: f64,
}

impl FaceBox {
    #[must_use]
    pub fn area(self) -> f32 {
        self.width * self.height
    }
}

/// Exponential smoothing for a continuously detected face.
///
/// Call [`Self::reset`] whenever the privacy policy hides the frame. This avoids
/// carrying a stale position across a detection failure.
#[derive(Clone, Copy, Debug)]
pub struct FaceSmoother {
    previous: Option<FaceBox>,
    alpha: f32,
}

impl FaceSmoother {
    #[must_use]
    pub fn new(alpha: f32) -> Self {
        Self {
            previous: None,
            alpha: alpha.clamp(0.0, 1.0),
        }
    }

    pub fn reset(&mut self) {
        self.previous = None;
    }

    pub fn update(&mut self, current: FaceBox) -> FaceBox {
        let Some(previous) = self.previous else {
            self.previous = Some(current);
            return current;
        };

        let alpha = self.alpha;
        let smooth = |old: f32, new: f32| alpha.mul_add(new, (1.0 - alpha) * old);
        let smoothed = FaceBox {
            x: smooth(previous.x, current.x),
            y: smooth(previous.y, current.y),
            width: smooth(previous.width, current.width),
            height: smooth(previous.height, current.height),
            angle_degrees: f64::from(alpha).mul_add(
                current.angle_degrees,
                (1.0 - f64::from(alpha)) * previous.angle_degrees,
            ),
        };
        self.previous = Some(smoothed);
        smoothed
    }
}

/// A clipped overlay target and the matching crop within the un-clipped overlay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OverlayPlacement {
    pub destination: Rect,
    pub source: Rect,
    pub canvas_size: Size,
}

/// Returns an enlarged square around a face, clipped to the video frame.
#[must_use]
pub fn overlay_placement(face: FaceBox, scale: f32, frame_size: Size) -> Option<OverlayPlacement> {
    if scale <= 0.0
        || face.width <= 0.0
        || face.height <= 0.0
        || frame_size.width <= 0
        || frame_size.height <= 0
    {
        return None;
    }

    let side = rounded_i32(f64::from(face.width.max(face.height) * scale))?.max(1);
    let center_x = f64::from(face.x + face.width / 2.0);
    let center_y = f64::from(face.y + face.height / 2.0);
    let target_x = rounded_i32(center_x - f64::from(side) / 2.0)?;
    let target_y = rounded_i32(center_y - f64::from(side) / 2.0)?;

    let left = target_x.clamp(0, frame_size.width);
    let top = target_y.clamp(0, frame_size.height);
    let right = (target_x + side).clamp(0, frame_size.width);
    let bottom = (target_y + side).clamp(0, frame_size.height);

    if left >= right || top >= bottom {
        return None;
    }

    Some(OverlayPlacement {
        destination: Rect::new(left, top, right - left, bottom - top),
        source: Rect::new(left - target_x, top - target_y, right - left, bottom - top),
        canvas_size: Size::new(side, side),
    })
}

fn rounded_i32(value: f64) -> Option<i32> {
    let rounded = value.round();
    if !rounded.is_finite() || rounded < f64::from(i32::MIN) || rounded > f64::from(i32::MAX) {
        return None;
    }

    // Bounds and finiteness were checked immediately above.
    #[allow(clippy::cast_possible_truncation)]
    Some(rounded as i32)
}

/// Privacy-first output decision. Anything except exactly one face is hidden.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visibility {
    ShowMaskedFace,
    HideEntireFrame,
}

#[must_use]
pub const fn visibility_for_face_count(face_count: usize) -> Visibility {
    if face_count == 1 {
        Visibility::ShowMaskedFace
    } else {
        Visibility::HideEntireFrame
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn face() -> FaceBox {
        FaceBox {
            x: 40.0,
            y: 20.0,
            width: 20.0,
            height: 30.0,
            angle_degrees: 0.0,
        }
    }

    #[test]
    fn expands_face_to_square() {
        let placement = overlay_placement(face(), 2.0, Size::new(100, 100)).unwrap();
        assert_eq!(placement.destination, Rect::new(20, 5, 60, 60));
        assert_eq!(placement.source, Rect::new(0, 0, 60, 60));
        assert_eq!(placement.canvas_size, Size::new(60, 60));
    }

    #[test]
    fn clips_at_frame_edge_and_offsets_source() {
        let placement = overlay_placement(
            FaceBox {
                x: 0.0,
                y: 0.0,
                ..face()
            },
            2.0,
            Size::new(100, 100),
        )
        .unwrap();

        assert_eq!(placement.destination, Rect::new(0, 0, 40, 45));
        assert_eq!(placement.source, Rect::new(20, 15, 40, 45));
    }

    #[test]
    fn rejects_invalid_geometry() {
        assert_eq!(overlay_placement(face(), 0.0, Size::new(100, 100)), None);
        assert_eq!(overlay_placement(face(), 2.0, Size::new(0, 100)), None);
    }

    #[test]
    fn fails_closed_unless_exactly_one_face_is_detected() {
        assert_eq!(visibility_for_face_count(0), Visibility::HideEntireFrame);
        assert_eq!(visibility_for_face_count(1), Visibility::ShowMaskedFace);
        assert_eq!(visibility_for_face_count(2), Visibility::HideEntireFrame);
    }

    #[test]
    fn smooths_consecutive_detections_and_resets_after_failure() {
        let mut smoother = FaceSmoother::new(0.5);
        assert_eq!(smoother.update(face()), face());

        let moved = FaceBox {
            x: 60.0,
            y: 40.0,
            width: 40.0,
            height: 50.0,
            angle_degrees: 20.0,
        };
        assert_eq!(
            smoother.update(moved),
            FaceBox {
                x: 50.0,
                y: 30.0,
                width: 30.0,
                height: 40.0,
                angle_degrees: 10.0,
            }
        );

        smoother.reset();
        assert_eq!(smoother.update(moved), moved);
    }
}
