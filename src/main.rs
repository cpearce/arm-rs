extern crate argparse;
extern crate rayon;

mod index;
mod itemizer;
mod transaction_reader;
mod fptree;
mod generate_rules;
mod command_line_args;

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
use command_line_args::Arguments;
use command_line_args::parse_args_or_exit;

use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::Write;
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

fn mine_fp_growth(args: &Arguments) -> Result<(), Box<Error>> {
    println!("Mining data set: {}", args.input_file_path);
    println!("Making first pass of dataset to count item frequencies...");
    // Make one pass of the dataset to calculate the item frequencies
    // for the initial tree.
    let start = Instant::now();
    let timer = Instant::now();
    let mut itemizer: Itemizer = Itemizer::new();
    let item_count = count_item_frequencies(
        TransactionReader::new(&args.input_file_path, &mut itemizer),
    ).unwrap();
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
    println!(
        "Building initial FPTree took {} seconds.",
        timer.elapsed().as_secs()
    );

    println!("Starting recursive FPGrowth...");
    let timer = Instant::now();
    let min_count = (args.min_support * (fptree.num_transactions() as f64)) as u32;
    let patterns: Vec<Vec<u32>> = fp_growth(&fptree, min_count, &vec![], &itemizer);
    println!(
        "FPGrowth generated {} frequent itemsets in {} seconds.",
        patterns.len(),
        timer.elapsed().as_secs()
    );

    println!("Generating rules...");
    let timer = Instant::now();
    let rules = generate_rules(&patterns, args.min_confidence, args.min_lift, &index);
    println!(
        "Generated {} rules in {} seconds.",
        rules.len(),
        timer.elapsed().as_secs()
    );

    {
        let mut output = File::create(&args.output_rules_path)?;
        writeln!(output, "Antecedent->Consequent,Confidence,Lift,Support")?;
        for rule in rules {
            writeln!(
                output,
                "{},{},{},{}",
                rule.to_string(&itemizer),
                confidence(&rule, &index),
                lift(&rule, &index),
                index.support(&rule.merge())
            )?;
        }
    }

    println!("Total runtime: {} seconds", start.elapsed().as_secs());

    Ok(())
}

fn main() {
    let arguments = parse_args_or_exit();

    if let Err(err) = mine_fp_growth(&arguments) {
        println!("Error: {}", err);
        process::exit(1);
    }
}
