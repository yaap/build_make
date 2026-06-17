/*
 * Copyright (C) 2025 The Android Open Source Project
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

//! This module verifies the content of api-versions.xml and checks it for internal consistency.
mod api_versions;
use crate::api_versions::{load, Api, MajorMinorVersion};
use clap::Parser;
use itertools::Itertools;
use std::{fs, path::PathBuf};

use anyhow::{bail, ensure, Context, Result};
use std::collections::{HashMap, HashSet};

#[derive(Parser, Debug)]
struct Args {
    /// Path to api-versions.xml
    #[arg(short, long)]
    api_versions_path: PathBuf,

    #[arg(short, long)]
    deprecated_at_birth_allowlist_path: Option<PathBuf>,

    #[arg(long)]
    max_sdk_version_full: MajorMinorVersion,
}

fn main() -> Result<()> {
    let args = Args::parse();

    ensure!(
        args.api_versions_path.exists(),
        "api_versions_path does not exist: {:?}",
        args.api_versions_path
    );

    if let Some(path) = &args.deprecated_at_birth_allowlist_path {
        ensure!(path.exists(), "deprecated_at_birth_allowlist_path does not exist: {:?}", path);
    }

    let api = load(&args.api_versions_path).with_context(|| {
        format!("Failed to load api versions from {:?}", args.api_versions_path)
    })?;

    let allowlist: HashSet<String> = match &args.deprecated_at_birth_allowlist_path {
        Some(path) => fs::read_to_string(path)
            .with_context(|| format!("Failed to read allowlist file at {:?}", path))?
            .lines()
            .map(|line| line.to_string())
            .collect(),
        None => HashSet::new(),
    };

    let mut problems = Vec::<String>::new();
    problems.append(&mut no_deprecated_at_birth(&api, &allowlist));
    problems.append(&mut no_adservices_ext_crossover(&api));
    problems.append(&mut no_deprecated_after_last_known(&api, args.max_sdk_version_full));

    assert!(problems.is_empty(), "{}", problems.iter().join("\n"));

    Ok(())
}

fn no_deprecated_at_birth(api: &Api, allowlist: &HashSet<String>) -> Vec<String> {
    let mut problems = Vec::<String>::new();

    for class in api.classes.values() {
        if let Some(deprecated) = &class.deprecated {
            if deprecated == &class.since {
                let allowlist_key = format!("{} {}", &class.name, deprecated);
                if !allowlist.contains(allowlist_key.as_str()) {
                    problems.push(allowlist_key);
                }
            }
        }
        for field in class.fields.values() {
            if let Some(deprecated) = &field.deprecated {
                let since = field.since.as_ref().unwrap_or(&class.since);

                if deprecated == since {
                    let allowlist_key = format!("{}#{} {}", &class.name, &field.name, deprecated);
                    if !allowlist.contains(allowlist_key.as_str()) {
                        problems.push(allowlist_key);
                    }
                }
            }
        }
        for method in class.methods.values() {
            if let Some(deprecated) = &method.deprecated {
                let since = method.since.as_ref().unwrap_or(&class.since);
                if deprecated == since {
                    let allowlist_key = format!("{}#{} {}", &class.name, &method.name, deprecated);
                    if !allowlist.contains(allowlist_key.as_str()) {
                        problems.push(allowlist_key);
                    }
                }
            }
        }
    }

    problems
}

/// Checks if the provided optional version exceeds the maximum allowed SDK version.
///
/// Returns `Some(String)` with a formatted error message if the version is present and
/// greater than `max_sdk_version`. Otherwise, returns `None`.
fn check_version_msg(
    version_opt: Option<MajorMinorVersion>,
    max_sdk_version: MajorMinorVersion,
    context: String,
) -> Option<String> {
    if let Some(version) = version_opt {
        if version > max_sdk_version {
            return Some(format!(
                "{}: SDK version \"{}\" is greater than the maximum allowed \"{}\"",
                context, version, max_sdk_version
            ));
        }
    }
    None
}

fn no_deprecated_after_last_known(api: &Api, max_sdk_version: MajorMinorVersion) -> Vec<String> {
    let mut problems: Vec<String> = Vec::new();

    for class in api.classes.values() {
        problems.extend(check_version_msg(
            Some(class.since),
            max_sdk_version,
            format!("Class '{}', attribute 'since'", class.name),
        ));

        problems.extend(check_version_msg(
            class.deprecated,
            max_sdk_version,
            format!("Class '{}', attribute 'deprecated'", class.name),
        ));

        for field in class.fields.values() {
            let field_id = format!("Class '{}', field '{}'", class.name, field.name);
            problems.extend(check_version_msg(
                field.since,
                max_sdk_version,
                format!("{}, attribute 'since'", field_id),
            ));
            problems.extend(check_version_msg(
                field.deprecated,
                max_sdk_version,
                format!("{}, attribute 'deprecated'", field_id),
            ));
        }

        for method in class.methods.values() {
            let method_id = format!("Class '{}', method '{}'", class.name, method.name);
            problems.extend(check_version_msg(
                method.since,
                max_sdk_version,
                format!("{}, attribute 'since'", method_id),
            ));
            problems.extend(check_version_msg(
                method.deprecated,
                max_sdk_version,
                format!("{}, attribute 'deprecated'", method_id),
            ));
        }
    }

    problems
}

fn no_adservices_ext_crossover(api: &Api) -> Vec<String> {
    let mut problems = Vec::<String>::new();

    for class in api.classes.values() {
        if let Some(sdks) = &class.sdks {
            if let Err(error) = validate_sdk_string(sdks) {
                problems.push(error.to_string())
            };
        }
        for field in class.fields.values() {
            if let Some(sdks) = &field.sdks {
                if let Err(error) = validate_sdk_string(sdks) {
                    problems.push(error.to_string())
                };
            }
        }
        for method in class.methods.values() {
            if let Some(sdks) = &method.sdks {
                if let Err(error) = validate_sdk_string(sdks) {
                    problems.push(error.to_string())
                };
            }
        }
    }
    problems
}

fn validate_sdk_string(sdks: &String) -> Result<String> {
    let mut sdk_map = HashMap::new();
    for sdk in sdks.split(",") {
        let mut s = sdk.split(":");
        let extension = s.next().unwrap();
        let version = s.next().unwrap();
        assert!(s.next().is_none(), "Malformed sdks value: {sdks}");
        assert!(
            sdk_map.insert(extension, version).is_none(),
            "Extension {extension} already in map"
        );
    }
    if sdk_map.contains_key("1000000") {
        sdk_map.remove("0");
        if sdk_map.len() != 1 {
            bail!(format!("Found extra extension value in addition to adservices: {}", &sdks));
        }
    }
    Ok("".to_string())
}
