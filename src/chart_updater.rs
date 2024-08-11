use std::collections::HashMap;

use redis::AsyncCommands;

use crate::{
    charts::{
        advanced_pie::AdvancedPie,
        chart::ChartType,
        drilldown_pie::DrilldownPie,
        simple_map::SimpleMap,
        simple_pie::SimplePie,
        single_line_chart::{SingleLineChart, SingleLineChartFilter},
        Chart,
    },
    date_util::tms2000_to_timestamp,
    submit_data_schema::SubmitDataChartSchema,
};

pub async fn update_chart<C: AsyncCommands>(
    chart: &Chart,
    data: &SubmitDataChartSchema,
    tms2000: i64,
    country_iso: Option<&str>,
    // Used where possible (currently not possible for line charts)
    pipeline: &mut redis::Pipeline,
    con: &mut C,
) -> Result<(), serde_json::Error> {
    match chart.r#type {
        ChartType::SingleLineChart => {
            let data: SingleLineChart = serde_json::from_value(data.data.clone())?;
            let should_block = match chart.data.get("filter") {
                Some(filter) => {
                    match serde_json::from_value::<SingleLineChartFilter>(filter.clone()) {
                        Ok(filter) => filter.should_block(&data),
                        Err(_) => false,
                    }
                }
                None => false,
            };
            if should_block {
                return Ok(());
            }
            update_line_chart_data(chart.id, tms2000, "1", data.value, con).await;
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
        ChartType::SimpleBar => {
            // TODO Currently not supported
        }
        ChartType::AdvancedBar => {
            // TODO Currently not supported
        }
    }
    Ok(())
}

pub fn update_pie_data(
    service_id: u32,
    chart_id: u64,
    tms2000: i64,
    value_name: &str,
    value: u16,
    pipeline: &mut redis::Pipeline,
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
    value: u16,
    pipeline: &mut redis::Pipeline,
) {
    // The charts are saved the same way
    update_pie_data(service_id, chart_id, tms2000, value_name, value, pipeline);
}

pub async fn update_line_chart_data<C: AsyncCommands>(
    chart_id: u64,
    tms2000: i64,
    line: &str,
    value: i16,
    con: &mut C,
) {
    let key = format!("data:{{{}}}.{}", chart_id, line);
    match con.hincr(key, tms2000_to_timestamp(tms2000), value).await {
        Ok(()) => (),
        Err(e) => {
            // TODO Proper logging framework
            eprintln!("Failed to update line chart data: {}", e);
            ()
        }
    }
}

pub fn update_drilldown_pie_data(
    service_id: u32,
    chart_id: u64,
    tms2000: i64,
    value_name: &str,
    values: HashMap<String, u16>,
    pipeline: &mut redis::Pipeline,
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
