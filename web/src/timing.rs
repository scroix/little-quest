use little_quest::input::Clock;

/// Browser timestamps are rounded and jitter by up to about this much. Fed straight to the
/// fixed-step `Clock`, that noise makes frames that should run 1 tick run 0 or 2.
const JITTER: f64 = 0.002;

/// Share of the timing error carried to the next frame. Forgiving the rest keeps a display a
/// little off 60 Hz, such as 59.94, at one tick per frame.
const CARRY: f64 = 0.99;

#[derive(Default)]
pub struct Pacer {
    owed: f64,
}

impl Pacer {
    /// The time to give the `Clock` for a frame that took `dt`. A frame near a common display
    /// interval gets exactly that interval. The difference carries over, so a display at any
    /// other rate still runs in real time.
    pub fn pace(&mut self, dt: f64) -> f64 {
        let owed = self.owed + dt;
        let given = [Clock::STEP * 2.0, Clock::STEP, Clock::STEP * 0.5]
            .into_iter()
            .find(|interval| (owed - interval).abs() <= JITTER)
            .unwrap_or(owed);
        self.owed = (owed - given) * CARRY;
        given
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jitter(k: usize) -> f64 {
        let h = (k as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15) >> 40;
        (h as f64 / (1u64 << 24) as f64 - 0.5) * 0.8
    }

    fn ticks_per_frame(hz: f64, stamp: impl Fn(f64, usize) -> f64, paced: bool) -> [u32; 4] {
        let mut pacer = Pacer::default();
        let mut clock = Clock::default();
        let mut hist = [0; 4];
        let mut last = stamp(0.0, 0);
        for k in 1..6000 {
            let t = stamp(k as f64 * 1000.0 / hz, k);
            let dt = (t - last) / 1000.0;
            last = t;
            let dt = if paced { pacer.pace(dt) } else { dt };
            hist[(clock.advance(dt) as usize).min(3)] += 1;
        }
        hist
    }

    type Stamp = (&'static str, fn(f64, usize) -> f64);
    const STAMPS: [Stamp; 4] = [
        ("exact", |t, _| t),
        ("1 ms rounded", |t, _| t.round()),
        ("offset origin", |t, _| 1000.37 + t),
        ("rounded with jitter", |t, k| (t + jitter(k)).round()),
    ];

    #[test]
    fn a_steady_display_never_makes_a_frame_with_the_wrong_tick_count() {
        for (name, stamp) in STAMPS {
            let thirty = ticks_per_frame(30.0, stamp, true);
            assert_eq!(thirty, [0, 0, 5999, 0], "30 Hz {name}");
            let sixty = ticks_per_frame(60.0, stamp, true);
            assert_eq!(sixty, [0, 5999, 0, 0], "60 Hz {name}");
            let [idle, one, two, more] = ticks_per_frame(120.0, stamp, true);
            assert_eq!([two, more], [0, 0], "120 Hz {name}");
            assert!(
                idle.abs_diff(one) <= 1,
                "120 Hz {name}: {idle} idle, {one} ticking"
            );
        }
    }

    #[test]
    fn every_refresh_rate_runs_in_real_time() {
        for hz in [
            30.0, 50.0, 60.0, 75.0, 90.0, 100.0, 120.0, 144.0, 165.0, 240.0,
        ] {
            for (name, stamp) in STAMPS {
                let [zero, one, two, more] = ticks_per_frame(hz, stamp, true);
                let ticks = one + two * 2 + more * 3;
                let seconds = (zero + one + two + more) as f64 / hz;
                let per_second = ticks as f64 / seconds;
                assert!(
                    (per_second - 60.0).abs() < 0.6,
                    "{hz} Hz {name}: {per_second} ticks per second"
                );
            }
        }
    }

    #[test]
    fn a_display_just_off_60_hz_still_runs_one_tick_per_frame() {
        assert_eq!(ticks_per_frame(59.94, STAMPS[0].1, true), [0, 5999, 0, 0]);
        assert_eq!(ticks_per_frame(60.02, STAMPS[1].1, true), [0, 5999, 0, 0]);
    }

    #[test]
    fn without_pacing_the_same_timestamps_do_make_them() {
        let [idle, _, two, _] = ticks_per_frame(60.0, STAMPS[1].1, false);
        assert!(idle > 1000 && two > 1000, "{idle} idle, {two} double");
    }

    #[test]
    fn a_stall_is_left_alone() {
        for dt in [0.1, 0.025, 0.0] {
            assert_eq!(Pacer::default().pace(dt), dt);
        }
    }
}
