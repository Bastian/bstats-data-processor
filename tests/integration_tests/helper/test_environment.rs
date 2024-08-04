use data_processor::{
    charts::chart::{ChartType, DefaultChartTemplate},
    service::Service,
    software::Software,
};
use redis::AsyncCommands;
use serde_json::json;

use super::redis_testcontainer::RedisTestcontainer;

pub struct TestEnvironment {
    redis_testcontainer: RedisTestcontainer,
    software: Vec<Software>,
    services: Vec<Service>,
}

impl TestEnvironment {
    pub async fn empty() -> Self {
        Self {
            redis_testcontainer: RedisTestcontainer::new().await,
            software: Vec::new(),
            services: Vec::new(),
        }
    }

    pub async fn with_data() -> Self {
        let mut environment: TestEnvironment = Self::empty().await;
        environment.add_software(get_bukkit_software()).await;
        environment.add_software(get_bungeecord_software()).await;
        environment.add_service(get_bukkit_global_service()).await;
        environment
            .add_service(get_bungeecord_global_service())
            .await;
        environment
    }

    pub async fn add_software(&mut self, software: Software) {
        let mut con: redis::aio::MultiplexedConnection = self.redis_multiplexed_connection().await;

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
        let mut con: redis::aio::MultiplexedConnection = self.redis_multiplexed_connection().await;

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

    pub fn redis_client(&self) -> &redis::Client {
        &self.redis_testcontainer.client()
    }

    pub fn software(&self) -> &Vec<Software> {
        &self.software
    }

    pub fn services(&self) -> &Vec<Service> {
        &self.services
    }

    pub async fn redis_multiplexed_connection(&self) -> redis::aio::MultiplexedConnection {
        self.redis_client()
            .get_multiplexed_tokio_connection()
            .await
            .unwrap()
    }
}

pub fn get_bukkit_software() -> Software {
    Software {
        id: 1,
        name: String::from("Bukkit / Spigot"),
        url: String::from("bukkit"),
        global_plugin: Some(1),
        metrics_class: Some(String::from("https://raw.githubusercontent.com/Bastian/bstats-metrics/single-file/bukkit/Metrics.java")),
        example_plugin: Some(String::from("https://github.com/Bastian/bstats-metrics/blob/1.x.x/bstats-bukkit/src/examples/java/ExamplePlugin.java")),
        max_requests_per_ip: 10,
        hide_in_plugin_list: false,
        default_charts: vec![
            DefaultChartTemplate {
                id: String::from("servers"),
                chart_type: ChartType::SingleLineChart,
                title: String::from("Servers using %plugin.name%"),
                data: json!({
                    "lineName": "Servers",
                    "filter": {
                        "enabled": false,
                        "maxValue": 1,
                        "minValue": 1
                    }
                }),
                request_parser: json!({
                    "predefinedValue": 1
                }),
            },
            DefaultChartTemplate {
                id: String::from("players"),
                chart_type: ChartType::SingleLineChart,
                title: String::from("Players on servers using %plugin.name%"),
                data: json!({
                    "lineName": "Players",
                    "filter": {
                        "enabled": true,
                        "maxValue": 200,
                        "minValue": 0
                    }
                }),
                request_parser: json!({
                    "nameInRequest": "playerAmount",
                    "type": "number",
                    "position": "global"
                }),
            },
            DefaultChartTemplate {
                id: String::from("onlineMode"),
                chart_type: ChartType::SimplePie,
                title: String::from("Online mode"),
                data: json!({
                    "filter": {
                        "enabled": false,
                        "useRegex": false,
                        "blacklist": false,
                        "filter": []
                    }
                }),
                request_parser: json!({
                    "nameInRequest": "onlineMode",
                    "position": "global",
                    "type": "boolean",
                    "trueValue": "online",
                    "falseValue": "offline"
                }),
            },
            DefaultChartTemplate {
                id: String::from("minecraftVersion"),
                chart_type: ChartType::SimplePie,
                title: String::from("Minecraft Version"),
                data: json!({
                    "filter": {
                        "enabled": false,
                        "useRegex": false,
                        "blacklist": false,
                        "filter": []
                    }
                }),
                request_parser: json!({
                    "useHardcodedParser": "bukkitMinecraftVersion",
                    "position": "global"
                }),
            },
            DefaultChartTemplate {
                id: String::from("serverSoftware"),
                chart_type: ChartType::SimplePie,
                title: String::from("Server Software"),
                data: json!({
                    "filter": {
                        "enabled": false,
                        "useRegex": false,
                        "blacklist": false,
                        "filter": []
                    }
                }),
                request_parser: json!({
                    "useHardcodedParser": "bukkitServerSoftware",
                    "position": "global"
                }),
            },
            DefaultChartTemplate {
                id: String::from("pluginVersion"),
                chart_type: ChartType::SimplePie,
                title: String::from("Plugin Version"),
                data: json!({
                    "filter": {
                        "enabled": false,
                        "useRegex": false,
                        "blacklist": false,
                        "filter": []
                    }
                }),
                request_parser: json!({
                    "nameInRequest": "pluginVersion",
                    "position": "plugin"
                }),
            },
            DefaultChartTemplate {
                id: String::from("coreCount"),
                chart_type: ChartType::SimplePie,
                title: String::from("Core count"),
                data: json!({
                    "filter": {
                        "enabled": true,
                        "useRegex": true,
                        "blacklist": false,
                        "filter": [
                            "([0-9]){1,2}"
                        ]
                    }
                }),
                request_parser: json!({
                    "nameInRequest": "coreCount",
                    "type": "number",
                    "position": "global"
                }),
            },
            DefaultChartTemplate {
                id: String::from("osArch"),
                chart_type: ChartType::SimplePie,
                title: String::from("System arch"),
                data: json!({
                    "filter": {
                        "enabled": false,
                        "useRegex": false,
                        "blacklist": false,
                        "filter": []
                    }
                }),
                request_parser: json!({
                    "nameInRequest": "osArch",
                    "position": "global"
                }),
            },
            DefaultChartTemplate {
                id: String::from("os"),
                chart_type: ChartType::DrilldownPie,
                title: String::from("Operating System"),
                data: json!({
                    "filter": {
                        "enabled": false,
                        "useRegex": false,
                        "blacklist": false,
                        "filter": []
                    }
                }),
                request_parser: json!({
                    "position": "global",
                    "useHardcodedParser": "os"
                }),
            },
            DefaultChartTemplate {
                id: String::from("location"),
                chart_type: ChartType::SimplePie,
                title: String::from("Server Location"),
                data: json!({
                    "filter": {
                        "enabled": false,
                        "useRegex": false,
                        "blacklist": false,
                        "filter": []
                    }
                }),
                request_parser: json!({
                    "predefinedValue": "%country.name%"
                }),
            },
            DefaultChartTemplate {
                id: String::from("javaVersion"),
                chart_type: ChartType::DrilldownPie,
                title: String::from("Java Version"),
                data: json!({
                    "filter": {
                        "enabled": false,
                        "useRegex": false,
                        "blacklist": false,
                        "filter": []
                    }
                }),
                request_parser: json!({
                    "useHardcodedParser": "javaVersion",
                    "position": "global"
                }),
            },
            DefaultChartTemplate {
                id: String::from("locationMap"),
                chart_type: ChartType::SimpleMap,
                title: String::from("Server Location"),
                data: json!({
                    "valueName": "Servers",
                    "filter": {
                        "enabled": false,
                        "useRegex": false,
                        "blacklist": false,
                        "filter": []
                    }
                }),
                request_parser: json!({
                    "predefinedValue": "AUTO"
                }),
            },
        ]
    }
}

pub fn get_bungeecord_software() -> Software {
    Software {
        id: 2,
        name: String::from("Bungeecord"),
        url: String::from("bungeecord"),
        global_plugin: Some(2),
        metrics_class: Some(String::from("https://raw.githubusercontent.com/Bastian/bstats-metrics/single-file/bungeecord/Metrics.java")),
        example_plugin: Some(String::from("https://github.com/Bastian/bstats-metrics/blob/1.x.x/bstats-bungeecord/src/examples/java/ExamplePlugin.java")),
        max_requests_per_ip: 10,
        hide_in_plugin_list: false,
        default_charts: vec![]
    }
}

pub fn get_bukkit_global_service() -> Service {
    Service {
        id: 1,
        name: String::from("_bukkit_"),
        owner: String::from("Admin"),
        software_id: 1,
        global: true,
        charts: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 38279],
    }
}

pub fn get_bungeecord_global_service() -> Service {
    Service {
        id: 2,
        name: String::from("_bungeecord_"),
        owner: String::from("Admin"),
        software_id: 2,
        global: true,
        charts: vec![21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31],
    }
}
