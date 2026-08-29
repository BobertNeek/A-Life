pub(crate) const PHASE31_SLOW_FRAME_THRESHOLD_NS: u64 = 25_000_000;
pub(crate) const PHASE31_SLOW_FRAME_LIMIT: usize = 100;

pub(crate) trait RankedSlowFrame {
    fn frame_duration_ns(&self) -> u64;
    fn frame_index(&self) -> u64;
}

pub(crate) fn retain_ranked_slow_frame<T: RankedSlowFrame>(slow_frames: &mut Vec<T>, sample: T) {
    if sample.frame_duration_ns() <= PHASE31_SLOW_FRAME_THRESHOLD_NS {
        return;
    }
    slow_frames.push(sample);
    slow_frames.sort_unstable_by(|left, right| {
        right
            .frame_duration_ns()
            .cmp(&left.frame_duration_ns())
            .then_with(|| left.frame_index().cmp(&right.frame_index()))
    });
    slow_frames.truncate(PHASE31_SLOW_FRAME_LIMIT);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy)]
    struct Sample {
        frame_index: u64,
        frame_duration_ns: u64,
    }

    impl RankedSlowFrame for Sample {
        fn frame_duration_ns(&self) -> u64 {
            self.frame_duration_ns
        }

        fn frame_index(&self) -> u64 {
            self.frame_index
        }
    }

    #[test]
    fn filters_threshold_and_caps_worst_100() {
        let mut slow_frames = Vec::new();
        retain_ranked_slow_frame(
            &mut slow_frames,
            Sample {
                frame_index: 0,
                frame_duration_ns: 25_000_000,
            },
        );
        for frame_ms in 26_u64..=126 {
            retain_ranked_slow_frame(
                &mut slow_frames,
                Sample {
                    frame_index: frame_ms,
                    frame_duration_ns: frame_ms * 1_000_000,
                },
            );
        }

        assert_eq!(slow_frames.len(), 100);
        assert_eq!(slow_frames[0].frame_duration_ns, 126_000_000);
        assert_eq!(slow_frames[99].frame_duration_ns, 27_000_000);
        assert!(slow_frames
            .windows(2)
            .all(|pair| pair[0].frame_duration_ns >= pair[1].frame_duration_ns));
    }
}
