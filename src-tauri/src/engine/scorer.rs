use crate::engine::types::{CheckResult, Severity};

pub fn score(checks: &[CheckResult]) -> u8 {
    let mut s: i32 = 100;
    for c in checks {
        s -= match c.severity {
            Severity::Ok | Severity::Info => 0,
            Severity::Warning => 10,
            Severity::Danger => 20,
            Severity::Critical => 35,
        };
    }
    s.clamp(0, 100) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::types::now_ts;
    use std::collections::HashMap;

    fn mk(sev: Severity) -> CheckResult {
        CheckResult {
            id: "x".into(),
            title: "x".into(),
            severity: sev,
            message: "m".into(),
            action_uri: None,
            evidence: HashMap::new(),
            checked_at: now_ts(),
        }
    }

    #[test]
    fn empty_checks_score_100() {
        assert_eq!(score(&[]), 100);
    }

    #[test]
    fn weights_match_table() {
        assert_eq!(score(&[mk(Severity::Warning)]), 90);
        assert_eq!(score(&[mk(Severity::Danger)]), 80);
        assert_eq!(score(&[mk(Severity::Critical)]), 65);
    }

    #[test]
    fn score_clamped_at_zero() {
        let many = (0..10).map(|_| mk(Severity::Critical)).collect::<Vec<_>>();
        assert_eq!(score(&many), 0);
    }
}
