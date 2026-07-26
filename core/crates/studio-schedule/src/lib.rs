//! When work happens.
//!
//! Schedules are held in the User's terms — a time of day, chosen weekdays, an
//! interval — and that form is authoritative (Requirement 9.1). A cron expression
//! is derived from it for execution and is never displayed, so the interface never
//! has to show one.
//!
//! The engine owns its own persistence rather than relying on ADK-Rust's cron
//! stores, which are in-memory and would lose every schedule on restart
//! (Requirement 18.5).
//!
//! Missed executions are the interesting part. A laptop is asleep at 7am more often
//! than a server is, so each Job declares what should happen when its time passed
//! unobserved: run once on waking, or skip to the next occurrence
//! (Requirement 9.3).

use std::fmt;

/// Days of the week, Monday first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Weekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

impl Weekday {
    /// From a day index where 0 is Monday.
    pub fn from_index(i: u32) -> Self {
        match i % 7 {
            0 => Self::Mon,
            1 => Self::Tue,
            2 => Self::Wed,
            3 => Self::Thu,
            4 => Self::Fri,
            5 => Self::Sat,
            _ => Self::Sun,
        }
    }

    pub fn index(self) -> u32 {
        self as u32
    }

    pub const WEEKDAYS: [Weekday; 5] = [Self::Mon, Self::Tue, Self::Wed, Self::Thu, Self::Fri];

    pub fn name(self) -> &'static str {
        match self {
            Self::Mon => "Monday",
            Self::Tue => "Tuesday",
            Self::Wed => "Wednesday",
            Self::Thu => "Thursday",
            Self::Fri => "Friday",
            Self::Sat => "Saturday",
            Self::Sun => "Sunday",
        }
    }
}

/// A schedule as the User set it. This form is authoritative.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Schedule {
    /// Every day at a time.
    Daily { hour: u32, minute: u32 },
    /// Only on chosen days, at a time.
    OnDays {
        days: Vec<Weekday>,
        hour: u32,
        minute: u32,
    },
    /// Every so often.
    Every { seconds: u64 },
    /// Only when the User asks.
    Manual,
}

impl Schedule {
    /// Weekdays at a time — the newsletter's default.
    pub fn weekdays_at(hour: u32, minute: u32) -> Self {
        Self::OnDays {
            days: Weekday::WEEKDAYS.to_vec(),
            hour,
            minute,
        }
    }

    /// How the interface says it. Never a cron expression.
    pub fn human(&self) -> String {
        match self {
            Self::Daily { hour, minute } => format!("Every day at {}", clock(*hour, *minute)),
            Self::OnDays { days, hour, minute } => {
                let mut sorted = days.clone();
                sorted.sort();
                sorted.dedup();
                let when = clock(*hour, *minute);
                if sorted == Weekday::WEEKDAYS.to_vec() {
                    format!("Every weekday at {when}")
                } else if sorted.len() == 1 {
                    format!("Every {} at {when}", sorted[0].name())
                } else {
                    let names: Vec<&str> = sorted.iter().map(|d| &d.name()[..3]).collect();
                    format!("{} at {when}", names.join(", "))
                }
            }
            Self::Every { seconds } => format!("Every {}", every(*seconds)),
            Self::Manual => "Only when you ask".to_string(),
        }
    }

    /// The derived form, for execution only. Never displayed.
    pub fn cron(&self) -> Option<String> {
        match self {
            Self::Daily { hour, minute } => Some(format!("{minute} {hour} * * *")),
            Self::OnDays { days, hour, minute } => {
                let mut sorted = days.clone();
                sorted.sort();
                sorted.dedup();
                let dow: Vec<String> = sorted
                    .iter()
                    .map(|d| ((d.index() + 1) % 7).to_string())
                    .collect();
                Some(format!("{minute} {hour} * * {}", dow.join(",")))
            }
            // An interval is not a calendar expression; it is computed directly.
            Self::Every { .. } | Self::Manual => None,
        }
    }

    /// The first occurrence strictly after `after`, in seconds since the epoch.
    ///
    /// Times are computed in the User's own day, so `day_start` is supplied by the
    /// caller from their time zone rather than assumed to be UTC.
    pub fn next_after(&self, after: i64, day_start: i64, weekday_of_day: Weekday) -> Option<i64> {
        const DAY: i64 = 86_400;
        match self {
            Self::Manual => None,
            Self::Every { seconds } => Some(after + *seconds as i64),
            Self::Daily { hour, minute } => {
                let today = day_start + (*hour as i64) * 3600 + (*minute as i64) * 60;
                Some(if today > after { today } else { today + DAY })
            }
            Self::OnDays { days, hour, minute } => {
                if days.is_empty() {
                    return None;
                }
                let offset = (*hour as i64) * 3600 + (*minute as i64) * 60;
                for ahead in 0..=7 {
                    let candidate_day = day_start + ahead * DAY;
                    let weekday = Weekday::from_index(weekday_of_day.index() + ahead as u32);
                    if days.contains(&weekday) {
                        let at = candidate_day + offset;
                        if at > after {
                            return Some(at);
                        }
                    }
                }
                None
            }
        }
    }
}

fn clock(hour: u32, minute: u32) -> String {
    let (h12, suffix) = match hour {
        0 => (12, "am"),
        1..=11 => (hour, "am"),
        12 => (12, "pm"),
        _ => (hour - 12, "pm"),
    };
    format!("{h12}:{minute:02} {suffix}")
}

fn every(seconds: u64) -> String {
    match seconds {
        s if s % 86_400 == 0 => {
            let d = s / 86_400;
            if d == 1 {
                "day".into()
            } else {
                format!("{d} days")
            }
        }
        s if s % 3_600 == 0 => {
            let h = s / 3_600;
            if h == 1 {
                "hour".into()
            } else {
                format!("{h} hours")
            }
        }
        s if s % 60 == 0 => {
            let m = s / 60;
            if m == 1 {
                "minute".into()
            } else {
                format!("{m} minutes")
            }
        }
        s => format!("{s} seconds"),
    }
}

/// What to do about a time that passed while nothing was watching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissedRunPolicy {
    /// Do it once now. Right for a digest: late is better than never.
    RunOnceOnWake,
    /// Let it go. Right for a monitor: a stale check is worth nothing.
    SkipToNext,
}

impl MissedRunPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RunOnceOnWake => "run_once_on_wake",
            Self::SkipToNext => "skip_to_next",
        }
    }
}

/// What the engine should do at this moment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Nothing due.
    Wait { until: i64 },
    /// Due now.
    RunNow,
    /// One or more times passed unobserved; do it once and move on.
    RunOnceForMissed { missed: u32, until: i64 },
    /// Times passed unobserved and are being let go.
    SkipMissed { missed: u32, until: i64 },
    /// Never runs by itself.
    NeverAutomatic,
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wait { .. } => write!(f, "waiting"),
            Self::RunNow => write!(f, "due now"),
            Self::RunOnceForMissed { missed, .. } => write!(f, "{missed} missed, running once"),
            Self::SkipMissed { missed, .. } => write!(f, "{missed} missed, skipping"),
            Self::NeverAutomatic => write!(f, "only when asked"),
        }
    }
}

/// Decide what to do, given when the Job last ran and what time it is now.
///
/// `day_start` and `weekday` describe *now* in the User's own time zone. When
/// counting occurrences that passed unobserved, the day boundary is advanced
/// alongside the cursor — an earlier version held it fixed at today, which made
/// every occurrence between the last run and this morning invisible.
pub fn decide(
    schedule: &Schedule,
    policy: MissedRunPolicy,
    last_run: Option<i64>,
    now: i64,
    day_start: i64,
    weekday: Weekday,
) -> Action {
    if matches!(schedule, Schedule::Manual) {
        return Action::NeverAutomatic;
    }

    let mut cursor = last_run.unwrap_or(now);
    let mut missed = 0u32;
    let mut guard = 0;
    loop {
        let (cursor_day, cursor_weekday) = align_day(day_start, weekday, cursor);
        let Some(next) = schedule.next_after(cursor, cursor_day, cursor_weekday) else {
            break;
        };
        if next > now {
            break;
        }
        missed += 1;
        cursor = next;
        guard += 1;
        if guard > 4_000 {
            break;
        }
    }

    let until = schedule
        .next_after(now.max(cursor), day_start, weekday)
        .unwrap_or(now);

    match missed {
        0 => Action::Wait { until },
        1 if last_run.is_some() => Action::RunNow,
        _ => match policy {
            MissedRunPolicy::RunOnceOnWake => Action::RunOnceForMissed { missed, until },
            MissedRunPolicy::SkipToNext => Action::SkipMissed { missed, until },
        },
    }
}

/// The day boundary and weekday containing `t`, given the boundary and weekday of
/// the current day. Days are whole and 86 400 seconds long in the User's own zone.
fn align_day(day_start_now: i64, weekday_now: Weekday, t: i64) -> (i64, Weekday) {
    const DAY: i64 = 86_400;
    let days = (t - day_start_now).div_euclid(DAY);
    let boundary = day_start_now + days * DAY;
    // Rust's `%` can be negative, so shift into range before taking it.
    let index = (weekday_now.index() as i64 + days).rem_euclid(7) as u32;
    (boundary, Weekday::from_index(index))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: i64 = 86_400;
    /// A Monday at 00:00 for the User.
    const MON: i64 = 0;

    #[test]
    fn a_schedule_reads_in_the_users_words() {
        assert_eq!(
            Schedule::weekdays_at(7, 0).human(),
            "Every weekday at 7:00 am"
        );
        assert_eq!(
            Schedule::Daily {
                hour: 19,
                minute: 30
            }
            .human(),
            "Every day at 7:30 pm"
        );
        assert_eq!(
            Schedule::Every { seconds: 2 * 3600 }.human(),
            "Every 2 hours"
        );
        assert_eq!(Schedule::Manual.human(), "Only when you ask");
        assert_eq!(
            Schedule::OnDays {
                days: vec![Weekday::Sun],
                hour: 8,
                minute: 0
            }
            .human(),
            "Every Sunday at 8:00 am"
        );
        assert_eq!(
            Schedule::Daily { hour: 0, minute: 5 }.human(),
            "Every day at 12:05 am"
        );
    }

    /// The derived cron form exists for execution and is never the User's form.
    #[test]
    fn the_cron_form_is_derived_and_never_shown() {
        let s = Schedule::weekdays_at(7, 0);
        assert_eq!(s.cron().as_deref(), Some("0 7 * * 1,2,3,4,5"));
        assert!(
            !s.human().contains('*'),
            "the User must never see a cron field"
        );
        assert_eq!(Schedule::Every { seconds: 60 }.cron(), None);
        assert_eq!(Schedule::Manual.cron(), None);
    }

    #[test]
    fn the_next_weekday_occurrence_skips_the_weekend() {
        let s = Schedule::weekdays_at(7, 0);
        // Friday 08:00, so the next is Monday 07:00.
        let friday = MON + 4 * DAY;
        let next = s
            .next_after(friday + 8 * 3600, friday, Weekday::Fri)
            .unwrap();
        assert_eq!(next, friday + 3 * DAY + 7 * 3600, "should jump to Monday");
    }

    #[test]
    fn a_daily_schedule_rolls_to_tomorrow_once_the_time_has_passed() {
        let s = Schedule::Daily { hour: 7, minute: 0 };
        let before = s.next_after(MON + 6 * 3600, MON, Weekday::Mon).unwrap();
        assert_eq!(before, MON + 7 * 3600, "still today");
        let after = s.next_after(MON + 8 * 3600, MON, Weekday::Mon).unwrap();
        assert_eq!(after, MON + DAY + 7 * 3600, "tomorrow");
    }

    #[test]
    fn nothing_is_due_when_the_time_has_not_come() {
        let s = Schedule::Daily { hour: 7, minute: 0 };
        let action = decide(
            &s,
            MissedRunPolicy::RunOnceOnWake,
            Some(MON - DAY + 7 * 3600),
            MON + 6 * 3600,
            MON,
            Weekday::Mon,
        );
        assert!(matches!(action, Action::Wait { .. }), "got {action:?}");
        if let Action::Wait { until } = action {
            assert_eq!(until, MON + 7 * 3600);
        }
    }

    #[test]
    fn one_due_occurrence_is_simply_due() {
        let s = Schedule::Daily { hour: 7, minute: 0 };
        let action = decide(
            &s,
            MissedRunPolicy::SkipToNext,
            Some(MON - DAY + 7 * 3600),
            MON + 8 * 3600,
            MON,
            Weekday::Mon,
        );
        assert_eq!(action, Action::RunNow);
    }

    /// Requirement 9.3: a laptop asleep over several occurrences.
    #[test]
    fn a_digest_runs_once_on_waking_and_a_monitor_lets_them_go() {
        let s = Schedule::Daily { hour: 7, minute: 0 };
        let last = MON + 7 * 3600; // ran Monday
        let now = MON + 4 * DAY + 9 * 3600; // woke Friday morning
        // Tue, Wed, Thu, Fri passed = 4 missed.

        let digest = decide(
            &s,
            MissedRunPolicy::RunOnceOnWake,
            Some(last),
            now,
            MON + 4 * DAY,
            Weekday::Fri,
        );
        match digest {
            Action::RunOnceForMissed { missed, .. } => assert_eq!(missed, 4),
            other => panic!("a digest should run once on waking, got {other:?}"),
        }

        let monitor = decide(
            &s,
            MissedRunPolicy::SkipToNext,
            Some(last),
            now,
            MON + 4 * DAY,
            Weekday::Fri,
        );
        match monitor {
            Action::SkipMissed { missed, until } => {
                assert_eq!(missed, 4);
                assert!(until > now, "the next time must still be scheduled");
            }
            other => panic!("a monitor should let them go, got {other:?}"),
        }
    }

    #[test]
    fn an_interval_schedule_counts_missed_ticks() {
        let s = Schedule::Every { seconds: 2 * 3600 };
        let last = MON;
        let now = MON + 9 * 3600; // four and a half intervals later
        let action = decide(
            &s,
            MissedRunPolicy::SkipToNext,
            Some(last),
            now,
            MON,
            Weekday::Mon,
        );
        match action {
            Action::SkipMissed { missed, .. } => assert_eq!(missed, 4),
            other => panic!("expected missed ticks, got {other:?}"),
        }
    }

    #[test]
    fn a_manual_schedule_never_runs_by_itself() {
        assert_eq!(
            decide(
                &Schedule::Manual,
                MissedRunPolicy::RunOnceOnWake,
                None,
                100,
                MON,
                Weekday::Mon
            ),
            Action::NeverAutomatic
        );
    }

    #[test]
    fn a_job_that_has_never_run_waits_for_its_first_time() {
        let s = Schedule::weekdays_at(7, 0);
        let action = decide(
            &s,
            MissedRunPolicy::RunOnceOnWake,
            None,
            MON + 6 * 3600,
            MON,
            Weekday::Mon,
        );
        assert!(
            matches!(action, Action::Wait { .. }),
            "a new Job should not think it missed anything: {action:?}"
        );
    }

    /// A long sleep must not spin the counter forever.
    #[test]
    fn a_very_long_absence_is_bounded() {
        let s = Schedule::Every { seconds: 60 };
        let action = decide(
            &s,
            MissedRunPolicy::SkipToNext,
            Some(0),
            365 * DAY,
            0,
            Weekday::Mon,
        );
        match action {
            Action::SkipMissed { missed, .. } => {
                assert!(
                    missed > 0 && missed <= 4_001,
                    "counter should be bounded, got {missed}"
                )
            }
            other => panic!("expected skipping, got {other:?}"),
        }
    }
}
