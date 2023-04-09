use std::collections::HashMap;
use std::env;

use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;

use std::convert::TryFrom;

use std::process::exit;

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

fn split_once(s: &str, sep: char) -> Option<(&str, &str)> {
    for (pos, c) in s.char_indices() {
        if c == sep {
            if pos == 0 {
                return Some((&s[0..0], &s[sep.len_utf8()..]));
            }
            return Some((&s[0..pos], &s[pos + sep.len_utf8()..]));
        }
    }
    None
}

fn organise_args(
    args: &Vec<String>,
    legal_options: &HashMap<String, String>,
) -> Result<StringMultiMap, String> {
    let mut m = StringMultiMap::new();
    for s in args {
        let temp: &String = s;
        match split_once(s, '=') {
            None => {
                if !legal_options.contains_key(&temp.to_string()) {
                    return Result::Err(format!("Unknown option {}", temp));
                }
                m.insert(s.clone(), vec!["_".to_string()]);
            }
            Some((lhs, rhs)) => {
                let toks = rhs.split(',');
                if !legal_options.contains_key(lhs) {
                    return Err(format!("Unknown option {}", lhs));
                }
                let e = m.entry(lhs.to_string()).or_default();
                for t in toks {
                    e.push(t.to_string());
                }
            }
        }
    }
    Ok(m)
}

fn show_options(legal_options: &HashMap<String, String>) {
    println!("decker [options]");
    for (k, v) in legal_options {
        println!("  {}  {}", k, v);
    }
    println!();
    println!("Arguments which take further input are of the form --opt=a,b,c")
}

fn short_value(s: &str) -> i8 {
    s.parse::<i8>().unwrap_or(-1)
}

fn ushort_value(s: &str) -> u8 {
    s.parse::<u8>().unwrap_or(0)
}

fn bool_value(s: &str) -> bool {
    s == "Y" || s == "y"
}

fn unsigned_value(s: &str) -> u64 {
    s.parse::<u64>().unwrap_or(0)
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

fn caps(v: usize) -> i8 {
    match i8::try_from(v) {
        Ok(res) => res,
        Err(_) => i8::MAX - 1,
    }
}

fn capus(v: usize) -> u8 {
    match u8::try_from(v) {
        Ok(res) => res,
        Err(_) => u8::MAX - 1,
    }
}

fn capu(v: usize) -> u64 {
    match u64::try_from(v) {
        Ok(res) => res,
        Err(_) => u64::MAX - 1,
    }
}

fn group_name_prefix(group_name: &str) -> String {
    match split_once(group_name, '-') {
        None => group_name.to_string(),
        Some((lhs, _)) => lhs.to_string(),
    }
}

fn main() {
    let mut args: Vec<String> = env::args().collect();
    args.remove(0);
    let mut conf = match load_config(args, "cards.dat".to_string(), "".to_string()) {
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
    println!("Options:{}", conf.get_string());
    sel.dump(conf.why, conf.more_info);
}
