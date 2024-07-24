use std::fs;

use clap::Parser;
use rand::Rng;
use vdfr::KeyValueOptions;

#[derive(Debug, Parser)]
struct Args {
    #[clap(subcommand)]
    subcommand: Subcommand,
}

#[derive(Debug, Parser)]
enum Subcommand {
    /// Parse app info file
    #[clap(name = "app")]
    AppInfo {
        /// Path to the file
        file: std::path::PathBuf,
        /// Use legacy parser
        #[clap(short, long)]
        legacy: bool,
        /// Dump back the parsed data into JSON file, prefixed with app_
        #[clap(short, long)]
        redump: bool,
    },
    /// Parse package info file
    #[clap(name = "pkg")]
    PackageInfo {
        /// Path to the file
        file: std::path::PathBuf,
        /// Use legacy parser
        #[clap(short, long)]
        legacy: bool,
        /// Dump back the parsed data into JSON file, prefixed with pkg_
        #[clap(short, long)]
        redump: bool,
    },
    /// Parse key-values file
    #[clap(name = "kv")]
    KV {
        /// Path to the file
        file: std::path::PathBuf,
        /// Use legacy parser
        #[clap(short, long)]
        legacy: bool,
        /// Dump back the parsed data into JSON file, prefixed with kv_
        #[clap(short, long)]
        redump: bool,
    },
}

fn get_random_num(total: usize) -> usize {
    let mut rng = rand::thread_rng();
    rng.gen_range(0..total)
}

fn work_app_info(file: &std::path::PathBuf, legacy: bool, redump: bool) {
    let data = if legacy {
        let mut file = fs::File::open(file).unwrap();
        let time_it = std::time::Instant::now();
        let reader = vdfr::legacy_parser::parse_app_info(&mut file).unwrap();

        println!("Version: {}", reader.version);
        println!("Universe: {}", reader.universe);
        println!("Total apps: {}", reader.apps.len());
        println!("Time taken to parse: {:?}", time_it.elapsed());
        let total = reader.apps.values().count();
        let random_app_pos = get_random_num(total);
        let random_app = reader.apps.values().nth(random_app_pos).unwrap();
        println!("Random app: {:?}", random_app);
        reader
    } else {
        let data = fs::read(file).unwrap();

        let time_it = std::time::Instant::now();
        let reader = vdfr::parser::parse_app_info(&data).unwrap();
        println!("Version: {}", reader.version);
        println!("Universe: {}", reader.universe);
        println!("Total apps: {}", reader.apps.len());
        println!("Time taken to parse: {:?}", time_it.elapsed());
        let total = reader.apps.values().count();
        let random_app_pos = get_random_num(total);
        let random_app = reader.apps.values().nth(random_app_pos).unwrap();
        println!("Random app: {:?}", random_app);
        reader
    };

    if redump {
        let filename = file.file_stem().unwrap().to_str().unwrap();
        let output_path = file
            .parent()
            .unwrap()
            .join(format!("app_{}.json", filename));
        let time_it = std::time::Instant::now();
        let output_file = fs::File::create(&output_path).unwrap();
        vdfr::serde_json::to_writer_pretty(output_file, &data).unwrap();
        println!("Time taken to dump JSON: {:?}", time_it.elapsed());

        let output_path_redump = file
            .parent()
            .unwrap()
            .join(format!("app_{}_redump.vdf", filename));
        let time_it = std::time::Instant::now();
        let mut output_file_redump = fs::File::create(&output_path_redump).unwrap();
        vdfr::writer::write_app_info(&mut output_file_redump, &data).unwrap();
        println!("Time taken to redump VDF: {:?}", time_it.elapsed());
    }
}

fn work_pkg_info(file: &std::path::PathBuf, legacy: bool, redump: bool) {
    let data = if legacy {
        let mut file = fs::File::open(file).unwrap();
        let time_it = std::time::Instant::now();
        let reader = vdfr::legacy_parser::parse_package_info(&mut file).unwrap();

        println!("Version: {}", reader.version);
        println!("Total packages: {}", reader.packages.len());
        println!("Time taken to parse: {:?}", time_it.elapsed());
        let total = reader.packages.values().count();
        let random_pkg_pos = get_random_num(total);
        let random_pkg = reader.packages.values().nth(random_pkg_pos).unwrap();
        println!("Random package: {:?}", random_pkg);
        reader
    } else {
        let data = fs::read(file).unwrap();

        let time_it = std::time::Instant::now();
        let reader = vdfr::parser::parse_package_info(&data).unwrap();
        println!("Version: {}", reader.version);
        println!("Total packages: {}", reader.packages.len());
        println!("Time taken to parse: {:?}", time_it.elapsed());
        let total = reader.packages.values().count();
        let random_pkg_pos = get_random_num(total);
        let random_pkg = reader.packages.values().nth(random_pkg_pos).unwrap();
        println!("Random package: {:?}", random_pkg);
        reader
    };

    if redump {
        let filename = file.file_stem().unwrap().to_str().unwrap();
        let output_path = file
            .parent()
            .unwrap()
            .join(format!("pkg_{}.json", filename));
        let time_it = std::time::Instant::now();
        let output_file = fs::File::create(&output_path).unwrap();
        vdfr::serde_json::to_writer_pretty(output_file, &data).unwrap();
        println!("Time taken to dump JSON: {:?}", time_it.elapsed());

        let output_path_redump = file
            .parent()
            .unwrap()
            .join(format!("pkg_{}_redump.vdf", filename));
        let time_it = std::time::Instant::now();
        let mut output_file_redump = fs::File::create(&output_path_redump).unwrap();
        vdfr::writer::write_package_info(&mut output_file_redump, &data).unwrap();
        println!("Time taken to redump VDF: {:?}", time_it.elapsed());
    }
}

fn work_kv(file: &std::path::PathBuf, legacy: bool, redump: bool) {
    let data = if legacy {
        let mut file = fs::File::open(file).unwrap();
        let time_it = std::time::Instant::now();
        let reader =
            vdfr::legacy_parser::parse_keyvalues(&mut file, KeyValueOptions::default()).unwrap();

        println!("Total key-values: {}", reader.len());
        println!("Time taken to parse: {:?}", time_it.elapsed());
        reader
    } else {
        let data = fs::read(file).unwrap();

        let time_it = std::time::Instant::now();
        let reader = vdfr::parser::parse_keyvalues(&data).unwrap();
        println!("Total key-values: {}", reader.len());
        println!("Time taken to parse: {:?}", time_it.elapsed());
        reader
    };

    if redump {
        let filename = file.file_stem().unwrap().to_str().unwrap();
        let output_path = file.parent().unwrap().join(format!("kv_{}.json", filename));
        let time_it = std::time::Instant::now();
        let output_file = fs::File::create(&output_path).unwrap();
        vdfr::serde_json::to_writer_pretty(output_file, &data).unwrap();
        println!("Time taken to dump JSON: {:?}", time_it.elapsed());

        let output_path_redump = file
            .parent()
            .unwrap()
            .join(format!("kv_{}_redump.vdf", filename));
        let time_it = std::time::Instant::now();
        let mut output_file_redump = fs::File::create(&output_path_redump).unwrap();
        vdfr::writer::write_keyvalues(&mut output_file_redump, &data).unwrap();
        println!("Time taken to redump VDF: {:?}", time_it.elapsed());
    }
}

fn main() {
    let args = Args::parse();

    match args.subcommand {
        Subcommand::AppInfo {
            file,
            legacy,
            redump,
        } => work_app_info(&file, legacy, redump),
        Subcommand::PackageInfo {
            file,
            legacy,
            redump,
        } => work_pkg_info(&file, legacy, redump),
        Subcommand::KV {
            file,
            legacy,
            redump,
        } => work_kv(&file, legacy, redump),
    }
}
