use snarkvm::prelude::*;
use snarkvm::console::account::{PrivateKey, Address};
use rand::rngs::OsRng;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::env;
use std::collections::HashMap;

fn generate_keypair(rng: &mut OsRng) -> (PrivateKey<Testnet3>, Address<Testnet3>) {
    let private_key = PrivateKey::<Testnet3>::new(rng).unwrap();
    let address = Address::try_from(&private_key).unwrap();
    (private_key, address)
}

fn chi_square_test(counts: &HashMap<String, usize>, sample_size: usize) -> f64 {
    let expected_count = sample_size as f64 / counts.len() as f64;
    let mut chi_square = 0.0;

    for count in counts.values() {
        let count = *count as f64;
        chi_square += (count - expected_count).powi(2) / expected_count;
    }

    chi_square
}

fn main() {
    // Read the desired suffix and sample size from the command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <desired_suffix> <sample_size>", args[0]);
        std::process::exit(1);
    }
    let desired_suffix = &args[1];
    let sample_size: usize = args[2].parse().unwrap();

    let guess_count = Arc::new(AtomicUsize::new(0));
    let found = Arc::new(AtomicUsize::new(0));
    let address_counts = Arc::new(std::sync::Mutex::new(HashMap::new()));

    rayon::scope(|s| {
        for _ in 0..num_cpus::get() {
            let guess_count = Arc::clone(&guess_count);
            let found = Arc::clone(&found);
            let address_counts = Arc::clone(&address_counts);
            let desired_suffix = desired_suffix.clone();

            s.spawn(move |_| {
                let mut rng = OsRng;
                while found.load(Ordering::Relaxed) < sample_size {
                    let (private_key, address) = generate_keypair(&mut rng);
                    guess_count.fetch_add(1, Ordering::Relaxed);

                    {
                        let mut counts = address_counts.lock().unwrap();
                        let entry = counts.entry(address.to_string()).or_insert(0);
                        *entry += 1;
                    }

                    if guess_count.load(Ordering::Relaxed) % 100_000 == 0 {
                        println!("Number of guesses: {}", guess_count.load(Ordering::Relaxed));
                    }

                    if address.to_string().ends_with(&desired_suffix) {
                        // We found an address with the desired suffix, print it
                        println!("Found address: {}", address.to_string());
                        println!("Found private key: {}", private_key.to_string());
                        found.fetch_add(1, Ordering::Relaxed);
                    }
                }
            });
        }
    });

    let address_counts = address_counts.lock().unwrap();
    let chi_square = chi_square_test(&address_counts, sample_size);
    let degrees_of_freedom = address_counts.len() - 1;
    let p_value = chi2_p_value(chi_square, degrees_of_freedom);

    println!("Sample collection completed.");
    println!("Number of guesses: {}", guess_count.load(Ordering::Relaxed));
    println!("Chi-square: {}", chi_square);
    println!("Degrees of freedom: {}", degrees_of_freedom);
    println!("P-value: {}", p_value);
}

fn chi2_p_value(chi_square: f64, degrees_of_freedom: usize) -> f64 {
    use statrs::distribution::{ChiSquared, ContinuousCDF};

    let chi2 = ChiSquared::new(degrees_of_freedom as f64).unwrap();
    1.0 - chi2.cdf(chi_square)
}