use std::collections::HashMap;

use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::{Value, json};

use crate::{models::charts::drilldown_pie::DrilldownPie, parser::ParserInput};

use super::Parser;

pub struct OsParser;

impl Parser for OsParser {
    fn parse(&self, input: &ParserInput) -> Option<Value> {
        let (outer, inner) = match (global_str(input, "osName"), global_str(input, "osVersion")) {
            (Some(os_name), Some(os_version)) => parse_os(os_name, os_version),
            _ => {
                // Non-Java clients send raw values, so the mapping can be fixed here instead of in every client
                let (os_name, os_version) = node_to_java_os(
                    global_str(input, "nodeOsType")?,
                    global_str(input, "nodeOsRelease")?,
                    global_str(input, "nodeOsVersion"),
                );
                parse_os(&os_name, &os_version)
            }
        };
        Some(json!(DrilldownPie {
            values: HashMap::from([(outer, HashMap::from([(inner, 1),])),])
        }))
    }
}

fn global_str<'a>(input: &ParserInput<'a>, key: &str) -> Option<&'a str> {
    input.global.get(key).and_then(Value::as_str)
}

/// Maps Node's `os.type()`, `os.release()` and `os.version()` to Java's `os.name` and `os.version`.
fn node_to_java_os(os_type: &str, release: &str, version: Option<&str>) -> (String, String) {
    match os_type {
        "Darwin" => (String::from("Mac OS X"), macos_version(release)),
        "Windows_NT" => (windows_edition(release, version), release.to_string()),
        _ => (os_type.to_string(), release.to_string()),
    }
}

/// Darwin 20 to 24 are macOS 11 to 15, then macOS jumped to 26 with Darwin 25.
fn macos_version(darwin_release: &str) -> String {
    let mut parts = darwin_release.split('.');
    let Some(Ok(major)) = parts.next().map(str::parse::<u32>) else {
        return darwin_release.to_string();
    };
    let minor: u32 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    match major {
        0..=19 => format!("10.{}", major.saturating_sub(4)),
        20..=24 => format!("{}.{}", major - 9, minor),
        _ => format!("{}.{}", major + 1, minor),
    }
}

static WINDOWS_EDITION: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^Windows(?: Server)? \d+(?:\.\d+)?(?: R2)?").unwrap());

/// Windows 11's product name still says Windows 10, so the build number decides.
fn windows_edition(release: &str, version: Option<&str>) -> String {
    let Some(edition) = version.and_then(|v| WINDOWS_EDITION.find(v)) else {
        return String::from("Windows");
    };
    let build: u32 = release
        .split('.')
        .nth(2)
        .and_then(|b| b.parse().ok())
        .unwrap_or(0);
    match edition.as_str() {
        "Windows 10" if build >= 22000 => String::from("Windows 11"),
        edition => edition.to_string(),
    }
}

fn parse_os(os_name: &str, os_version: &str) -> (String, String) {
    match os_name {
        // In Linux, the os name is "Linux" and the os version is the kernel version.
        os if os.starts_with("Linux") => (String::from("Linux"), os_version.to_string()),
        os if os.starts_with("Windows Server") => (String::from("Windows Server"), os.to_string()),
        os if os.starts_with("Windows NT") => (String::from("Windows NT"), os.to_string()),
        os if os.starts_with("Windows") => (String::from("Windows"), os.to_string()),
        os if os.starts_with("Mac OS X") => {
            (String::from("Mac OS X"), format!("Mac OS X {}", os_version))
        }
        os if os.contains("BSD") => (String::from("BSD"), format!("{} {}", os, os_version)),
        os => (String::from("Other"), format!("{} ({})", os, os_version)),
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_parse_os() {
        struct TestCase {
            name: String,
            os_name: String,
            os_version: String,
            expected: (String, String),
        }

        let test_cases = vec![
            TestCase {
                name: String::from("Windows Server 2022"),
                os_name: String::from("Windows Server 2022"),
                os_version: String::from(""),
                expected: (
                    String::from("Windows Server"),
                    String::from("Windows Server 2022"),
                ),
            },
            TestCase {
                name: String::from("Windows Server 2012 R2"),
                os_name: String::from("Windows Server 2012 R2"),
                os_version: String::from(""),
                expected: (
                    String::from("Windows Server"),
                    String::from("Windows Server 2012 R2"),
                ),
            },
            TestCase {
                name: String::from("Windows 11"),
                os_name: String::from("Windows 11"),
                os_version: String::from("10.0"),
                expected: (String::from("Windows"), String::from("Windows 11")),
            },
            TestCase {
                name: String::from("Mac OS X 14.5"),
                os_name: String::from("Mac OS X"),
                os_version: String::from("14.5"),
                expected: (String::from("Mac OS X"), String::from("Mac OS X 14.5")),
            },
            TestCase {
                name: String::from("FreeBSD 13.1"),
                os_name: String::from("FreeBSD"),
                os_version: String::from("13.1-RELEASE-p9"),
                expected: (String::from("BSD"), String::from("FreeBSD 13.1-RELEASE-p9")),
            },
            TestCase {
                name: String::from("Linux"),
                os_name: String::from("Linux"),
                os_version: String::from("5.15.0-105-generic"),
                expected: (String::from("Linux"), String::from("5.15.0-105-generic")),
            },
            TestCase {
                name: String::from("Linux WSL2"),
                os_name: String::from("Linux"),
                os_version: String::from("5.15.153.1-microsoft-standard-WSL2"),
                expected: (
                    String::from("Linux"),
                    String::from("5.15.153.1-microsoft-standard-WSL2"),
                ),
            },
            TestCase {
                name: String::from("Made-Up OS"),
                os_name: String::from("bStats OS"),
                os_version: String::from("1.2.3"),
                expected: (String::from("Other"), String::from("bStats OS (1.2.3)")),
            },
        ];

        for test_case in test_cases {
            let result = parse_os(&test_case.os_name, &test_case.os_version);
            assert_eq!(
                result, test_case.expected,
                "Wrong result for {}",
                test_case.name
            );
        }
    }

    #[test]
    fn test_parse_node_os() {
        struct TestCase {
            name: &'static str,
            os_type: &'static str,
            release: &'static str,
            version: Option<&'static str>,
            expected: (&'static str, &'static str),
        }

        let test_cases = vec![
            TestCase {
                name: "Linux",
                os_type: "Linux",
                release: "6.1.0-18-amd64",
                version: Some("#1 SMP PREEMPT_DYNAMIC Debian 6.1.76-1 (2024-02-01)"),
                expected: ("Linux", "6.1.0-18-amd64"),
            },
            TestCase {
                name: "macOS 10.15",
                os_type: "Darwin",
                release: "19.6.0",
                version: Some("Darwin Kernel Version 19.6.0"),
                expected: ("Mac OS X", "Mac OS X 10.15"),
            },
            TestCase {
                name: "macOS 14.5",
                os_type: "Darwin",
                release: "23.5.0",
                version: Some("Darwin Kernel Version 23.5.0"),
                expected: ("Mac OS X", "Mac OS X 14.5"),
            },
            TestCase {
                name: "macOS 15.6",
                os_type: "Darwin",
                release: "24.6.0",
                version: Some("Darwin Kernel Version 24.6.0"),
                expected: ("Mac OS X", "Mac OS X 15.6"),
            },
            TestCase {
                name: "macOS 26.1",
                os_type: "Darwin",
                release: "25.1.0",
                version: Some("Darwin Kernel Version 25.1.0"),
                expected: ("Mac OS X", "Mac OS X 26.1"),
            },
            TestCase {
                name: "Unparsable Darwin release",
                os_type: "Darwin",
                release: "unknown",
                version: None,
                expected: ("Mac OS X", "Mac OS X unknown"),
            },
            TestCase {
                name: "Windows 10",
                os_type: "Windows_NT",
                release: "10.0.19045",
                version: Some("Windows 10 Pro"),
                expected: ("Windows", "Windows 10"),
            },
            TestCase {
                name: "Windows 11 named Windows 10",
                os_type: "Windows_NT",
                release: "10.0.22631",
                version: Some("Windows 10 Pro"),
                expected: ("Windows", "Windows 11"),
            },
            TestCase {
                name: "Windows 11",
                os_type: "Windows_NT",
                release: "10.0.26100",
                version: Some("Windows 11 Home"),
                expected: ("Windows", "Windows 11"),
            },
            TestCase {
                name: "Windows 8.1",
                os_type: "Windows_NT",
                release: "6.3.9600",
                version: Some("Windows 8.1 Pro"),
                expected: ("Windows", "Windows 8.1"),
            },
            TestCase {
                name: "Windows Server 2022",
                os_type: "Windows_NT",
                release: "10.0.20348",
                version: Some("Windows Server 2022 Datacenter"),
                expected: ("Windows Server", "Windows Server 2022"),
            },
            TestCase {
                name: "Windows Server 2012 R2",
                os_type: "Windows_NT",
                release: "6.3.9600",
                version: Some("Windows Server 2012 R2 Standard"),
                expected: ("Windows Server", "Windows Server 2012 R2"),
            },
            TestCase {
                name: "Windows without version",
                os_type: "Windows_NT",
                release: "10.0.22631",
                version: None,
                expected: ("Windows", "Windows"),
            },
            TestCase {
                name: "FreeBSD",
                os_type: "FreeBSD",
                release: "13.1-RELEASE-p9",
                version: Some("FreeBSD 13.1-RELEASE-p9 GENERIC"),
                expected: ("BSD", "FreeBSD 13.1-RELEASE-p9"),
            },
            TestCase {
                name: "Made-Up OS",
                os_type: "bStats OS",
                release: "1.2.3",
                version: None,
                expected: ("Other", "bStats OS (1.2.3)"),
            },
        ];

        for test_case in test_cases {
            let (os_name, os_version) =
                node_to_java_os(test_case.os_type, test_case.release, test_case.version);
            let (outer, inner) = parse_os(&os_name, &os_version);
            assert_eq!(
                (outer.as_str(), inner.as_str()),
                test_case.expected,
                "Wrong result for {}",
                test_case.name
            );
        }
    }

    fn parse(global: Value) -> Option<Value> {
        let global: HashMap<String, Value> = serde_json::from_value(global).unwrap();
        OsParser.parse(&ParserInput {
            global: &global,
            service: &HashMap::new(),
        })
    }

    #[test]
    fn prefers_java_fields_over_node_fields() {
        let result = parse(json!({
            "osName": "Linux",
            "osVersion": "6.1.0",
            "nodeOsType": "Darwin",
            "nodeOsRelease": "23.5.0",
        }));
        assert_eq!(result, Some(json!({"values": {"Linux": {"6.1.0": 1}}})));
    }

    #[test]
    fn falls_back_to_node_fields() {
        let result = parse(json!({
            "nodeOsType": "Darwin",
            "nodeOsRelease": "23.5.0",
            "nodeOsVersion": "Darwin Kernel Version 23.5.0",
        }));
        assert_eq!(
            result,
            Some(json!({"values": {"Mac OS X": {"Mac OS X 14.5": 1}}}))
        );
    }

    #[test]
    fn ignores_requests_without_os_fields() {
        assert_eq!(parse(json!({"osArch": "amd64"})), None);
    }
}
