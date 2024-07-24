use std::collections::HashMap;

use serde_json::{json, Value};

use crate::charts::drilldown_pie::DrilldownPie;

use super::Parser;

pub struct OsParser;

impl Parser for OsParser {
    fn parse(&self, schema: crate::submit_data_schema::SubmitDataSchema) -> Option<Value> {
        let os_name = schema.extra.get("osName").and_then(|v| v.as_str())?;
        let os_version = schema.extra.get("osVersion").and_then(|v| v.as_str())?;
        let (outer, inner) = parse_os(os_name, os_version);
        Some(json!(DrilldownPie {
            values: HashMap::from([(outer, HashMap::from([(inner, 1),])),])
        }))
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
}
