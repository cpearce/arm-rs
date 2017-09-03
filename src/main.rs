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

fn mine_fp_growth(input_csv_file_path: &str,
                  output_csv_file_path: &str,
                  min_support: f64) -> Result<(), Box<Error>> {
    println!("Mining data set: {}", input_csv_file_path);
    println!("Making first pass of dataset to count item frequencies...");
    // Make one pass of the dataset to calculate the item frequencies
    // for the initial tree.
    let start = Instant::now();
    let timer = Instant::now();
    let mut itemizer: Itemizer = Itemizer::new();
    let item_count = count_item_frequencies(
        TransactionReader::new(input_csv_file_path, &mut itemizer)).unwrap();
    println!("First pass took {} seconds.", timer.elapsed().as_secs());

    println!("Building initial FPTree based on item frequencies...");

    // Load the initial tree, by re-reading the data set and inserting
    // each transaction into the tree sorted by item frequency.
    let timer = Instant::now();
    let mut fptree = FPTree::new();
    for mut transaction in TransactionReader::new(input_csv_file_path, &mut itemizer) {
        sort_transaction(&mut transaction, &item_count, SortOrder::Decreasing);
        fptree.insert(&transaction, 1);
    }
    println!("Building initial FPTree took {} seconds.", timer.elapsed().as_secs());

    println!("Starting recursive FPGrowth...");
    let timer = Instant::now();
    let min_count = (min_support * (fptree.num_transactions() as f64)) as u32;
    let patterns: Vec<Vec<u32>> = fp_growth(&fptree, min_count, &vec![], &itemizer);
    println!("FPGrowth took {} seconds.", timer.elapsed().as_secs());

    // Convert frequent itemsets from a vector of item ids to
    // vector of friendly string item names.
    let item_id_to_string = |x: &u32| -> String { itemizer.str_of(*x) };
    let u32_vec_to_string_vec = |v: &Vec<u32>| -> Vec<String> {
        sorted(v.iter().map(&item_id_to_string))
    };

    // Output the itemsets.
    let timer = Instant::now();
    {
        println!("FPGrowth complete, generated {} frequent itemsets.", patterns.len());
        println!("Writing frequent itemsets to output file...");
        let mut output = File::create(output_csv_file_path)?;
        for itemset in patterns.iter().map(u32_vec_to_string_vec) {
            writeln!(output, "{}", itemset.join(","))?;
        }
    }
    println!("Writing frequent itemsets took {} seconds", timer.elapsed().as_secs());

    println!("Total runtime: {} seconds", start.elapsed().as_secs());

    Ok(())
}

fn print_usage() {
    println!("Usage:");
    println!("");
    println!("arm input_csv_file_path output_csv_file_path min_support");
    println!("");
    println!("  input_csv_file_path: path to transaction data set in CSV format,");
    println!("      one transaction per line.");
    println!("  output_csv_file_path: path to file to write out frequent item sets.");
    println!("      Itemsets written in CSV format.");
    println!("  min_support: minimum support threshold. Floating point value in");
    println!("      the range [0,1].");
}

fn parse_args() -> Result<(String, String, f64), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        print_usage();
        return Err(String::from("Invalid number of args."));
    }

    let path = Path::new(&args[1]);
    if !path.exists() || !path.is_file() {
        return Err(String::from("Input file doesn't exist or is not file."));
    }
    let path = Path::new(&args[2]);
    if path.exists() {
        return Err(String::from("Output file already exists; refusing to overwrite!"));
    }

    let min_support: f64 = match args[3].parse::<f64>() {
        Ok(f) => f,
        Err(e) => return Err(String::from("Failed to parse min_support: ") + &e.to_string())
    };
    if min_support < 0.0 || min_support > 1.0 {
        return Err(String::from("Minimum support must be in range [0,1]"));
    }

    let input = args[1].clone();
    let output = args[2].clone();
    Ok((input, output, min_support))
}

fn main() {

    let (input, output, min_support) = match parse_args() {
        Ok(x) => x,
        Err(e) => {
            println!("Error: {}", e);
            process::exit(1);
        },
    };

    if let Err(err) = mine_fp_growth(&input, &output, min_support) {
        println!("Error: {}", err);
        process::exit(1);
    }
}
