//   Copyright 2018 Chris Pearce
//
//   Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
//    You may obtain a copy of the License at
//
//        http://www.apache.org/licenses/LICENSE-2.0
//
//    Unless required by applicable law or agreed to in writing, software
//    distributed under the License is distributed on an "AS IS" BASIS,
//    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//    See the License for the specific language governing permissions and
//    limitations under the License.

extern crate argparse;
extern crate fnv;
extern crate itertools;
extern crate rayon;

mod index;
mod item;
mod vec_sets;
mod item_counter;
mod itemizer;
mod transaction_reader;
mod fptree;
mod generate_rules;
mod command_line_args;
mod rule;

use rule::RuleSet;
use itemizer::Itemizer;
use item::Item;
use item_counter::ItemCounter;
use transaction_reader::TransactionReader;
use fptree::{FPTree, fp_growth, ItemSet};
use generate_rules::generate_rules;
use command_line_args::{Arguments, parse_args_or_exit};
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
    let (mut item_count, num_transactions) =
        count_item_frequencies(TransactionReader::new(&args.input_file_path, &mut itemizer))
            .unwrap();
    println!(
        "First pass took {} ms, num_transactions={}.",
        duration_as_ms(&timer.elapsed()),
        num_transactions
    );

    println!("Reordering itemizer lexicographically...");
    let timer = Instant::now();
    itemizer.reorder_sorted(&mut item_count);
    println!(
        "Reordered itemizer in {} ms.",
        duration_as_ms(&timer.elapsed())
    );

    println!("Building initial FPTree based on item frequencies...");

    // Load the initial tree, by re-reading the data set and inserting
    // each transaction into the tree sorted by item frequency.
    let timer = Instant::now();
    let mut fptree = FPTree::new();
    let min_count = (args.min_support * (num_transactions as f64)).ceil() as u32;
    for transaction in TransactionReader::new(&args.input_file_path, &mut itemizer) {
        // Strip out infrequent items from the transaction. This can
        // drastically reduce the tree size, and speed up loading the
        // initial tree.
        let mut filtered_transaction = transaction
            .into_iter()
            .filter(|&item| item_count.get(&item) > min_count)
            .collect::<Vec<Item>>();
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
    let rules = generate_rules(
        &patterns,
        num_transactions as u32,
        args.min_confidence,
        args.min_lift,
    );
    println!(
        "Generated {} rules in {} ms, writing to disk.",
        rules.len(),
        duration_as_ms(&timer.elapsed())
    );

    let timer = Instant::now();
    write_rules(&rules, &args.output_rules_path, &itemizer)?;
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

fn write_rules(
    rules: &RuleSet,
    output_rules_path: &str,
    itemizer: &Itemizer,
) -> Result<(), Box<Error>> {
    let mut output = BufWriter::new(File::create(output_rules_path)?);
    writeln!(output, "Antecedent => Consequent,Confidence,Lift,Support")?;
    for rule in rules {
        write_item_slice(&mut output, &rule.antecedent, &itemizer)?;
        write!(output, " => ")?;
        write_item_slice(&mut output, &rule.consequent, &itemizer)?;
        writeln!(
            output,
            ",{},{},{}",
            rule.confidence(),
            rule.lift(),
            rule.support(),
        )?;
    }
    Ok(())
}

fn write_item_slice(
    output: &mut BufWriter<File>,
    items: &[Item],
    itemizer: &Itemizer,
) -> Result<(), Box<Error>> {
    let mut first = true;
    for item in items.iter().map(|&id| itemizer.str_of(id)) {
        if !first {
            write!(output, " ")?;
        } else {
            first = false;
        }
        output.write(item.as_bytes())?;
    }
    Ok(())
}

fn main() {
    let arguments = parse_args_or_exit();

    if let Err(err) = mine_fp_growth(&arguments) {
        println!("Error: {}", err);
        process::exit(1);
    }
}
