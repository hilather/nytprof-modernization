//! Stream-oriented aggregation into [`nytprof_model::ProfileModel`].
//!
//! This crate is a thin façade: core state and definitions live in
//! `nytprof-model` (aggregate-comparison contract A1–A9). Callers that want a
//! free-function streaming API can depend on this package.

pub use nytprof_model::{
    f64_close, is_workload_sub, CallEdgeTotal, LineTotal, LineTotals, ModelError, ProfileModel,
    Result, SubDef, SubReturnTotals, SubTotal,
};

use nytprof_types::Event;

/// Fold one logical event into `model` (exact A1–A9 definitions).
#[inline]
pub fn accumulate(model: &mut ProfileModel, event: &Event) -> Result<()> {
    model.accumulate(event)
}

/// Aggregate an entire event slice into a new model.
pub fn aggregate_events(events: &[Event]) -> Result<ProfileModel> {
    ProfileModel::from_events(events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nytprof_types::tags;
    use serde_json::Value;

    #[test]
    fn free_accumulate_matches_model_method() {
        let event = Event::new(
            0,
            tags::TIME_LINE,
            vec![Value::from(7), Value::from(2u64), Value::from(9u64)],
        );
        let mut a = ProfileModel::new();
        let mut b = ProfileModel::new();
        accumulate(&mut a, &event).unwrap();
        b.accumulate(&event).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.time_line_events, 1);
        assert_eq!(
            a.line_total(2, 9),
            Some(LineTotal {
                calls: 1,
                ticks: 7
            })
        );
    }
}
