//! `wl_output.transform` semantics shared by `screens` (scale/size math) and
//! `capture` (screencopy buffer orientation).
//!
//! A screencopy buffer holds the output's pixels in *buffer* orientation: the
//! unrotated physical mode (e.g. 1280x720 for a 720x1280 portrait output). The
//! compositor renders the logical (layout) image into that buffer through the
//! output transform, so recovering the logical image means applying the
//! transform `T` to the buffer. This is wlroots' `wlr_box_transform(box, T,
//! buffer_w, buffer_h)` (the inverse of the logical -> buffer mapping wlroots
//! uses when rendering, which is `wlr_box_transform(box, invert(T), ...)`), and
//! the same orientation grim produces.

/// The eight `wl_output.transform` values (wire values 0..=7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Transform {
    #[default]
    Normal,
    Rot90,
    Rot180,
    Rot270,
    Flipped,
    Flipped90,
    Flipped180,
    Flipped270,
}

impl Transform {
    /// All eight transforms, in wire order.
    #[cfg(test)]
    pub const ALL: [Self; 8] = [
        Self::Normal,
        Self::Rot90,
        Self::Rot180,
        Self::Rot270,
        Self::Flipped,
        Self::Flipped90,
        Self::Flipped180,
        Self::Flipped270,
    ];

    /// Map a `wl_output.transform` wire value; unknown values fall back to
    /// `Normal` (the protocol defines no others).
    pub fn from_raw(value: u32) -> Self {
        match value {
            1 => Self::Rot90,
            2 => Self::Rot180,
            3 => Self::Rot270,
            4 => Self::Flipped,
            5 => Self::Flipped90,
            6 => Self::Flipped180,
            7 => Self::Flipped270,
            _ => Self::Normal,
        }
    }

    /// Whether the transform swaps width and height (any 90/270 variant).
    pub fn swaps_axes(self) -> bool {
        matches!(
            self,
            Self::Rot90 | Self::Rot270 | Self::Flipped90 | Self::Flipped270
        )
    }

    /// `(w, h)` of a buffer-oriented size after the transform (logical
    /// orientation).
    pub fn transformed_size(self, w: u32, h: u32) -> (u32, u32) {
        if self.swaps_axes() { (h, w) } else { (w, h) }
    }

    /// Map a buffer pixel `(bx, by)` of a `bw x bh` buffer to its position in
    /// the logically oriented image (`transformed_size(bw, bh)`).
    ///
    /// Mirrors wlroots `wlr_box_transform` for a 1x1 box.
    pub fn buffer_to_logical(self, bx: u32, by: u32, bw: u32, bh: u32) -> (u32, u32) {
        match self {
            Self::Normal => (bx, by),
            Self::Rot90 => (bh - 1 - by, bx),
            Self::Rot180 => (bw - 1 - bx, bh - 1 - by),
            Self::Rot270 => (by, bw - 1 - bx),
            Self::Flipped => (bw - 1 - bx, by),
            Self::Flipped90 => (by, bx),
            Self::Flipped180 => (bx, bh - 1 - by),
            Self::Flipped270 => (bh - 1 - by, bw - 1 - bx),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_raw_round_trips_wire_order() {
        for (i, t) in Transform::ALL.iter().enumerate() {
            assert_eq!(Transform::from_raw(i as u32), *t);
        }
        assert_eq!(Transform::from_raw(99), Transform::Normal);
    }

    #[test]
    fn swaps_axes_only_for_quarter_turns() {
        let swapping: Vec<_> = Transform::ALL
            .iter()
            .filter(|t| t.swaps_axes())
            .copied()
            .collect();
        assert_eq!(
            swapping,
            vec![
                Transform::Rot90,
                Transform::Rot270,
                Transform::Flipped90,
                Transform::Flipped270
            ]
        );
        assert_eq!(Transform::Rot90.transformed_size(1280, 720), (720, 1280));
        assert_eq!(Transform::Flipped.transformed_size(1280, 720), (1280, 720));
    }

    #[test]
    fn mapping_is_a_bijection_for_every_transform() {
        let (bw, bh) = (3u32, 2u32);
        for t in Transform::ALL {
            let (lw, lh) = t.transformed_size(bw, bh);
            let mut seen = vec![false; (lw * lh) as usize];
            for by in 0..bh {
                for bx in 0..bw {
                    let (lx, ly) = t.buffer_to_logical(bx, by, bw, bh);
                    assert!(lx < lw && ly < lh, "{t:?} out of range");
                    let i = (ly * lw + lx) as usize;
                    assert!(!seen[i], "{t:?} maps two pixels to ({lx},{ly})");
                    seen[i] = true;
                }
            }
        }
    }
}
