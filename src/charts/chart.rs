use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

/// The definition of a concrete chart
#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug)]
pub struct Chart {
    /// The unique id of the chart
    pub id: i32,
    /// A custom human-readable id of the chart. Only unique within the service.
    pub id_custom: String,
    #[serde(rename = "type")]
    pub chart_type: ChartType,
    pub position: i32,
    pub title: String,
    /// Whether this chart is a default chart or a custom (user-created) chart.
    ///
    /// Default charts are automatically created and cannot be deleted by the
    /// user.
    pub is_default: bool,
    /// The ID of the service this chart belongs to
    #[serde(rename = "pluginId")]
    pub service_id: i32,
    /// Additional data depending on the chart type.
    ///
    /// For example, for a line chart, it might look like this:
    /// ```json
    /// "lineName": "Players",
    ///  "filter": {
    ///    "enabled": true,
    ///    "maxValue": 200,
    ///    "minValue": 0
    ///  }
    /// ```
    // TODO Better typing or at least document what the data could look like
    //  for each chart type.
    pub data: serde_json::Value,
}

/// A template for a default chart (i.e. how a chart for a newly created service
/// should look like).
#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DefaultChartTemplate {
    /// A human-readable id of the chart. Only unique within the service.
    ///
    /// Will become the `id_custom` of the chart.
    pub id: String,
    #[serde(rename = "type")]
    pub chart_type: ChartType,
    pub title: String,
    /// Additional data depending on the chart type.
    ///
    /// For example, for a line chart, it might look like this:
    /// ```json
    /// "lineName": "Players",
    ///  "filter": {
    ///    "enabled": true,
    ///    "maxValue": 200,
    ///    "minValue": 0
    ///  }
    /// ```
    // TODO Better typing or at least document what the data could look like
    //  for each chart type.
    pub data: serde_json::Value,

    #[serde(rename = "requestParser")]
    pub request_parser: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ChartType {
    #[serde(rename = "single_linechart")]
    SingleLineChart,

    #[serde(rename = "simple_pie")]
    SimplePie,

    #[serde(rename = "advanced_pie")]
    AdvancedPie,

    #[serde(rename = "drilldown_pie")]
    DrilldownPie,

    #[serde(rename = "simple_map")]
    SimpleMap,

    #[serde(rename = "advanced_map")]
    AdvancedMap,

    #[serde(rename = "simple_bar")]
    SimpleBar,

    #[serde(rename = "advanced_bar")]
    AdvancedBar,
}
