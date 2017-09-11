extern crate rayon;
extern crate itertools;

mod index;
mod itemizer;
mod transaction_reader;
mod fptree;
mod generate_rules;

use itemizer::Itemizer;
use transaction_reader::TransactionReader;
use fptree::FPTree;
use fptree::sort_transaction;
use fptree::fp_growth;
use fptree::SortOrder;
use generate_rules::confidence;
use generate_rules::lift;
use generate_rules::generate_rules;
use index::Index;

use itertools::sorted;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::process;
use std::time::Instant;

fn count_item_frequencies(reader: TransactionReader) -> Result<HashMap<u32, u32>, Box<Error>> {
    let mut item_count: HashMap<u32, u32> = HashMap::new();
    for transaction in reader {
        for item in transaction {
            let counter = item_count.entry(item).or_insert(0);
            *counter += 1;
        }
    }
    Ok(item_count)
}

pub struct Arguments {
    input_file_path: String,
    output_rules_path: String,
    min_support: f64,
    min_confidence: f64,
    min_lift: f64,
}

impl Arguments {
    fn new(input_file_path: String,
        output_rules_path: String,
        min_support: f64,
        min_confidence: f64,
        min_lift: f64) -> Arguments
    {
        Arguments{input_file_path: input_file_path,
                output_rules_path: output_rules_path,
                min_support: min_support,
                min_confidence: min_confidence,
                min_lift: min_lift}
    }
}

fn mine_fp_growth(args: &Arguments) -> Result<(), Box<Error>> {
    println!("Mining data set: {}", args.input_file_path);
    println!("Making first pass of dataset to count item frequencies...");
    // Make one pass of the dataset to calculate the item frequencies
    // for the initial tree.
    let start = Instant::now();
    let timer = Instant::now();
    let mut itemizer: Itemizer = Itemizer::new();
    let item_count = count_item_frequencies(
        TransactionReader::new(&args.input_file_path, &mut itemizer)).unwrap();
    println!("First pass took {} seconds.", timer.elapsed().as_secs());

    println!("Building initial FPTree based on item frequencies...");

    // Load the initial tree, by re-reading the data set and inserting
    // each transaction into the tree sorted by item frequency.
    let timer = Instant::now();
    let mut index = Index::new();
    let mut fptree = FPTree::new();
    for mut transaction in TransactionReader::new(&args.input_file_path, &mut itemizer) {
        sort_transaction(&mut transaction, &item_count, SortOrder::Decreasing);
        fptree.insert(&transaction, 1);
        index.insert(&transaction);
    }
    println!("Building initial FPTree took {} seconds.", timer.elapsed().as_secs());

    println!("Starting recursive FPGrowth...");
    let timer = Instant::now();
    let min_count = (args.min_support * (fptree.num_transactions() as f64)) as u32;
    let patterns: Vec<Vec<u32>> = fp_growth(&fptree, min_count, &vec![], &itemizer);
    println!("FPGrowth generated {} frequent itemsets in {} seconds.", patterns.len(), timer.elapsed().as_secs());

    println!("Generating rules...");
    let timer = Instant::now();
    let rules = generate_rules(&patterns, args.min_confidence, args.min_lift, &index);
    println!("Generated {} rules in {} seconds.", rules.len(), timer.elapsed().as_secs());

    {
        let mut output = File::create(&args.output_rules_path)?;
        for rule in rules {
            writeln!(output, "{}, {}, {}, {}",
                rule.to_string(&itemizer),
                confidence(&rule, &index),
                lift(&rule, &index),
                index.support(&rule.merge()))?;
        }
    }

    println!("Total runtime: {} seconds", start.elapsed().as_secs());

    Ok(())
}

fn print_usage() {
    println!("Usage:");
    println!("");
    println!("arm input_csv_file_path output_rules_csv_file_path min_support min_confidence min_lift");
    println!("");
    println!("  input_csv_file_path: path to transaction data set in CSV format,");
    println!("      one transaction per line.");
    println!("  output_rules_csv_file_path: path to file to write out rules.");
    println!("      Format: antecedent -> consequent, confidence, lift, support");
    println!("  min_support: minimum support threshold. Floating point value in");
    println!("      the range [0,1].");
    println!("  min_confidence: minimum confidence threshold. Floating point value in");
    println!("      the range [0,1].");
    println!("  min_lift: minimum lift threshold. Floating point value in");
    println!("      the range [1,∞].");
}

fn parse_args() -> Result<Arguments, String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 6 {
        print_usage();
        return Err(String::from("Invalid number of args."));
    }

    let path = Path::new(&args[1]);
    if !path.exists() || !path.is_file() {
        return Err(String::from("Input file doesn't exist or is not file."));
    }
    let path = Path::new(&args[2]);
    if path.exists() {
        return Err(String::from("Output rules file already exists; refusing to overwrite!"));
    }

    let min_support: f64 = match args[3].parse::<f64>() {
        Ok(f) => f,
        Err(e) => return Err(String::from("Failed to parse min_support: ") + &e.to_string())
    };
    if min_support < 0.0 || min_support > 1.0 {
        return Err(String::from("Minimum support must be in range [0,1]"));
    }

    let min_confidence: f64 = match args[4].parse::<f64>() {
        Ok(f) => f,
        Err(e) => return Err(String::from("Failed to parse min_confidence: ") + &e.to_string())
    };
    if min_confidence < 0.0 || min_confidence > 1.0 {
        return Err(String::from("Minimum support must be in range [0,1]"));
    }

    let min_lift: f64 = match args[5].parse::<f64>() {
        Ok(f) => f,
        Err(e) => return Err(String::from("Failed to parse min_lift: ") + &e.to_string())
    };
    if min_lift < 1.0 {
        return Err(String::from("Minimum lift must be in range [1,∞]"));
    }

    let input = args[1].clone();
    let rules = args[2].clone();
    Ok(Arguments::new(input, rules, min_support, min_confidence, min_lift))
}

fn main() {

    let arguments = match parse_args() {
        Ok(x) => x,
        Err(e) => {
            println!("Error: {}", e);
            process::exit(1);
        },
    };

    if let Err(err) = mine_fp_growth(&arguments) {
        println!("Error: {}", err);
        process::exit(1);
    }
}
