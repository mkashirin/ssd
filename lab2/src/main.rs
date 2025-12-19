use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use md5::{Digest as Md5Digest, Md5};
use rayon::prelude::*;
use sha2::Sha256;
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const PASSWORD_LENGTH: usize = 5;
const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    file: Option<String>,

    #[arg(short, long, default_value_t = 4)]
    threads: usize,
}

enum HashType {
    Md5,
    Sha256,
}

fn check_password(
    password: &[u8],
    targets: &HashMap<String, HashType>,
) -> Option<(String, String)> {
    let password_str = std::str::from_utf8(password).unwrap();

    let mut md5_hasher = Md5::new();
    md5_hasher.update(password);
    let md5_hash = hex::encode(md5_hasher.finalize());
    if targets.contains_key(&md5_hash) {
        return Some((md5_hash, password_str.to_string()));
    }

    let mut sha256_hasher = Sha256::new();
    sha256_hasher.update(password);
    let sha256_hash = hex::encode(sha256_hasher.finalize());
    if targets.contains_key(&sha256_hash) {
        return Some((sha256_hash, password_str.to_string()));
    }

    None
}

fn brute_force_single_threaded(
    targets: &HashMap<String, HashType>,
) -> HashMap<String, String> {
    let mut found = HashMap::new();
    let mut password = [0u8; PASSWORD_LENGTH];

    let total_combinations = CHARSET.len().pow(PASSWORD_LENGTH as u32);
    let pb = ProgressBar::new(total_combinations as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})").unwrap()
        .progress_chars("#>-"));

    for c1 in CHARSET {
        password[0] = *c1;
        for c2 in CHARSET {
            password[1] = *c2;
            for c3 in CHARSET {
                password[2] = *c3;
                for c4 in CHARSET {
                    password[3] = *c4;
                    for c5 in CHARSET {
                        password[4] = *c5;
                        pb.inc(1);

                        if let Some((hash, pass)) =
                            check_password(&password, targets)
                        {
                            println!(
                                "\n[Single-threaded] Found: {} -> {}",
                                hash, pass
                            );
                            found.insert(hash, pass);
                            if found.len() == targets.len() {
                                pb.finish_with_message("All passwords found.");
                                return found;
                            }
                        }
                    }
                }
            }
        }
    }
    pb.finish_with_message("Search complete.");
    found
}

fn brute_force_multi_threaded(
    targets: &HashMap<String, HashType>,
    num_threads: usize,
) -> HashMap<String, String> {
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
        .unwrap();

    let found_map = Arc::new(Mutex::new(HashMap::new()));
    let keep_running = Arc::new(AtomicBool::new(true));

    let total_combinations = CHARSET.len().pow(PASSWORD_LENGTH as u32) as u64;
    let pb = ProgressBar::new(total_combinations);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})").unwrap()
        .progress_chars("#>-"));

    (0..total_combinations).into_par_iter().for_each(|i| {
        if !keep_running.load(Ordering::Relaxed) {
            return;
        }

        let mut password = [0u8; PASSWORD_LENGTH];
        let mut current_index = i;
        for j in (0..PASSWORD_LENGTH).rev() {
            password[j] =
                CHARSET[(current_index % CHARSET.len() as u64) as usize];
            current_index /= CHARSET.len() as u64;
        }

        pb.inc(1);

        if let Some((hash, pass)) = check_password(&password, targets) {
            let mut found = found_map.lock().unwrap();
            #[allow(clippy::map_entry)]
            if !found.contains_key(&hash) {
                println!("\n[Multi-threaded] Found: {} -> {}", hash, pass);
                found.insert(hash, pass);
                if found.len() == targets.len() {
                    keep_running.store(false, Ordering::Relaxed);
                }
            }
        }
    });

    pb.finish_with_message("Search complete.");
    found_map.lock().unwrap().clone()
}

fn main() {
    let args = Args::parse();
    let mut targets = HashMap::new();

    if let Some(file_path) = args.file {
        println!("Reading hashes from '{}'...", file_path);
        let file = fs::File::open(file_path).expect("Failed to open file.");
        for line in io::BufReader::new(file).lines() {
            add_target(&mut targets, line.expect("Failed to read line."));
        }
    } else {
        println!("Enter hash values, one per line (press Ctrl+D or Ctrl+Z to finish):");
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            add_target(&mut targets, line.expect("Failed to read line."));
        }
    }

    if targets.is_empty() {
        println!("No hash values provided. Exiting.");
        return;
    }

    println!("\nTargets to crack ({}):", targets.len());
    for hash in targets.keys() {
        println!("- {}", hash);
    }

    println!("\n--- Starting Single-Threaded Brute Force ---");
    let start_single = Instant::now();
    let results_single = brute_force_single_threaded(&targets);
    let duration_single = start_single.elapsed();
    println!("--- Single-threaded search finished. ---");
    print_results(&results_single);
    println!("Time elapsed: {:.2?}", duration_single);

    println!(
        "\n--- Starting Multi-Threaded Brute Force ({} threads) ---",
        args.threads
    );
    let start_multi = Instant::now();
    let results_multi = brute_force_multi_threaded(&targets, args.threads);
    let duration_multi = start_multi.elapsed();
    println!("--- Multi-threaded search finished. ---");
    print_results(&results_multi);
    println!("Time elapsed: {:.2?}", duration_multi);

    // --- Comparison ---
    println!("--- RESULTS: ---");
    println!("Single-Threaded Time: {:.2?}", duration_single);
    println!(
        "Multi-Threaded Time:  {:.2?} ({} threads)",
        duration_multi, args.threads
    );
    if duration_multi < duration_single {
        let speedup =
            duration_single.as_secs_f64() / duration_multi.as_secs_f64();
        println!("Multi-threading was {:.2}x faster.", speedup);
    } else {
        println!("Multi-threading was not faster in this case (likely due to overhead).");
    }
}

fn add_target(targets: &mut HashMap<String, HashType>, hash: String) {
    let hash = hash.trim().to_lowercase();
    if hash.len() == 32 {
        targets.insert(hash, HashType::Md5);
    } else if hash.len() == 64 {
        targets.insert(hash, HashType::Sha256);
    } else {
        eprintln!("Warning: Skipping invalid or unsupported hash '{}'", hash);
    }
}

fn print_results(results: &HashMap<String, String>) {
    if results.is_empty() {
        println!("No passwords were found.");
    } else {
        println!("Found Passwords:");
        for (hash, password) in results {
            println!("  - {} -> {}", hash, password);
        }
    }
}
