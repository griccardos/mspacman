use std::{
    collections::{HashMap, HashSet},
    io::Write,
    process::Command,
};

use crate::{
    error::AppError,
    structs::{
        appstate::AppState, event::EventCommand, package::Package, packageupdate::PackageUpdate,
        reason::Reason,
    },
    update_tables,
    utils::natural_cmp,
    version::Version,
};

pub fn refresh_packages_and_update_tables(state: &mut AppState) -> Result<(), AppError> {
    //run these in parallel
    let jh1 = std::thread::spawn(get_installed_packages);
    let jh2 = std::thread::spawn(get_all_packages);
    let jh3 = std::thread::spawn(get_updates);
    let jh4 = std::thread::spawn(get_provides);

    //now join threads
    let installed = jh1.join().expect("Thread error")?;
    let all = jh2.join().expect("Thread error")?;
    let updates = jh3.join().expect("Thread error")?;
    let provides = jh4.join().expect("Thread error")?;

    state.packages = combine_packages(installed, all, updates, provides);

    update_tables(state);
    Ok(())
}

pub fn run_command(state: &mut AppState, command: EventCommand) -> Result<(), AppError> {
    let (comm, mut args, needs_package_list, packs) = match command {
        EventCommand::RemoveSelected(packs) => ("pacman", vec!["-R"], true, packs),
        EventCommand::InstallOrUpdateSelected(packs) => ("pacman", vec!["-S"], true, packs),
        EventCommand::QuerySelected(packs) => ("pacman", vec!["-Qi"], true, packs),
        EventCommand::SyncDatabase => ("pacman", vec!["-Sy"], false, vec![]),
        EventCommand::SyncAndUpdateAll => ("pacman", vec!["-Syu"], false, vec![]),
    };
    if needs_package_list && packs.is_empty() {
        return Err(String::from("No packages selected").into());
    }

    args.extend(packs.iter().map(|a| a.as_str()));

    std::io::stdout()
        .write_all(format!("\nRunning command: {} {}\n", comm, args.join(" ")).as_bytes())?;
    //try run command as is
    let res = Command::new(comm).args(&args).status()?;
    let mut ret = Ok(());
    if !res.success() {
        std::io::stdout().write_all("running sudo\n".as_bytes())?;
        //run as sudo
        args.insert(0, comm);
        args.insert(0, "-S"); //for sudo
        let res = Command::new("sudo").args(&args).status()?;
        if !res.success() {
            std::io::stdout().write_all("Failed to run command".as_bytes())?;
            ret = Err(String::from("Failed to run command").into());
        }
    }
    std::io::stdout().write_all("\nPress enter to continue...".as_bytes())?;
    std::io::stdout().flush()?;
    crossterm::event::read()?;

    refresh_packages_and_update_tables(state)?;

    ret
}

pub fn combine_packages(
    installed: Vec<Package>,
    all: Vec<Package>,
    updates: Vec<PackageUpdate>,
    provides: HashMap<String, Vec<String>>,
) -> Vec<Package> {
    //start with installed, this may include those not in repo
    let installed_names = installed
        .iter()
        .map(|p| p.name.clone())
        .collect::<HashSet<_>>();
    let mut combined = installed;
    //we now add all local packages not installed
    for pack in all.iter() {
        if !installed_names.contains(&pack.name) {
            combined.push(pack.clone());
        }
    }

    //we add update info
    for pack in updates.iter() {
        if let Some(p) = combined.iter_mut().find(|p| p.name == pack.name) {
            p.new_version = Some(pack.new_version.clone());
            p.change_type = Some(pack.change_type.clone());
        }
    }
    combined.sort_by(|a, b| natural_cmp(&a.name, &b.name));

    //add files provided
    for pack in combined.iter_mut() {
        if let Some(prs) = provides.get(&pack.name) {
            pack.provides = prs.clone();
        }
    }

    combined
}

pub fn get_provides() -> Result<HashMap<String, Vec<String>>, AppError> {
    let output = Command::new("pacman").arg("-Ql").output()?;
    let output = String::from_utf8(output.stdout)?;
    let vals = output
        .lines()
        .filter_map(|line| {
            if let Some((pack, path)) = line.split_once(" ") {
                Some((pack.to_string(), path.to_string()))
            } else {
                None
            }
        })
        .collect::<Vec<(String, String)>>();
    let dict = vals.into_iter().fold(
        HashMap::<String, Vec<String>>::new(),
        |mut acc, (pack, path)| {
            acc.entry(pack).or_default().push(path);
            acc
        },
    );
    Ok(dict)
}

pub fn pacman_exists() -> bool {
    Command::new("pacman").output().is_ok()
}

pub fn get_packages_command(command: &str) -> Result<Vec<Package>, AppError> {
    let output = Command::new("pacman")
        .env("LC_TIME", "C")
        .arg(command)
        .output()?;
    let output = String::from_utf8(output.stdout)?;
    let mut packs: Vec<Package> = vec![];
    let mut pack = Package::default();

    for line in output.lines() {
        //if listing provides:
        if !pack.name.is_empty() && line.starts_with(&pack.name) {
            if let Some((_, pr)) = line.split_once(" ") {
                pack.provides.push(pr.to_string());
            }
            continue;
        }

        let pair = line.split_once(':');
        if pair.is_none() {
            continue;
        }
        let (key, value) = pair.unwrap();
        let key = key.trim();
        let value = value.trim();
        match key {
            "Name" => {
                if !pack.name.is_empty() {
                    packs.push(pack);
                    pack = Package::default();
                }
                pack.name = value.to_string()
            }
            "Version" => pack.version = value.to_string(),
            "Depends On" => {
                pack.dependencies = value
                    .split_whitespace()
                    .map(|r| r.to_string())
                    .filter(|r| r != "None")
                    .collect()
            }
            "Required By" => {
                pack.required_by = value
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .filter(|r| r != "None")
                    .collect()
            }
            "Optional For" => {
                pack.optional_for = value
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .filter(|r| r != "None")
                    .collect()
            }
            "Install Reason" => {
                pack.reason = match value {
                    "Explicitly installed" => Reason::Explicit,
                    "Installed as a dependency for another package" => Reason::Dependency,
                    // _ => value.to_string(),
                    _ => Reason::Other(value.to_string()),
                }
            }
            "Install Date" => pack.installed = Some(to_date(value)),
            "Description" => pack.description = value.to_string(),
            "Validated By" => pack.validated = value == "Signature",
            _ => {}
        }
    }
    packs.push(pack);

    Ok(packs)
}

pub fn get_all_packages() -> Result<Vec<Package>, AppError> {
    let packs = get_packages_command("-Si")?;
    Ok(packs)
}

pub fn get_installed_packages() -> Result<Vec<Package>, AppError> {
    get_packages_command("-Qi")
}

pub fn get_updates() -> Result<Vec<PackageUpdate>, AppError> {
    let output = Command::new("pacman")
        .env("LC_TIME", "C")
        .arg("-Qu")
        .output()?;
    let output = String::from_utf8(output.stdout)?;
    let mut updates: Vec<PackageUpdate> = vec![];
    for line in output.lines() {
        let line = line.replace(" -> ", " ");
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() == 3 {
            let current = Version::from(parts[1]);
            let new = Version::from(parts[2]);
            updates.push(PackageUpdate {
                name: parts[0].to_string(),
                current_version: parts[1].to_string(),
                new_version: parts[2].to_string(),
                change_type: current.change_type(&new),
            });
        }
    }

    Ok(updates)
}

pub fn to_date(value: &str) -> String {
    //get rid of the timezone
    let time = match jiff::fmt::strtime::parse("%a %b %e %H:%M:%S %Y", value) {
        Ok(time) => time,
        Err(e) => panic!("Could not parse '{value}': {e}"),
    };
    time.to_datetime().unwrap().to_string().replace("T", " ")
}
