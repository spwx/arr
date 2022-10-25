mod error;
mod find_file;
mod parse_command;
mod parse_yaml;
mod util;

pub use util::{get_all_executors, parse_all};

use error::ArrError;
use find_file::find_file;
use parse_command::parse_command;
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
        // find the YAML file
        let art_file = find_file(&self.technique, &self.art_path)?;

        // parse the YAML
        let yaml = parse_art_file(&art_file)?;

        // verify the chosen test works with this OS
        is_os_supported(&yaml, self.test_num)?;

        // check super user privileges
        if cfg!(unix) {
            check_superuser_requirement(&yaml, self.test_num)?;
        }

        // combine default and provided variables
        let args = gather_args(&yaml, self.vars.clone(), self.test_num);

        // run the check
        let check_command = get_check_command(&yaml, self.test_num, &art_file, &args)?;
        for (command, executor) in check_command {
            execute(&command, &executor)?;
        }

        // run the dependency
        let dependency_command = get_dependency_command(&yaml, self.test_num, &art_file, &args)?;
        for (command, executor) in dependency_command {
            execute(&command, &executor)?;
        }

        // run the attack
        let (attack_command, attack_executor) =
            get_attack_command(&yaml, self.test_num, &art_file, &args)?;

        execute(&attack_command, &attack_executor)?;

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
) -> HashMap<String, String> {
    let mut args: HashMap<String, String> = yaml.atomic_tests[test_num]
        .input_arguments
        .iter()
        .map(|(k, v)| (k.clone(), v.default.clone()))
        .collect();

    args.extend(vars);

    for (k, v) in args.iter() {
        info!("Set variable `{}` to `{}`", k, v);
    }

    args
}

fn get_check_command(
    yaml: &AtomicReadTeamTechnique,
    test_num: usize,
    art_path: &Path,
    vars: &HashMap<String, String>,
) -> Result<Vec<(String, String)>, ArrError> {
    let mut commands: Vec<(String, String)> = Vec::new();
    if let Some(dependency_executor) = &yaml.atomic_tests[test_num].dependency_executor_name {
        if let Some(dependencies) = &yaml.atomic_tests[test_num].dependencies {
            for dependency in dependencies {
                let command = parse_commands(&dependency.prereq_command, art_path, vars)?;
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
    art_path: &Path,
    vars: &HashMap<String, String>,
) -> Result<Vec<(String, String)>, ArrError> {
    let mut commands: Vec<(String, String)> = Vec::new();
    if let Some(dependency_executor) = &yaml.atomic_tests[test_num].dependency_executor_name {
        if let Some(dependencies) = &yaml.atomic_tests[test_num].dependencies {
            for dependency in dependencies {
                let command = parse_commands(&dependency.get_prereq_command, art_path, vars)?;
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
    art_path: &Path,
    vars: &HashMap<String, String>,
) -> Result<(String, String), ArrError> {
    let command = yaml.atomic_tests[test_num]
        .executor
        .command
        .clone()
        .unwrap_or_else(|| "".to_string());
    let executor = yaml.atomic_tests[test_num].executor.name.to_string();
    let command = parse_commands(&command, art_path, vars)?;

    info!("The attack executor is `{}`", &executor);
    info!("The attack command is `{}`", &command);

    Ok((command, executor))
}

fn parse_commands(
    commands: &str,
    art_path: &Path,
    vars: &HashMap<String, String>,
) -> Result<String, ArrError> {
    let parsed_commands = commands
        .lines()
        .map(|command| parse_command(command, art_path, vars))
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
        .map_err(ArrError::CommandIoFailure)?;

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
