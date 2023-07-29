#![allow(dead_code)]
use std::error::Error;
use std::process;

use csv;
use serde;
use serde::de::{self, Deserializer, Unexpected};
use serde::Deserialize;

/// Deserialize bool from String with custom value mapping
fn bool_from_string<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    match String::deserialize(deserializer)?.as_ref() {
        "y" | "Y" => Ok(true),
        "n" | "N" => Ok(false),
        other => Err(de::Error::invalid_value(Unexpected::Str(other), &"y or n")),
    }
}

fn vec_from_string<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw: String = String::deserialize(deserializer)?;
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    let parts = raw.split(';').map(|s| s.to_string()).collect();
    Ok(parts)
}

#[derive(Debug, Deserialize)]
struct Cost {
    #[serde(rename = "cost", deserialize_with = "csv::invalid_option")]
    coin: Option<i8>,
    #[serde(rename = "potion cost", deserialize_with = "csv::invalid_option")]
    potion: Option<i8>,
    #[serde(deserialize_with = "csv::invalid_option")]
    debt: Option<i8>,
}

#[derive(Debug, Deserialize)]
struct Card {
    #[serde(rename = "Card title")]
    name: String,
    #[serde(rename = "Pile title")]
    pile: String,
    #[serde(rename = "Group")]
    card_group: String,
    #[serde(rename = "in supply", deserialize_with = "bool_from_string")]
    supply: bool,
    #[serde(rename = "is kingdom", deserialize_with = "bool_from_string")]
    kingdom: bool,
    types: String,
    #[serde(flatten)]
    cost: Cost,
    #[serde(deserialize_with = "vec_from_string")]
    keywords: Vec<String>,
}

fn example() -> Result<(), Box<dyn Error>> {
    let mut reader = csv::Reader::from_path("cards.dat")?;
    for (line_no, result) in reader.deserialize::<Card>().enumerate() {
        let Ok(card) = result else {
            println!("Could not parse line {}", line_no+2);  
            continue;
        };
        println!("{:?}", card);
    }
    Ok(())
}

fn main() {
    if let Err(err) = example() {
        println!("error running example: {}", err);
        process::exit(1);
    }
}
