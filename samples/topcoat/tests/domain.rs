use tatami_log::domain::{
    ApplicationResult, RecallRating, Uniform, parse_iso_date, schedule_review,
};
use time::macros::date;

#[test]
fn review_schedule_prioritizes_recall_and_rewards_live_application() {
    let reviewed_on = date!(2026 - 08 - 12);

    let cases = [
        (
            RecallRating::Forgot,
            ApplicationResult::NotTried,
            date!(2026 - 08 - 13),
        ),
        (
            RecallRating::Fuzzy,
            ApplicationResult::Attempted,
            date!(2026 - 08 - 15),
        ),
        (
            RecallRating::Clear,
            ApplicationResult::NotTried,
            date!(2026 - 08 - 19),
        ),
        (
            RecallRating::Clear,
            ApplicationResult::Worked,
            date!(2026 - 08 - 26),
        ),
    ];

    for (recall, application, expected) in cases {
        assert_eq!(
            schedule_review(reviewed_on, recall, application).next_review_on,
            expected
        );
    }
}

#[test]
fn persisted_choices_and_dates_reject_unknown_values() {
    assert_eq!("gi".parse::<Uniform>(), Ok(Uniform::Gi));
    assert_eq!("no_gi".parse::<Uniform>(), Ok(Uniform::NoGi));
    assert!("nogi".parse::<Uniform>().is_err());

    assert_eq!("fuzzy".parse::<RecallRating>(), Ok(RecallRating::Fuzzy));
    assert!("almost".parse::<RecallRating>().is_err());

    assert_eq!(
        "attempted".parse::<ApplicationResult>(),
        Ok(ApplicationResult::Attempted)
    );
    assert!("won".parse::<ApplicationResult>().is_err());

    assert_eq!(parse_iso_date("2026-08-12"), Ok(date!(2026 - 08 - 12)));
    assert!(parse_iso_date("2026-02-30").is_err());
}
