use super::budget::BudgetTracker;
use super::format::format_row;

#[test]
fn format_row_escapes_delimiters() {
    let row = format_row(&["a|b", "x\\y", "line1\nline2"]);
    assert!(row.contains("a\\|b"));
    assert!(row.contains("x\\\\y"));
    assert!(row.contains("line1\\nline2"));
}

#[test]
fn budget_tracker_enforces_limit() {
    let mut tracker = BudgetTracker::new(10);
    assert!(tracker.add(4));
    assert_eq!(tracker.remaining(), 6);
    assert!(tracker.add(6));
    assert_eq!(tracker.remaining(), 0);
    assert!(!tracker.add(1));
}
