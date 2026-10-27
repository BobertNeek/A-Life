"""Device-free Alberta nightly policy tests; no clock/settings mutation."""
import datetime as dt
import unittest
from unittest.mock import patch

import care_night_window as policy


class AlbertaNightWindowTests(unittest.TestCase):
    def test_current_mdt_transition_day_and_permanent_abt_keep_eight_hours(self):
        for text in ('2026-10-28', '2026-10-31', '2026-11-01', '2026-11-02',
                     '2027-01-15', '2027-03-14', '2027-11-07'):
            with self.subTest(date=text):
                date = dt.date.fromisoformat(text)
                window = policy.for_date(date)
                self.assertEqual(window.start, dt.datetime.combine(date, dt.time(7), policy.UTC))
                self.assertEqual(window.hard_end, dt.datetime.combine(date, dt.time(15), policy.UTC))
                self.assertEqual(window.hard_end - window.start, dt.timedelta(hours=8))
                self.assertEqual(window.cutoff(180).time(), dt.time(14, 57))

    def test_policy_does_not_depend_on_zoneinfo_rules(self):
        with patch('zoneinfo.ZoneInfo', side_effect=AssertionError('stale/unavailable tzdb must not be consulted')):
            window = policy.for_date(dt.date(2026, 11, 2))
            self.assertEqual((window.start.hour, window.hard_end.hour), (7, 15))

    def test_exact_open_cutoff_and_hard_end_boundaries(self):
        window = policy.for_date(dt.date(2026, 11, 1))
        for now in (window.start - dt.timedelta(microseconds=1), window.cutoff(180),
                    window.hard_end, window.hard_end + dt.timedelta(hours=1)):
            with self.subTest(now=now), self.assertRaisesRegex(ValueError, 'dated Alberta'):
                window.admit(now, 180)
        window.admit(window.start, 180)
        window.admit(window.cutoff(180) - dt.timedelta(microseconds=1), 180)

    def test_late_scheduler_wake_gets_remaining_window_without_end_extension(self):
        window = policy.for_date(dt.date(2026, 11, 2))
        window.admit(dt.datetime(2026, 11, 2, 8, tzinfo=policy.UTC), 180)
        self.assertEqual(window.hard_end.hour, 15)
        self.assertFalse(window.manifest(180)['scheduler_future_wake_verified'])

    def test_old_date_naive_time_and_invalid_save_reserve_refuse(self):
        with self.assertRaises(ValueError):
            policy.for_date(dt.date(2026, 10, 26))
        window = policy.for_date(dt.date(2026, 10, 27))
        with self.assertRaisesRegex(ValueError, 'dated Alberta'):
            window.admit(dt.datetime(2026, 10, 28, 7, tzinfo=policy.UTC), 180)
        with self.assertRaisesRegex(ValueError, 'aware'):
            window.admit(dt.datetime(2026, 10, 27, 7), 180)
        for reserve in (30, 0, -1, 3601, True):
            with self.subTest(reserve=reserve), self.assertRaises(ValueError):
                window.cutoff(reserve)


if __name__ == '__main__':
    unittest.main()
