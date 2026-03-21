use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

pub struct SpawnTracker {
    spawns: Vec<SpawnRecord>,
    max_spawns_per_run: usize,
    max_spawns_same_goal: usize,
}

struct SpawnRecord {
    goal_hash: u64,
    goal_preview: String,
    #[allow(dead_code)]
    step: i64,
    status: Option<String>,
}

pub enum DelegationWarning {
    TotalLimitReached { count: usize, limit: usize },
    SameGoalRepeated { goal_preview: String, count: usize, limit: usize },
}

fn hash_goal(goal: &str) -> u64 {
    let normalized = goal.trim().to_lowercase();
    let mut hasher = DefaultHasher::new();
    normalized.hash(&mut hasher);
    hasher.finish()
}

impl SpawnTracker {
    pub fn new(max_spawns_per_run: usize, max_spawns_same_goal: usize) -> Self {
        Self {
            spawns: Vec::new(),
            max_spawns_per_run,
            max_spawns_same_goal,
        }
    }

    pub fn record_spawn(&mut self, goal: &str, step: i64) {
        self.spawns.push(SpawnRecord {
            goal_hash: hash_goal(goal),
            goal_preview: uni_common::safe_truncate(goal, 100).to_string(),
            step,
            status: None,
        });
    }

    pub fn record_result(&mut self, goal: &str, status: &str) {
        let hash = hash_goal(goal);
        if let Some(record) = self.spawns.iter_mut().rev()
            .find(|r| r.goal_hash == hash && r.status.is_none())
        {
            record.status = Some(status.to_string());
        }
    }

    pub fn check_limits(&self) -> Option<DelegationWarning> {
        if self.spawns.len() >= self.max_spawns_per_run {
            return Some(DelegationWarning::TotalLimitReached {
                count: self.spawns.len(),
                limit: self.max_spawns_per_run,
            });
        }

        let mut goal_counts: HashMap<u64, usize> = HashMap::new();
        for spawn in &self.spawns {
            *goal_counts.entry(spawn.goal_hash).or_default() += 1;
        }
        for (hash, count) in &goal_counts {
            if *count >= self.max_spawns_same_goal {
                let preview = self.spawns.iter()
                    .find(|s| s.goal_hash == *hash)
                    .map(|s| s.goal_preview.clone())
                    .unwrap_or_default();
                return Some(DelegationWarning::SameGoalRepeated {
                    goal_preview: preview,
                    count: *count,
                    limit: self.max_spawns_same_goal,
                });
            }
        }

        None
    }

    pub fn format_warning(warning: &DelegationWarning) -> String {
        match warning {
            DelegationWarning::TotalLimitReached { count, limit } => {
                format!(
                    "Cannot spawn more sub-agents: reached limit of {} (max {}). \
                     Work with what you have and provide your best answer.",
                    count, limit
                )
            }
            DelegationWarning::SameGoalRepeated { goal_preview, count, limit } => {
                format!(
                    "Cannot spawn sub-agent for '{}': this goal has been attempted {} times (max {}). \
                     The approach may not be working — try a different strategy or provide your best answer.",
                    goal_preview, count, limit
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_under_limit() {
        let mut tracker = SpawnTracker::new(10, 2);
        tracker.record_spawn("search for news", 1);
        assert!(tracker.check_limits().is_none());
    }

    #[test]
    fn test_total_limit() {
        let mut tracker = SpawnTracker::new(3, 2);
        tracker.record_spawn("task 1", 1);
        tracker.record_spawn("task 2", 2);
        tracker.record_spawn("task 3", 3);
        assert!(matches!(
            tracker.check_limits(),
            Some(DelegationWarning::TotalLimitReached { .. })
        ));
    }

    #[test]
    fn test_same_goal_limit() {
        let mut tracker = SpawnTracker::new(10, 2);
        tracker.record_spawn("search for X", 1);
        tracker.record_spawn("search for X", 2);
        assert!(matches!(
            tracker.check_limits(),
            Some(DelegationWarning::SameGoalRepeated { .. })
        ));
    }

    #[test]
    fn test_different_goals_ok() {
        let mut tracker = SpawnTracker::new(10, 2);
        tracker.record_spawn("search for X", 1);
        tracker.record_spawn("search for Y", 2);
        tracker.record_spawn("analyze Z", 3);
        assert!(tracker.check_limits().is_none());
    }

    #[test]
    fn test_record_result() {
        let mut tracker = SpawnTracker::new(10, 2);
        tracker.record_spawn("search for X", 1);
        tracker.record_result("search for X", "completed");
        assert_eq!(tracker.spawns[0].status, Some("completed".to_string()));
    }

    #[test]
    fn test_format_warning_total() {
        let msg = SpawnTracker::format_warning(&DelegationWarning::TotalLimitReached {
            count: 10,
            limit: 10,
        });
        assert!(msg.contains("reached limit of 10"));
    }

    #[test]
    fn test_format_warning_same_goal() {
        let msg = SpawnTracker::format_warning(&DelegationWarning::SameGoalRepeated {
            goal_preview: "search for X".to_string(),
            count: 2,
            limit: 2,
        });
        assert!(msg.contains("search for X"));
        assert!(msg.contains("attempted 2 times"));
    }

    #[test]
    fn test_safe_truncate_utf8() {
        let russian = "Поиск информации о погоде в Москве и Санкт-Петербурге на следующую неделю";
        let truncated = uni_common::safe_truncate(russian, 20).to_string();
        assert!(truncated.len() <= 20);
        // Verify it's valid UTF-8 (won't panic)
        let _ = truncated.chars().count();
    }

    #[test]
    fn test_case_insensitive_goal_matching() {
        let mut tracker = SpawnTracker::new(10, 2);
        tracker.record_spawn("Search for X", 1);
        tracker.record_spawn("search for x", 2);
        assert!(matches!(
            tracker.check_limits(),
            Some(DelegationWarning::SameGoalRepeated { .. })
        ));
    }
}
