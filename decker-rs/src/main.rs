use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;

use std::process::exit;

use clap::Parser;

static MAXCOINCOST: i8 = 11;

const MANY: u64 = 5000;

mod actions;
mod bad_rand;
mod cards;
mod collections;
mod config;
mod constraints;
mod costs;
mod piles;
mod properties;
mod selections;

use collections::{CardCollectionPtr, CollectionStatus};
use config::load_config;

// A bunch of utility functions that will be removed later

type StringMultiMap = std::collections::BTreeMap<String, Vec<String>>;

fn short_value(s: &str) -> i8 {
    s.parse::<i8>().unwrap_or(-1)
}

fn bool_value(s: &str) -> bool {
    s == "Y" || s == "y"
}

fn string_split(s: &str, sep: char) -> Vec<String> {
    let mut v = vec![];
    for i in s.split(sep) {
        v.push(i.to_string());
    }
    v
}

fn no_empty_split(s: &String, sep: char) -> Vec<String> {
    if s.is_empty() {
        return Vec::<String>::new();
    }
    string_split(s, sep)
}

fn read_boxes(fname: &String) -> Result<StringMultiMap, String> {
    let ifs = match File::open(Path::new(fname)) {
        Err(_) => return Err("Can't open file".to_string()),
        Ok(f) => f,
    };
    let input = BufReader::new(ifs);
    let mut res = StringMultiMap::new();
    for (num, item) in input.lines().enumerate() {
        let line = match item {
            Err(_) => break,
            Ok(l) => l,
        };
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let split_eq = string_split(&line, '=');
        if split_eq.len() != 2 || split_eq[0].is_empty() || split_eq[1].is_empty() {
            return Err(format!("Can't parse line {}", num));
        }
        let groups = string_split(&split_eq[1], ';');
        for g in groups {
            let e = res.entry(split_eq[0].to_string()).or_default();
            e.push(g);
        }
    }
    Ok(res)
}

fn group_name_prefix(group_name: &str) -> String {
    match group_name.split_once('-') {
        Some((lhs, _)) => lhs.to_string(),
        None => group_name.to_string(),
    }
}

#[derive(Parser)]
pub struct Cli {
    /// Seed for random number generator.
    #[arg(long)]
    seed: Option<u64>,

    /// Use bad (but cross platform) random number generator
    #[arg(long)]
    badrand: bool,

    /// Which boxes to include in the collection.
    #[arg(long, value_delimiter = ',')]
    boxes: Vec<String>,

    /// Which groups to include in the collection.
    #[arg(long, value_delimiter = ',')]
    groups: Vec<String>,

    /// Filename listing boxes and which groups they contain
    #[arg(long)]
    boxfile: Option<String>,

    /// Filename listing all cards.
    #[arg(long)]
    cardfile: Option<String>,

    /// Dump contents of collection and exit.
    #[arg(long)]
    list: bool,

    /// How many landscape cards to include (does not include artefacts etc).
    #[arg(long)]
    landscape_count: Option<u8>,

    /// Explain why cards were added.
    #[arg(long)]
    why: bool,

    /// Do not validate collection.
    #[arg(long)]
    no_validate: bool,

    /// Do not allow any of these cards.
    #[arg(long, value_delimiter = ',')]
    exclude: Vec<String>,

    /// This card must be in the selection.
    #[arg(long, value_delimiter = ',')]
    include: Vec<String>,

    /// Show info about selected cards.
    #[arg(long)]
    info: bool,

    /// Disable automatic adding reacts to attacks.
    #[arg(long)]
    no_attack_react: bool,

    /// Disable automatic adding of trash cards if cards give curses.
    #[arg(long)]
    no_anti_cursor: bool,

    /// Set the maximum number of times a cost can occur.
    #[arg(long, default_value_t = 0)]
    max_cost_repeat: u8,

    /// eg --min-type=Treasure:5 means that the selection will can contain at least 5 treasures."));
    #[arg(long, value_delimiter = ',')]
    min_type: Vec<String>,

    /// eg --max-type=Treasure:5 means that the selection will can contain at most 5 treasures."));
    #[arg(long, value_delimiter = ',')]
    max_type: Vec<String>,

    /// Most prefixes (groups and related groups) which can be included. Eg: Cornucopia would also allow Cornucopia-prizes.
    #[arg(long, default_value_t = 0)]
    max_prefixes: u8,
}

fn main() {
    let cli = Cli::parse();

    let mut conf = match load_config(cli, "cards.dat".to_string(), "".to_string()) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", e);
            exit(1);
        }
    };

    // Need to create the state separately, list, validate and sort

    let mut col = CardCollectionPtr::new_state(&conf.piles);
    if conf.validate {
        let warnings = match col.validate_collection() {
            CollectionStatus::CollOK => {
                vec![]
            }
            CollectionStatus::CollWarning(v) => v,
            CollectionStatus::_CollFatal(v) => v,
        };
        if !warnings.is_empty() {
            println!("Error validating collection:");
            for s in warnings {
                println!("{}", s);
            }
            exit(3);
        };
    };
    if conf.list_collection {
        for p in &conf.piles {
            println!("{}", p.get_name());
        }
        exit(0);
    };
    col.shuffle(&mut conf.rand);
    let col = CardCollectionPtr::from_state(col);
    let constraints = match conf.build_constraints(&col) {
        Ok(v) => v,
        Err(s) => {
            println!("{}", s);
            exit(4);
        }
    };
    let sel = match col.generate_selection(
        10,
        conf.optional_extras,
        &conf.includes,
        &constraints,
        &mut conf.rand,
    ) {
        Ok(s) => s,
        Err(m) => {
            eprintln!("Error: empty selection");
            eprintln!("Possible explanation: {}", m);
            exit(2);
        }
    };
    // TODO: Have to come up with another way to do this, as I'm removing get-string
    // println!("Options:{}", conf.get_string());
    sel.dump(conf.why, conf.more_info);
}
