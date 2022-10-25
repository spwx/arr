use std::{collections::HashMap, path::Path};

use crate::{find_file::all_techniques, parse_yaml::parse_art_file, ArrError};

pub fn parse_all(art_path: &Path) {
    for f in all_techniques(art_path) {
        print!(
            "Attempting to parse: {}...\t",
            f.file_name().to_string_lossy()
        );
        let res = parse_art_file(&f.into_path());

        match res {
            Ok(_) => println!("Success!"),
            Err(_) => {
                return;
            }
        }
    }
}

pub fn get_all_executors(art_path: &Path) -> Result<(), ArrError> {
    let mut executor_count: HashMap<String, usize> = HashMap::new();
    let mut dep_executor_count: HashMap<String, usize> = HashMap::new();

    for f in all_techniques(art_path) {
        let technique = parse_art_file(&f.into_path())?;

        for art_test in technique.atomic_tests {
            let count = executor_count.entry(art_test.executor.name).or_insert(0);
            *count += 1;

            if let Some(dep_executor) = art_test.dependency_executor_name {
                let count = dep_executor_count.entry(dep_executor).or_insert(0);
                *count += 1;
            }
        }
    }

    println!("Executors:");
    for (key, value) in executor_count.iter() {
        println!("{}: {}", key, value);
    }

    println!("\nDependency Executors:");
    for (key, value) in dep_executor_count.iter() {
        println!("{}: {}", key, value);
    }

    Ok(())
}

// pub fn run_all(art_path: &Path) {
//     for f in all_techniques(art_path) {
//         let technique = match parse_atr_file(&f.clone().into_path()) {
//             Ok(technique) => technique,
//             Err(e) => {
//                 eprintln!("{}", e.to_string());
//                 return;
//             }
//         };

//         for (test_num, _) in technique.atomic_tests.iter().enumerate() {
//             println!(
//                 "Running: {}, Test: {}...\t",
//                 technique.attack_technique, test_num
//             );

//             let file_name = f
//                 .clone()
//                 .file_name()
//                 .to_string_lossy()
//                 .strip_suffix(".yaml")
//                 .unwrap()
//                 .to_string();

//             let mut arr = Arr::new(file_name, HashMap::new(), test_num, art_path.to_owned());

//             match arr.run() {
//                 Ok(_) => println!("Success!"),
//                 Err(e) => match e {
//                     ArrError::OsNotSupported(_) => continue,
//                     _ => {
//                         eprintln!("Failure:\n\n{}", e.to_string());
//                         return;
//                     }
//                 },
//             }
//         }
//     }
// }
