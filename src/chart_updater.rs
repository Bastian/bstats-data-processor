use std::collections::HashMap;

use crate::util::slot_pipeline::SlotPipeline;
use crate::{
    models::charts::{
        Chart,
        advanced_pie::AdvancedPie,
        bar::Bar,
        chart::ChartType,
        drilldown_pie::DrilldownPie,
        simple_map::SimpleMap,
        simple_pie::SimplePie,
        single_line_chart::{SingleLineChart, SingleLineChartFilter},
    },
    submit_data_schema::SubmitDataChartSchema,
    util::date::tms2000_to_timestamp,
};

use crate::models::charts::chart::ChartFilter;

pub fn update_chart(
    chart: &Chart,
    data: &SubmitDataChartSchema,
    tms2000: i64,
    country_iso: Option<&str>,
    pipeline: &mut SlotPipeline,
) -> Result<(), serde_json::Error> {
    match chart.r#type {
        ChartType::SingleLineChart => {
            let data: SingleLineChart = serde_json::from_value(data.data.clone())?;
            let data = match chart.data.get("filter") {
                Some(filter) => {
                    match serde_json::from_value::<SingleLineChartFilter>(filter.clone()) {
                        Ok(filter) => filter.filter(&data),
                        Err(_) => Some(data),
                    }
                }
                None => Some(data),
            };
            if let Some(data) = data {
                update_line_chart_data(chart.id, tms2000, "1", data.value, pipeline);
            }
        }
        ChartType::SimplePie => {
            let data: SimplePie = serde_json::from_value(data.data.clone())?;
            update_pie_data(
                chart.service_id,
                chart.id,
                tms2000,
                &data.value,
                1,
                pipeline,
            );
        }
        ChartType::AdvancedPie => {
            let data: AdvancedPie = serde_json::from_value(data.data.clone())?;
            for (value_name, value) in data.values.iter() {
                update_pie_data(
                    chart.service_id,
                    chart.id,
                    tms2000,
                    value_name,
                    *value,
                    pipeline,
                );
            }
        }
        ChartType::DrilldownPie => {
            let data: DrilldownPie = serde_json::from_value(data.data.clone())?;
            for (value_name, values) in data.values.iter() {
                update_drilldown_pie_data(
                    chart.service_id,
                    chart.id,
                    tms2000,
                    value_name,
                    values.clone(),
                    pipeline,
                );
            }
        }
        ChartType::SimpleMap => {
            let data: SimpleMap = serde_json::from_value(data.data.clone())?;
            update_map_data(
                chart.service_id,
                chart.id,
                tms2000,
                if &data.value == "AUTO" {
                    if let Some(country_iso) = country_iso {
                        country_iso
                    } else {
                        return Ok(());
                    }
                } else {
                    &data.value
                },
                1,
                pipeline,
            );
        }
        ChartType::AdvancedMap => {
            // TODO Currently not supported
        }
        ChartType::SimpleBar | ChartType::AdvancedBar => {
            let data: Bar = serde_json::from_value(data.data.clone())?;
            for (category, bar_values) in data.values.iter() {
                update_bar_chart_data(
                    chart.service_id,
                    chart.id,
                    tms2000,
                    category,
                    bar_values,
                    pipeline,
                );
            }
        }
    }
    Ok(())
}

pub fn update_pie_data(
    service_id: u32,
    chart_id: u64,
    tms2000: i64,
    value_name: &str,
    value: u32,
    pipeline: &mut SlotPipeline,
) {
    let key = format!("data:{{{}}}.{}.{}", service_id, chart_id, tms2000);
    pipeline.zincr(&key, value_name, value);
    pipeline.expire(&key, 60 * 61);
}

pub fn update_map_data(
    service_id: u32,
    chart_id: u64,
    tms2000: i64,
    value_name: &str,
    value: u32,
    pipeline: &mut SlotPipeline,
) {
    // The charts are saved the same way
    update_pie_data(service_id, chart_id, tms2000, value_name, value, pipeline);
}

/// Line chart keys carry no hash tag, so they usually end up in a different
/// slot than the rest of the request. [`SlotPipeline`] groups them accordingly.
pub fn update_line_chart_data(
    chart_id: u64,
    tms2000: i64,
    line: &str,
    value: i32,
    pipeline: &mut SlotPipeline,
) {
    let key = format!("data:{}.{}", chart_id, line);
    pipeline.hincr(&key, tms2000_to_timestamp(tms2000) * 1000, value);
}

const MAX_BARS_PER_CATEGORY: usize = 25;

/// Accumulates bar values in a hash under the field `<category>:<bar_index>`,
/// matching the format the backend reads in `getBarChartData`.
pub fn update_bar_chart_data(
    service_id: u32,
    chart_id: u64,
    tms2000: i64,
    category: &str,
    bar_values: &[i64],
    pipeline: &mut SlotPipeline,
) {
    // The backend splits hash fields on `:`, so a category containing `:` would
    // be mis-parsed into a different category. Drop it instead of corrupting data.
    if category.contains(':') {
        return;
    }
    let key = format!("data:{{{}}}.{}.{}", service_id, chart_id, tms2000);
    for (bar_index, bar_value) in bar_values.iter().take(MAX_BARS_PER_CATEGORY).enumerate() {
        pipeline.hincr(&key, format!("{}:{}", category, bar_index), *bar_value);
    }
    pipeline.expire(&key, 60 * 61);
}

pub fn update_drilldown_pie_data(
    service_id: u32,
    chart_id: u64,
    tms2000: i64,
    value_name: &str,
    values: HashMap<String, u32>,
    pipeline: &mut SlotPipeline,
) {
    let mut total_value = 0;
    for (value_key, value) in values.iter() {
        total_value += value;
        let key = format!(
            "data:{{{}}}.{}.{}.{}",
            service_id, chart_id, tms2000, value_name
        );
        pipeline.zincr(&key, value_key, value);
        pipeline.expire(&key, 60 * 61);
    }
    let key = format!("data:{{{}}}.{}.{}", service_id, chart_id, tms2000);
    pipeline.zincr(&key, value_name, total_value);
    pipeline.expire(&key, 60 * 61);
}
