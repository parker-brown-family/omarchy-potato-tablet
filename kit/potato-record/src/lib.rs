//! The run record: the only contract between capture (`potato`) and judging (`judge/judge.py`).
//!
//! Capture writes every observer with `answer: None`. The judge fills answers in. An answer has
//! no default: an observer nobody judged reads as missing, never as pass and never as zero.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const FORMAT: u32 = 1;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Answer {
    Pass,
    Fail,
    NotObserved,
    NotAttempted,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    /// A person acts; the result fits or checks the calibration.
    Calibration,
    /// The agent acts, aiming with a named calibration; the result tests everything above it.
    Regression,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Input {
    /// Written into the touchscreen's input device: proves nothing about the glass.
    Synthetic,
    /// A finger on the glass.
    Physical,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Aim {
    /// How the points were chosen, e.g. "replay".
    pub source: String,
    /// Where the points came from, in words a person can check.
    pub provenance: String,
    /// Digitizer coordinates, in tap order.
    pub points: Vec<[i32; 2]>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Observation {
    /// None until the judge has looked.
    pub answer: Option<Answer>,
    /// What this observer should see if the task worked.
    pub expected: String,
    /// Evidence files, relative to the run directory. Empty means nothing was captured.
    pub evidence: Vec<String>,
    /// What the judge saw, in words.
    pub detail: Option<String>,
    /// Which check produced the answer.
    pub judged_by: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskRun {
    pub task: String,
    pub title: String,
    pub kind: TaskKind,
    pub input: Input,
    pub aim: Option<Aim>,
    /// The calibration the aim depends on, with its provenance.
    pub calibration: Option<String>,
    /// Whether the act ran: None if the run stopped before it.
    pub acted: Option<bool>,
    pub act_detail: Option<String>,
    /// Where the act's effect should appear, in framebuffer pixels (x, y, width, height): the
    /// window the kit opened, as Hyprland placed it. None when it could not be read.
    pub region: Option<[i32; 4]>,
    pub started: String,
    pub finished: Option<String>,
    pub observers: BTreeMap<String, Observation>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Record {
    pub format: u32,
    pub run: String,
    pub tablet: String,
    pub link: String,
    pub started: String,
    pub tasks: Vec<TaskRun>,
}

/// UTC timestamp, RFC 3339 to the second, from the system clock.
pub fn now_utc() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Howard Hinnant's civil-from-days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unjudged_observer_serialises_with_a_null_answer() {
        let o = Observation { answer: None, expected: "hello".into(), evidence: vec![], detail: None, judged_by: None };
        let json = serde_json::to_string(&o).unwrap();
        assert!(json.contains("\"answer\":null"), "{json}");
    }

    #[test]
    fn answers_use_the_names_the_judge_writes() {
        for (a, s) in [(Answer::Pass, "pass"), (Answer::Fail, "fail"), (Answer::NotObserved, "not_observed"), (Answer::NotAttempted, "not_attempted")] {
            assert_eq!(serde_json::to_string(&a).unwrap(), format!("\"{s}\""));
        }
    }

    #[test]
    fn timestamps_have_the_rfc3339_shape() {
        let t = now_utc();
        assert_eq!(t.len(), 20, "{t}");
        assert!(t.starts_with("20") && t.ends_with('Z'), "{t}");
    }
}
