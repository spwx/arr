mod error;
mod find_file;
mod parse_command;
mod parse_yaml;
mod util;

pub use util::{get_all_executors, parse_all};

use error::ArrError;
use find_file::{find_atomics_dir, find_file};
use parse_command::{parse_command, update_path};
use parse_yaml::{parse_art_file, AtomicReadTeamTechnique};

use log::{error, info};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Arr {
    technique: String,
    vars: HashMap<String, String>,
    test_num: usize,
    art_path: PathBuf,
}

impl Arr {
    pub fn new(
        technique: String,
        vars: HashMap<String, String>,
        test_num: usize,
        art_path: PathBuf,
    ) -> Self {
        Self {
            technique,
            vars,
            test_num,
            art_path,
        }
    }

    pub fn run(&self) -> Result<(), ArrError> {
        // find the `atomics` directory
        let atomics_dir = find_atomics_dir(&self.art_path)?;

        // find the YAML file
        let art_file = find_file(&self.technique, &atomics_dir)?;

        // parse the YAML
        let yaml = parse_art_file(&art_file)?;

        // verify the chosen test works with this OS
        is_os_supported(&yaml, self.test_num)?;

        // check super user privileges
        if cfg!(unix) {
            check_superuser_requirement(&yaml, self.test_num)?;
        }

        // combine default and provided variables
        let args = gather_args(&yaml, self.vars.clone(), self.test_num, &atomics_dir);

        // run the check
        let check_command = get_check_command(&yaml, self.test_num, &atomics_dir, &args)?;
        for (command, executor) in check_command {
            execute(&command, &executor)?;
        }

        // run the dependency
        let dependency_command = get_dependency_command(&yaml, self.test_num, &atomics_dir, &args)?;
        for (command, executor) in dependency_command {
            execute(&command, &executor)?;
        }

        // run the attack
        let (attack_command, attack_executor) =
            get_attack_command(&yaml, self.test_num, &atomics_dir, &args)?;

        execute(&attack_command, &attack_executor)?;

        Ok(())
    }

    pub fn cleanup(&self) -> Result<(), ArrError> {
        // find the `atomics` directory
        let atomics_dir = find_atomics_dir(&self.art_path)?;

        // find the YAML file
        let art_file = find_file(&self.technique, &atomics_dir)?;

        // parse the YAML
        let yaml = parse_art_file(&art_file)?;

        // verify the chosen test works with this OS
        is_os_supported(&yaml, self.test_num)?;

        // check super user privileges
        if cfg!(unix) {
            check_superuser_requirement(&yaml, self.test_num)?;
        }

        // combine default and provided variables
        let args = gather_args(&yaml, self.vars.clone(), self.test_num, &atomics_dir);

        // run the cleanup
        let (cleanup_command, cleanup_executor) =
            get_cleanup_command(&yaml, self.test_num, &atomics_dir, &args)?;

        if cleanup_command.is_empty() {
            error!("This test does not have a cleanup command");
            return Err(ArrError::Other("No cleanup command".to_string()));
        }

        execute(&cleanup_command, &cleanup_executor)?;

        Ok(())
    }
}

fn is_os_supported(yaml: &AtomicReadTeamTechnique, test_num: usize) -> Result<(), ArrError> {
    let local_os = std::env::consts::OS;

    let res = yaml.atomic_tests[test_num]
        .supported_platforms
        .iter()
        .any(|os| os.to_lowercase().trim().eq(local_os));

    match res {
        true => {
            info!(
                "Technique: {}, supports {}",
                &yaml.attack_technique, &local_os
            );
            Ok(())
        }
        false => {
            error!(
                "{} not supported for Technique: {}, Test: {}.",
                local_os, &yaml.attack_technique, test_num
            );
            Err(ArrError::OsNotSupported)
        }
    }
}

#[cfg(target_family = "unix")]
fn check_superuser_requirement(
    yaml: &AtomicReadTeamTechnique,
    test_num: usize,
) -> Result<(), ArrError> {
    if let Some(er) = &yaml.atomic_tests[test_num].executor.elevation_required {
        if *er && !nix::unistd::getuid().is_root() {
            error!(
                "Technique {} test {} requires root.",
                &yaml.attack_technique, test_num
            );
            return Err(ArrError::RootRequired);
        }
    }
    info!("Required user permissions met");
    Ok(())
}

fn gather_args(
    yaml: &AtomicReadTeamTechnique,
    vars: HashMap<String, String>,
    test_num: usize,
    atomics_dir: &Path,
) -> HashMap<String, String> {
    // get the default args
    let mut args: HashMap<String, String> = yaml.atomic_tests[test_num]
        .input_arguments
        .iter()
        .map(|(k, v)| (k.clone(), v.default.clone()))
        .collect();

    // replace defaults with user's args
    args.extend(vars);

    // parse the args to update the "PathToAtomics"
    let mut parsed_args = HashMap::new();

    for (k, v) in args.into_iter() {
        let (_, v) = update_path(&atomics_dir.to_string_lossy())(&v).unwrap();

        info!("Set variable `{}` to `{}`", &k, &v);

        parsed_args.insert(k, v);
    }

    parsed_args
}

fn get_check_command(
    yaml: &AtomicReadTeamTechnique,
    test_num: usize,
    atomics_dir: &Path,
    vars: &HashMap<String, String>,
) -> Result<Vec<(String, String)>, ArrError> {
    let mut commands: Vec<(String, String)> = Vec::new();
    if let Some(dependency_executor) = &yaml.atomic_tests[test_num].dependency_executor_name {
        if let Some(dependencies) = &yaml.atomic_tests[test_num].dependencies {
            for dependency in dependencies {
                let command = parse_commands(&dependency.prereq_command, atomics_dir, vars)?;
                commands.push((command, dependency_executor.to_string()));
            }
        }
    }

    for (command, executor) in &commands {
        info!("The check executor is `{}`", executor);
        info!("The check command is: `{}`", command);
    }

    Ok(commands)
}

fn get_dependency_command(
    yaml: &AtomicReadTeamTechnique,
    test_num: usize,
    atomics_dir: &Path,
    vars: &HashMap<String, String>,
) -> Result<Vec<(String, String)>, ArrError> {
    let mut commands: Vec<(String, String)> = Vec::new();
    if let Some(dependency_executor) = &yaml.atomic_tests[test_num].dependency_executor_name {
        if let Some(dependencies) = &yaml.atomic_tests[test_num].dependencies {
            for dependency in dependencies {
                let command = parse_commands(&dependency.get_prereq_command, atomics_dir, vars)?;
                commands.push((command, dependency_executor.to_string()));
            }
        }
    }

    for (command, executor) in &commands {
        info!("The dependency executor is `{}`", executor);
        info!("The dependency command is: `{}`", command);
    }

    Ok(commands)
}

fn get_attack_command(
    yaml: &AtomicReadTeamTechnique,
    test_num: usize,
    atomics_dir: &Path,
    vars: &HashMap<String, String>,
) -> Result<(String, String), ArrError> {
    let command = yaml.atomic_tests[test_num]
        .executor
        .command
        .clone()
        .unwrap_or_else(|| "".to_string());
    let executor = yaml.atomic_tests[test_num].executor.name.to_string();
    let command = parse_commands(&command, atomics_dir, vars)?;

    info!("The attack executor is `{}`", &executor);
    info!("The attack command is `{}`", &command);

    Ok((command, executor))
}

fn get_cleanup_command(
    yaml: &AtomicReadTeamTechnique,
    test_num: usize,
    atomics_dir: &Path,
    vars: &HashMap<String, String>,
) -> Result<(String, String), ArrError> {
    let command = yaml.atomic_tests[test_num]
        .executor
        .cleanup_command
        .clone()
        .unwrap_or("".to_string());
    let executor = yaml.atomic_tests[test_num].executor.name.to_string();
    let command = parse_commands(&command, atomics_dir, vars)?;

    info!("The cleanup executor is `{}`", &executor);
    info!("The cleanup command is `{}`", &command);

    Ok((command, executor))
}

fn parse_commands(
    commands: &str,
    atomics_dir: &Path,
    vars: &HashMap<String, String>,
) -> Result<String, ArrError> {
    let parsed_commands = commands
        .lines()
        .map(|command| parse_command(command, atomics_dir, vars))
        .collect::<Result<Vec<_>, _>>()?;

    let commands = parsed_commands.join(";");

    Ok(commands)
}

fn execute(command: &str, executor: &str) -> Result<(), ArrError> {
    log::info!("Using `{}` to execute the command: {}", &executor, &command);

    let executor_arg = if executor.eq("cmd") { "/c" } else { "-c" };

    let output = Command::new(executor)
        .arg(executor_arg)
        .arg(&command)
        .output()
        .map_err(|e| ArrError::CommandIoFailure(e.to_string()))?;

    match output.status.success() {
        true => {
            info!("Command executed with a successful return code");
            Ok(())
        }
        false => {
            let mut stdout = String::from_utf8_lossy(&output.stdout);
            let mut stderr = String::from_utf8_lossy(&output.stderr);

            stdout = if stdout.len() > 0 {
                stdout
            } else {
                std::borrow::Cow::Borrowed("(None)")
            };

            stderr = if stderr.len() > 0 {
                stderr
            } else {
                std::borrow::Cow::Borrowed("(None)")
            };

            error!("Unsuccessful return code from the command: `{}`", &command);
            error!("STDOUT: {}", stdout);
            error!("STDERR: {}", stderr);
            Err(ArrError::CommandExecutionFailed)
        }
    }
}
