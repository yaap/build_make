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
//! CLI program to convert Mainline Beta namespace configs from JSON to static Rust data structures.

use anyhow::Result;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use tinytemplate::TinyTemplate;

#[derive(Deserialize, Serialize)]
struct Namespace {
    container: String,
    allow_exported: bool,
}

#[derive(Deserialize, Serialize)]
struct Config {
    namespaces: HashMap<String, Namespace>,
}

impl Config {
    fn from_json_file(filename: &String) -> Result<Config> {
        Self::from_json(&fs::read_to_string(filename)?)
    }

    fn from_json(json: &str) -> Result<Config> {
        Ok(serde_json::from_str(json)?)
    }

    fn to_rust(&self) -> Result<String> {
        #[derive(Serialize)]
        struct Context<'a> {
            namespaces: Vec<(&'a String, &'a Namespace)>,
        }
        // TinyTemplate can't iterate over HashMap entries, so do that iteration for it here.
        let mut context = Context { namespaces: self.namespaces.iter().collect() };
        context.namespaces.sort_by(|a, b| a.0.cmp(b.0));

        let mut template = TinyTemplate::new();
        let template_str = include_str!("../templates/mainline_beta_namespace_config.rs.template");
        template.add_template("rust", template_str)?;
        Ok(template.render("rust", &context)?)
    }

    fn to_rust_file(&self, filename: &String) -> Result<()> {
        fs::write(filename, self.to_rust()?.as_bytes())?;
        Ok(())
    }
}

#[derive(Parser, Debug)]
#[clap(bin_name = "convert_mainline_beta_namespace_config")]
struct Cli {
    #[arg(long = "json-file")]
    json_filename: String,
    #[arg(long = "rust-file")]
    rust_filename: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    Config::from_json_file(&cli.json_filename)?.to_rust_file(&cli.rust_filename)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_json_invalid() {
        assert!(Config::from_json("").is_err());
        assert!(Config::from_json("{}").is_err());
    }

    #[test]
    fn test_from_json_empty() {
        let config = Config::from_json("{\"namespaces\": {}}");
        assert!(config.is_ok());
        let config = config.unwrap();
        assert!(config.namespaces.is_empty());
    }

    #[test]
    fn test_from_json_valid() {
        let config = Config::from_json(
            "{
            \"namespaces\": {
                \"test_namespace_unexported\": {
                    \"container\": \"test_mainline_container_unexported\",
                    \"allow_exported\": false
                },
                \"test_namespace_exported\": {
                    \"container\": \"test_mainline_container_exported\",
                    \"allow_exported\": true
                }
            }
        }",
        );
        assert!(config.is_ok());
        let config = config.unwrap();

        let unexported = config.namespaces.get("test_namespace_unexported");
        assert!(unexported.is_some());
        let unexported = unexported.unwrap();
        assert_eq!(unexported.container, "test_mainline_container_unexported");
        assert!(!unexported.allow_exported);

        let exported = config.namespaces.get("test_namespace_exported");
        assert!(exported.is_some());
        let exported = exported.unwrap();
        assert_eq!(exported.container, "test_mainline_container_exported");
        assert!(exported.allow_exported);
    }
}
