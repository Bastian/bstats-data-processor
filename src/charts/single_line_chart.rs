use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::charts::chart::ChartFilter;

#[derive(Debug, Validate, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
pub struct SingleLineChart {
    pub value: i16,
}

#[derive(Debug, Validate, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
pub struct SingleLineChartFilter {
    pub enabled: bool,
    #[serde(rename = "maxValue")]
    pub max_value: Option<i16>,
    #[serde(rename = "minValue")]
    pub min_value: Option<i16>,
}

impl SingleLineChartFilter {
    // Clamp a value according to the filter's (possibly partial) bounds.
    // If both bounds are present and inverted (min > max), we normalize by
    // swapping.
    fn clamp_value(&self, v: i16) -> i16 {
        match (self.min_value, self.max_value) {
            (Some(lo), Some(hi)) => v.clamp(lo.min(hi), lo.max(hi)),
            (Some(lo), None) => v.max(lo),
            (None, Some(hi)) => v.min(hi),
            (None, None) => v,
        }
    }
}

impl ChartFilter<SingleLineChart> for SingleLineChartFilter {
    fn filter(&self, data: &SingleLineChart) -> Option<SingleLineChart> {
        if !self.enabled {
            return Some(*data);
        }
        let clamped = self.clamp_value(data.value);
        Some(SingleLineChart { value: clamped })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn does_not_filter_when_disabled() {
        let filter = SingleLineChartFilter {
            enabled: false,
            max_value: Some(5),
            min_value: Some(0),
        };
        let data = SingleLineChart { value: 150 };
        let filtered = filter.filter(&data);
        assert_eq!(filtered, Some(data));
    }

    #[test]
    fn clamps_values() {
        let filter = SingleLineChartFilter {
            enabled: true,
            max_value: Some(100),
            min_value: Some(50),
        };
        // Upper bound
        let data = SingleLineChart { value: 150 };
        let filtered = filter.filter(&data);
        assert_eq!(filtered, Some(SingleLineChart { value: 100 }));

        // Lower bound
        let data = SingleLineChart { value: 25 };
        let filtered = filter.filter(&data);
        assert_eq!(filtered, Some(SingleLineChart { value: 50 }));
    }

    #[test]
    fn does_not_filter_when_within_bounds() {
        let filter = SingleLineChartFilter {
            enabled: true,
            max_value: Some(100),
            min_value: Some(0),
        };
        let data = SingleLineChart { value: 75 };
        let filtered = filter.filter(&data);
        assert_eq!(filtered, Some(data));
    }

    #[test]
    fn works_with_negative_values() {
        let filter = SingleLineChartFilter {
            enabled: true,
            max_value: Some(-10),
            min_value: Some(-100),
        };
        let data = SingleLineChart { value: -150 };
        let filtered = filter.filter(&data);
        assert_eq!(filtered, Some(SingleLineChart { value: -100 }));

        let data = SingleLineChart { value: -5 };
        let filtered = filter.filter(&data);
        assert_eq!(filtered, Some(SingleLineChart { value: -10 }));

        let data = SingleLineChart { value: -50 };
        let filtered = filter.filter(&data);
        assert_eq!(filtered, Some(data));
    }

    #[test]
    fn works_with_partial_bounds() {
        let filter = SingleLineChartFilter {
            enabled: true,
            max_value: Some(100),
            min_value: None,
        };
        let data = SingleLineChart { value: 150 };
        let filtered = filter.filter(&data);
        assert_eq!(filtered, Some(SingleLineChart { value: 100 }));

        let data = SingleLineChart { value: 50 };
        let filtered = filter.filter(&data);
        assert_eq!(filtered, Some(data));

        let filter = SingleLineChartFilter {
            enabled: true,
            max_value: None,
            min_value: Some(0),
        };
        let data = SingleLineChart { value: -50 };
        let filtered = filter.filter(&data);
        assert_eq!(filtered, Some(SingleLineChart { value: 0 }));

        let data = SingleLineChart { value: 50 };
        let filtered = filter.filter(&data);
        assert_eq!(filtered, Some(data));
    }

    #[test]
    fn does_not_clamp_on_bound_equality() {
        let filter = SingleLineChartFilter {
            enabled: true,
            max_value: Some(100),
            min_value: Some(50),
        };

        let data = SingleLineChart { value: 100 };
        assert_eq!(filter.filter(&data), Some(data));

        let data = SingleLineChart { value: 50 };
        assert_eq!(filter.filter(&data), Some(data));
    }

    #[test]
    fn works_with_no_bounds() {
        let filter: SingleLineChartFilter = SingleLineChartFilter {
            enabled: true,
            max_value: None,
            min_value: None,
        };
        let data = SingleLineChart { value: 42 };
        assert_eq!(filter.filter(&data), Some(data));
    }

    #[test]
    fn works_with_extreme_values() {
        let filter = SingleLineChartFilter {
            enabled: true,
            max_value: Some(i16::MAX),
            min_value: Some(i16::MIN),
        };

        let data = SingleLineChart { value: i16::MAX };
        assert_eq!(filter.filter(&data), Some(data));

        let data = SingleLineChart { value: i16::MIN };
        assert_eq!(filter.filter(&data), Some(data));
    }

    #[test]
    fn works_with_inverted_bounds() {
        let filter = SingleLineChartFilter {
            enabled: true,
            max_value: Some(0),
            min_value: Some(100),
        };
        let data = SingleLineChart { value: 50 };
        // After normalization, bounds become [0, 100]; 50 is within
        let filtered: Option<SingleLineChart> = filter.filter(&data);
        assert_eq!(filtered, Some(data));

        let data = SingleLineChart { value: 150 };
        let filtered = filter.filter(&data);
        assert_eq!(filtered, Some(SingleLineChart { value: 100 }));

        let data = SingleLineChart { value: -10 };
        let filtered = filter.filter(&data);
        assert_eq!(filtered, Some(SingleLineChart { value: 0 }));
    }
}
