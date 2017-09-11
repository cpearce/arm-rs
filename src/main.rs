extern crate argparse;
extern crate rayon;

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

use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{self, Write};
use std::process;
use std::time::Instant;
use argparse::{ArgumentParser, Store};

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

pub struct Arguments {
    input_file_path: String,
    output_rules_path: String,
    min_support: f64,
    min_confidence: f64,
    min_lift: f64,
}

fn parse_args_or_exit() -> Arguments {
    let mut args: Arguments = Arguments{
        input_file_path: String::new(),
        output_rules_path: String::new(),
        min_support: 0.0,
        min_confidence: 0.0,
        min_lift: 0.0
    };

    {
        let mut parser = ArgumentParser::new();
        parser.set_description("Light weight parallel FPGrowth in Rust.");

        parser.refer(&mut args.input_file_path)
            .add_option(&["--input"], Store, "Input dataset in CSV format.")
            .metavar("file_path")
            .required();

        parser.refer(&mut args.output_rules_path)
            .add_option(&["--output"], Store, "File path in which to store output rules. Format: antecedent -> consequent, confidence, lift, support.")
            .metavar("file_path")
            .required();

        parser.refer(&mut args.min_support)
            .add_option(&["--min-support"], Store, "Minimum itemset support threshold, in range [0,1].")
            .metavar("threshold")
            .required();

        parser.refer(&mut args.min_confidence)
            .add_option(&["--min-confidence"], Store, "Minimum rule confidence threshold, in range [0,1].")
            .metavar("threshold")
            .required();

        parser.refer(&mut args.min_lift)
            .add_option(&["--min-lift"], Store, "Minimum rule lift confidence threshold, in range [1,∞].")
            .metavar("threshold");

        if env::args().count() == 1 {
            parser.print_help("Usage:", &mut io::stderr()).unwrap();
            process::exit(1);
        }
 
        match parser.parse_args() {
            Ok(()) =>  {}
            Err(err) => {
                process::exit(err);
            }
        }
    }    

    if args.min_support < 0.0 || args.min_support > 1.0 {
        eprintln!("Minimum itemset support must be in range [0,1]");
        process::exit(1);
    }

    if args.min_confidence < 0.0 || args.min_confidence > 1.0 {
        eprintln!("Minimum rule confidence threshold must be in range [0,1]");
        process::exit(1);
    }

    if args.min_lift < 1.0 {
        eprintln!("Minimum lift must be in range [1,∞]");
        process::exit(1);
    }

    args
}

fn main() {
    let arguments = parse_args_or_exit();

    if let Err(err) = mine_fp_growth(&arguments) {
        println!("Error: {}", err);
        process::exit(1);
    }
}
