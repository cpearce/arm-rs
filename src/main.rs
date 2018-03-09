extern crate argparse;
extern crate itertools;
extern crate ordered_float;
extern crate rayon;

mod index;
mod item;
mod item_counter;
mod itemizer;
mod transaction_reader;
mod fptree;
mod generate_rules;
mod command_line_args;
mod rule;

use itemizer::Itemizer;
use item::Item;
use item_counter::ItemCounter;
use transaction_reader::TransactionReader;
use fptree::FPTree;
use fptree::fp_growth;
use fptree::ItemSet;
use generate_rules::generate_rules;
use rule::Rule;
use index::Index;
use command_line_args::Arguments;
use command_line_args::parse_args_or_exit;
use std::error::Error;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::process;
use std::time::{Duration, Instant};

fn count_item_frequencies(reader: TransactionReader) -> Result<(ItemCounter, usize), Box<Error>> {
    let mut item_count: ItemCounter = ItemCounter::new();
    let mut num_transactions = 0;
    for transaction in reader {
        num_transactions += 1;
        for item in transaction.iter() {
            item_count.add(item, 1);
        }
    }
    Ok((item_count, num_transactions))
}

fn duration_as_ms(duration: &Duration) -> u64 {
    (duration.as_secs() * 1_000 as u64) + (duration.subsec_nanos() / 1_000_000) as u64
}

fn mine_fp_growth(args: &Arguments) -> Result<(), Box<Error>> {
    println!("Mining data set: {}", args.input_file_path);
    println!("Making first pass of dataset to count item frequencies...");
    // Make one pass of the dataset to calculate the item frequencies
    // for the initial tree.
    let start = Instant::now();
    let timer = Instant::now();
    let mut itemizer: Itemizer = Itemizer::new();
    let (item_count, num_transactions) =
        count_item_frequencies(TransactionReader::new(&args.input_file_path, &mut itemizer))
            .unwrap();
    println!(
        "First pass took {} ms, num_transactions={}.",
        duration_as_ms(&timer.elapsed()),
        num_transactions
    );

    println!("Building initial FPTree based on item frequencies...");

    // Load the initial tree, by re-reading the data set and inserting
    // each transaction into the tree sorted by item frequency.
    let timer = Instant::now();
    let mut index = Index::new();
    let mut fptree = FPTree::new();
    let min_count = (args.min_support * (num_transactions as f64)) as u32;
    for transaction in TransactionReader::new(&args.input_file_path, &mut itemizer) {
        // Strip out infrequent items from the transaction. This can
        // drastically reduce the tree size, and speed up loading the
        // initial tree.
        let mut filtered_transaction: Vec<Item> = Vec::new();
        for item in transaction {
            if item_count.get(&item) > min_count {
                filtered_transaction.push(item);
            }
        }
        // Note: we deliberately insert even if the transaction is empty to
        // ensure the index increments its transaction count. Otherwise the
        // support counts will be wrong in the rule generation phase.
        index.insert(&filtered_transaction);
        if filtered_transaction.is_empty() {
            continue;
        }
        item_count.sort_descending(&mut filtered_transaction);
        fptree.insert(&filtered_transaction, 1);
    }
    println!(
        "Building initial FPTree took {} ms.",
        duration_as_ms(&timer.elapsed())
    );

    println!("Starting recursive FPGrowth...");
    let timer = Instant::now();
    let patterns: Vec<ItemSet> = fp_growth(
        &fptree,
        min_count,
        &vec![],
        num_transactions as u32,
        &itemizer,
    );

    println!(
        "FPGrowth generated {} frequent itemsets in {} ms.",
        patterns.len(),
        duration_as_ms(&timer.elapsed())
    );

    println!("Generating rules...");
    let timer = Instant::now();
    let rules: Vec<Rule> = generate_rules(
        &patterns,
        num_transactions as u32,
        args.min_confidence,
        args.min_lift,
    ).iter()
        .cloned()
        .collect();
    println!(
        "Generated {} rules in {} ms, writing to disk.",
        rules.len(),
        duration_as_ms(&timer.elapsed())
    );

    let timer = Instant::now();
    {
        let mut output = BufWriter::new(File::create(&args.output_rules_path).unwrap());
        writeln!(
            output,
            "Antecedent => Consequent, Confidence, Lift, Support"
        )?;
        for rule in rules {
            writeln!(
                output,
                "{}, {}, {}, {}",
                rule.to_string(&itemizer),
                rule.confidence(),
                rule.lift(),
                rule.support(),
            )?;
        }
    }
    let file_size = std::fs::metadata(&args.output_rules_path)?.len();
    let elapsed_ms = duration_as_ms(&timer.elapsed());
    println!(
        "Wrote rules to disk in {} ms into file of {} bytes; {:.1} MB/s.",
        elapsed_ms,
        file_size,
        (file_size as f64 / (elapsed_ms as f64 / 1000.0)) / 1_000_000.0
    );

    println!("Total runtime: {} ms", duration_as_ms(&start.elapsed()));

    Ok(())
}

fn main() {
    let arguments = parse_args_or_exit();

    if let Err(err) = mine_fp_growth(&arguments) {
        println!("Error: {}", err);
        process::exit(1);
    }
}
