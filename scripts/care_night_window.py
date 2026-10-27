"""Alberta training-window policy, independent of host/scheduler tzdata.

Current MDT and permanent ABT from 2026-11-01 are both UTC-6. This
application policy covers readiness from 2026-10-27 onward, not historical
Alberta clock conversions. It neither schedules a wake nor changes the OS clock.
"""
from dataclasses import dataclass
import datetime as dt

UTC = dt.timezone.utc
ALBERTA_UTC6 = dt.timezone(dt.timedelta(hours=-6), 'Alberta UTC-6')
FIRST_SUPPORTED_DATE = dt.date(2026, 10, 27)
AUTHORITY = 'https://www.alberta.ca/albertas-new-time-system-abt'


@dataclass(frozen=True)
class NightWindow:
    local_date: dt.date
    start: dt.datetime
    hard_end: dt.datetime

    def cutoff(self, final_reserve_seconds):
        if type(final_reserve_seconds) is not int or not 180 <= final_reserve_seconds <= 3600:
            raise ValueError('reserve 180..3600 seconds for stopped recovery/publication')
        return self.hard_end - dt.timedelta(seconds=final_reserve_seconds)

    def admit(self, now, final_reserve_seconds):
        if now.tzinfo is None or now.utcoffset() is None:
            raise ValueError('nightly admission requires an aware current time')
        if not self.start <= now.astimezone(UTC) < self.cutoff(final_reserve_seconds):
            raise ValueError('outside the dated Alberta UTC-6 collection window; no early or expired launch')

    def manifest(self, final_reserve_seconds):
        return {'kind': 'alberta-training-window-utc6-v1', 'local_date': self.local_date.isoformat(),
                'utc_offset_seconds': -21600, 'local_start': '01:00', 'local_hard_end': '09:00',
                'start_utc': self.start.isoformat(),
                'collection_cutoff_utc': self.cutoff(final_reserve_seconds).isoformat(),
                'hard_end_utc': self.hard_end.isoformat(), 'authority': AUTHORITY,
                'scheduler_future_wake_verified': False}


def for_date(local_date):
    if type(local_date) is not dt.date or local_date < FIRST_SUPPORTED_DATE:
        raise ValueError('nightly policy requires a local date on or after 2026-10-27')
    return NightWindow(local_date,
        dt.datetime.combine(local_date, dt.time(1), ALBERTA_UTC6).astimezone(UTC),
        dt.datetime.combine(local_date, dt.time(9), ALBERTA_UTC6).astimezone(UTC))
