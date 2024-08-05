use phf::phf_ordered_map;
use serde_json::{json, Value};

use crate::{charts::simple_pie::SimplePie, submit_data_schema::SubmitDataSchema};

use super::Parser;

pub struct BukkitServerSoftwareParser;

impl Parser for BukkitServerSoftwareParser {
    fn parse(&self, schema: &SubmitDataSchema) -> Option<Value> {
        let software_name = parse_bukkit_server_software(
            schema
                .extra
                .get("bukkitVersion")
                .and_then(|v| v.as_str())
                .as_deref(),
            schema
                .extra
                .get("bukkitName")
                .and_then(|v| v.as_str())
                .as_deref(),
        )?;
        Some(json!(SimplePie {
            value: software_name
        }))
    }
}

static SERVER_SOFTWARE_BRANDS: phf::OrderedMap<&'static str, &'static str> = phf_ordered_map! {
    "bukkit" => "Bukkit", // https://github.com/Bukkit/Bukkit, EOL
    // The order is important here -> TacoSpigot before Spigot or it will be detected as Spigot
    "taco" => "TacoSpigot", // https://github.com/TacoSpigot/TacoSpigot, EOL
    "paper" => "Paper", // https://github.com/PaperMC/Paper
    "folia" => "Folia", // https://github.com/PaperMC/Folia
    "spigot" => "Spigot", // https://hub.spigotmc.org/stash/projects/SPIGOT/repos/spigot/browse
    "catserver" => "CatServer", // https://github.com/Luohuayu/CatServer/
    "lava" => "Lava", // https://github.com/Timardo/Lava, EOL
    "mohist" => "Mohist", // https://github.com/MohistMC/Mohist
    "tuinity" => "Tuinity", // https://github.com/Tuinity/Tuinity, EOL
    "purpur" => "Purpur", // https://github.com/PurpurMC/Purpur
    "airplane" => "Airplane", // https://github.com/TECHNOVE/Airplane, EOL
    "yatopia" => "Yatopia", // https://github.com/YatopiaMC/Yatopia, EOL
    "arclight" => "Arclight", // https://github.com/IzzelAliz/Arclight
    "magma" => "Magma", // https://github.com/magmafoundation/Magma, EOL
    "titanium" => "Titanium", // https://github.com/Mythic-Projects/Titanium, EOL
    "scissors" => "Scissors", // https://github.com/AtlasMediaGroup/Scissors
    "gale" => "Gale", // https://github.com/GaleMC/Gale
    "glowstone" => "Glowstone", // https://github.com/GlowstoneMC/Glowstone, EOL
    "pufferfish" => "Pufferfish", // https://github.com/pufferfish-gg/Pufferfish
    "leaves" => "Leaves", // https://github.com/LeavesMC/Leaves
    "leaf" => "Leaf", // https://github.com/Winds-Studio/Leaf
};

fn parse_bukkit_server_software(
    bukkit_version: Option<&str>,
    bukkit_name: Option<&str>,
) -> Option<String> {
    let bukkit_version = bukkit_version?;

    // If it doesn't contain "MC: ", it's from an old bStats Metrics class
    if !bukkit_version.contains("MC:") {
        return None;
    }

    // First try to find the software name based on the bukkit version
    let bukkit_version_lower = bukkit_version.to_ascii_lowercase();
    let software_name = SERVER_SOFTWARE_BRANDS
        .entries()
        .find(|(brand, _)| bukkit_version_lower.contains(*brand))
        .map(|(_, name)| *name)
        .or_else(|| {
            // Then try to find the software name based on the bukkit name
            bukkit_name.and_then(|bukkit_name| {
                SERVER_SOFTWARE_BRANDS
                    .get(&bukkit_name.to_ascii_lowercase().as_str())
                    .cloned()
            })
        })
        .unwrap_or_else(|| {
            println!(
                "Unknown server software: bukkitVersion='{}', bukkitName='{}'",
                bukkit_version,
                bukkit_name.unwrap_or("<not set>")
            );
            "Unknown"
        });

    Some(String::from(software_name))
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_parse_bukkit_server_software() {
        struct TestCase {
            name: String,
            bukkit_version: String,
            bukkit_name: String,
            expected: String,
        }

        let test_cases = vec![
            TestCase {
                name: String::from("Paper 1.21"),
                bukkit_version: String::from("1.21-38-1f5db50 (MC: 1.21)"),
                bukkit_name: String::from("Paper"),
                expected: String::from("Paper"),
            },
            TestCase {
                name: String::from("Paper 1.20"),
                bukkit_version: String::from("git-Paper-196 (MC: 1.20.1)"),
                bukkit_name: String::from("Paper"),
                expected: String::from("Paper"),
            },
            TestCase {
                name: String::from("Spigot 1.21"),
                bukkit_version: String::from("4226-Spigot-146439e-2889b3a (MC: 1.21)"),
                bukkit_name: String::from("CraftBukkit"),
                expected: String::from("Spigot"),
            },
            TestCase {
                name: String::from("Purpur 1.21"),
                bukkit_version: String::from("1.21-2250-797ce6b (MC: 1.21)"),
                bukkit_name: String::from("Purpur"),
                expected: String::from("Purpur"),
            },
            TestCase {
                name: String::from("Purpur 1.20"),
                bukkit_version: String::from("git-Purpur-2062 (MC: 1.20.1)"),
                bukkit_name: String::from("Purpur"),
                expected: String::from("Purpur"),
            },
            TestCase {
                name: String::from("Glowstone 1.12.2"),
                bukkit_version: String::from("2021.7.0-SNAPSHOT.09043bd (MC: 1.12.2)"),
                bukkit_name: String::from("Glowstone"),
                expected: String::from("Glowstone"),
            },
            TestCase {
                name: String::from("\"CraftBukkit\" 1.21"),
                bukkit_version: String::from("4226-Bukkit-2889b3a (MC: 1.21)"),
                bukkit_name: String::from("CraftBukkit"),
                expected: String::from("Bukkit"),
            },
            TestCase {
                name: String::from("CatServer 1.18"),
                bukkit_version: String::from("1.18.2-edda1229 (MC: 1.18.2)"),
                bukkit_name: String::from("CatServer"),
                expected: String::from("CatServer"),
            },
            TestCase {
                name: String::from("Folia 1.20.6"),
                bukkit_version: String::from("1.20.6-5-d797082 (MC: 1.20.6)"),
                bukkit_name: String::from("Folia"),
                expected: String::from("Folia"),
            },
            TestCase {
                name: String::from("Folia 1.20.1"),
                bukkit_version: String::from("git-Folia-17 (MC: 1.20.1)"),
                bukkit_name: String::from("Folia"),
                expected: String::from("Folia"),
            },
            TestCase {
                name: String::from("Arclight 1.21"),
                bukkit_version: String::from("arclight-1.21-1.0.0-SNAPSHOT-b3349e9 (MC: 1.21)"),
                bukkit_name: String::from("Arclight"),
                expected: String::from("Arclight"),
            },
            TestCase {
                name: String::from("TacoSpigot 1.9.4"),
                bukkit_version: String::from("git-TacoSpigot-\"af15657\" (MC: 1.9.4)"),
                bukkit_name: String::from("Paper"),
                expected: String::from("TacoSpigot"),
            },
            TestCase {
                name: String::from("Leaves 1.20.6"),
                bukkit_version: String::from("1.20.6-215-e234432 (MC: 1.20.6)"),
                bukkit_name: String::from("Leaves"),
                expected: String::from("Leaves"),
            },
            TestCase {
                name: String::from("Leaf 1.21"),
                bukkit_version: String::from("1.21-DEV-3d7de13 (MC: 1.21)"),
                bukkit_name: String::from("Leaf"),
                expected: String::from("Leaf"),
            },
            TestCase {
                name: String::from("Pufferfish 1.20.4"),
                bukkit_version: String::from("git-Pufferfish-52 (MC: 1.20.4)"),
                bukkit_name: String::from("Pufferfish"),
                expected: String::from("Pufferfish"),
            },
        ];

        for test_case in test_cases {
            let result = parse_bukkit_server_software(
                Some(&test_case.bukkit_version),
                Some(&test_case.bukkit_name),
            );

            assert_eq!(
                result.unwrap(),
                test_case.expected,
                "Wrong result for {}",
                test_case.name
            );
        }
    }
}
