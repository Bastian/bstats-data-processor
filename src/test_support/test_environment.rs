use crate::{
    models::charts::{
        Chart,
        chart::{ChartType, DefaultChartTemplate},
    },
    models::service::Service,
    models::software::Software,
    util::redis::RedisClusterPool,
};
use deadpool_redis::cluster::Connection;
use redis::AsyncCommands;
use serde::Deserialize;
use serde_json::{self, Value};
use std::path::Path;
use tokio::fs;

use super::redis_testcontainer::RedisTestcontainer;

pub struct TestEnvironment {
    redis_testcontainer: RedisTestcontainer,
    software: Vec<Software>,
    services: Vec<Service>,
    charts: Vec<Chart>,
}

impl TestEnvironment {
    pub async fn empty() -> Self {
        Self {
            redis_testcontainer: RedisTestcontainer::new().await,
            software: Vec::new(),
            services: Vec::new(),
            charts: Vec::new(),
        }
    }

    /// Load default test environment
    pub async fn with_data() -> Self {
        TestEnvironment::from_files("src/test_support/environment")
            .await
            .expect("failed to load test data from files")
    }

    /// Load environment from software.json, services.json, charts.json in `dir`
    pub async fn from_files<P: AsRef<Path>>(dir: P) -> Result<Self, Box<dyn std::error::Error>> {
        let dir = dir.as_ref();
        let mut env = Self::empty().await;

        // software.json
        let software_raw = fs::read_to_string(dir.join("software.json")).await?;
        let api_softwares: Vec<ApiSoftware> = serde_json::from_str(&software_raw)?;
        let softwares: Vec<Software> = api_softwares.into_iter().map(map_software).collect();
        for s in softwares {
            env.add_software(s).await;
        }

        // services.json
        let services_raw = fs::read_to_string(dir.join("services.json")).await?;
        let api_services: Vec<ApiService> = serde_json::from_str(&services_raw)?;
        let services: Vec<Service> = api_services.into_iter().map(map_service).collect();
        for svc in services {
            env.add_service(svc).await;
        }

        // charts.json
        let charts_raw = fs::read_to_string(dir.join("charts.json")).await?;
        let api_charts: Vec<ApiChart> = serde_json::from_str(&charts_raw)?;
        let charts: Vec<Chart> = api_charts.into_iter().map(map_chart).collect();
        for ch in charts {
            env.add_chart(ch).await;
        }

        Ok(env)
    }

    pub async fn cleanup(&self) {
        self.redis_testcontainer.cleanup().await;
    }

    pub async fn add_software(&mut self, software: Software) {
        let mut con = self.redis_connection().await;

        let _: () = con.sadd("software.ids", software.id).await.unwrap();
        let _: () = con
            .set(
                format!("software.index.id.url:{}", software.url),
                software.id,
            )
            .await
            .unwrap();

        let cloned_software = software.clone();

        let _: () = con
            .hset_multiple(
                format!("software:{}", software.id),
                &vec![
                    ("name", software.name),
                    ("url", software.url),
                    ("globalPlugin", software.global_plugin.unwrap().to_string()),
                    ("metricsClass", software.metrics_class.unwrap().to_string()),
                    (
                        "examplePlugin",
                        software.example_plugin.unwrap().to_string(),
                    ),
                    ("maxRequestsPerIp", software.max_requests_per_ip.to_string()),
                    (
                        "hideInPluginList",
                        if software.hide_in_plugin_list {
                            String::from("1")
                        } else {
                            String::from("0")
                        },
                    ),
                    (
                        "defaultCharts",
                        serde_json::to_string(&software.default_charts).unwrap(),
                    ),
                ],
            )
            .await
            .unwrap();
        self.software.push(cloned_software);
    }

    pub async fn add_service(&mut self, service: Service) {
        let mut con = self.redis_connection().await;

        let software = self
            .software
            .iter()
            .find(|s| s.id == service.software_id)
            .unwrap();

        let _: () = con.sadd("plugins.ids", service.id).await.unwrap();
        let _: () = con
            .set(
                format!(
                    "plugins.index.id.url+name:{}.{}",
                    software.url,
                    service.name.to_ascii_lowercase()
                ),
                service.id,
            )
            .await
            .unwrap();

        let cloned_service = service.clone();

        let _: () = con
            .hset_multiple(
                format!("plugins:{}", service.id),
                &vec![
                    ("name", service.name),
                    ("owner", service.owner),
                    ("software", service.software_id.to_string()),
                    (
                        "global",
                        if service.global {
                            String::from("1")
                        } else {
                            String::from("0")
                        },
                    ),
                    ("charts", serde_json::to_string(&service.charts).unwrap()),
                ],
            )
            .await
            .unwrap();
        self.services.push(cloned_service);
    }

    pub async fn add_chart(&mut self, chart: Chart) {
        let mut con = self.redis_connection().await;

        let _: () = con.sadd("charts.uids", chart.id).await.unwrap();
        let _: () = con
            .set(
                format!(
                    "charts.index.uid.pluginId+chartId:{}.{}",
                    chart.service_id, chart.id_custom
                ),
                chart.id,
            )
            .await
            .unwrap();

        let cloned_chart = chart.clone();

        let _: () = con
            .hset_multiple(
                format!("charts:{}", chart.id),
                &vec![
                    ("id", chart.id_custom.to_string()),
                    ("pluginId", chart.service_id.to_string()),
                    (
                        "type",
                        serde_json::to_string(&chart.r#type)
                            .unwrap()
                            .trim_matches('"')
                            .to_string(),
                    ),
                    ("position", chart.position.to_string()),
                    ("title", chart.title),
                    (
                        "default",
                        if chart.default {
                            String::from("1")
                        } else {
                            String::from("0")
                        },
                    ),
                    ("data", serde_json::to_string(&chart.data).unwrap()),
                ],
            )
            .await
            .unwrap();

        self.charts.push(cloned_chart);
    }

    pub fn redis_pool(&self) -> &RedisClusterPool {
        &self.redis_testcontainer.pool()
    }

    pub fn software(&self) -> &Vec<Software> {
        &self.software
    }

    pub fn services(&self) -> &Vec<Service> {
        &self.services
    }

    pub fn charts(&self) -> &Vec<Chart> {
        &self.charts
    }

    pub async fn redis_connection(&self) -> Connection {
        self.redis_pool().get().await.unwrap()
    }
}

#[derive(Deserialize)]
struct ApiSoftware {
    id: u16,
    name: String,
    url: String,
    #[serde(rename = "globalPlugin")]
    global_plugin: Option<u32>,
    #[serde(rename = "metricsClass")]
    metrics_class: Option<String>,
    #[serde(rename = "examplePlugin")]
    example_plugin: Option<String>,
    #[serde(rename = "maxRequestsPerIp")]
    max_requests_per_ip: Option<u16>,
    #[serde(rename = "defaultCharts")]
    default_charts: Vec<ApiDefaultChartTemplate>,
    #[serde(rename = "hideInPluginList")]
    hide_in_plugin_list: bool,
}

#[derive(Deserialize)]
struct ApiDefaultChartTemplate {
    #[serde(rename = "idCustom")]
    id_custom: String,
    #[serde(rename = "type")]
    chart_type: ChartType,
    title: String,
    data: Value,
    #[serde(rename = "requestParser")]
    request_parser: Value,
}

#[derive(Deserialize)]
struct ApiService {
    id: u32,
    name: String,
    owner: ApiOwner,
    software: ApiSoftwareRef,
    #[serde(rename = "isGlobal")]
    is_global: bool,
    #[serde(rename = "chartIds")]
    chart_ids: Vec<u64>,
}

#[derive(Deserialize)]
struct ApiOwner {
    name: String,
}

#[derive(Deserialize)]
struct ApiSoftwareRef {
    id: u16,
}

#[derive(Deserialize)]
struct ApiChart {
    id: u64,
    #[serde(rename = "idCustom")]
    id_custom: String,
    #[serde(rename = "type")]
    chart_type: ChartType,
    position: u16,
    title: String,
    #[serde(rename = "isDefault")]
    is_default: bool,
    data: Value,
    #[serde(rename = "serviceId")]
    service_id: u32,
}

fn map_default_chart_template(api: ApiDefaultChartTemplate) -> DefaultChartTemplate {
    DefaultChartTemplate {
        id: api.id_custom,
        chart_type: api.chart_type,
        title: api.title,
        data: api.data,
        request_parser: api.request_parser,
    }
}

fn map_software(api: ApiSoftware) -> Software {
    Software {
        id: api.id,
        name: api.name,
        url: api.url,
        global_plugin: api.global_plugin,
        metrics_class: api.metrics_class,
        example_plugin: api.example_plugin,
        max_requests_per_ip: api.max_requests_per_ip.unwrap_or(0),
        hide_in_plugin_list: api.hide_in_plugin_list,
        default_charts: api
            .default_charts
            .into_iter()
            .map(map_default_chart_template)
            .collect(),
    }
}

fn map_service(api: ApiService) -> Service {
    Service {
        id: api.id,
        name: api.name,
        owner: api.owner.name,
        software_id: api.software.id,
        global: api.is_global,
        charts: api.chart_ids,
    }
}

fn map_chart(api: ApiChart) -> Chart {
    Chart {
        id: api.id,
        id_custom: api.id_custom,
        r#type: api.chart_type,
        position: api.position,
        title: api.title,
        default: api.is_default,
        data: api.data,
        service_id: api.service_id,
    }
}
