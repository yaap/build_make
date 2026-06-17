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

//! `finalize-non-api-flags` is a tool to create a snapshot of exception (that
//! is, exported non-API) flags that have "made it" to a given release. Since they're
//! used cross-binary, this binary supports automatically adding an SDK level
//! check so the flags are resilient to deletion. This means that once this
//! binary is run, the flags have passed a "point of no return" where they
//! cannot be rolled back.
use anyhow::{Context, Result};
use clap::Parser;
use std::{fs::File, io::Read, path::PathBuf};

use aconfig_protos::{parsed_flags, ProtoParsedFlags};

mod exception_flags;
mod load_from_cache;

pub(crate) type FlagId = String;

const ABOUT: &str = "Creates a temp file of 'finalized' exported non-API flags.

Ensures exported flags which don't guard APIs still get the generated SDK level
check so they can be safely cleaned up.

**Only flags on the exception list will be included. If an exported flag is not
on that list OR used to guard an API, it will not get the SDK level check, even if it's included in a
major platform release.**

This tool:

  - Reads the exception list from the source tree [--exception-list]
  - Reads the state of the current flags from release configs [--flag-sources]
  - Reads the state of the current flags from the aconfig cache [--aconfig-cache]
  - Filters the exception list to only include flags that are present in the
    release configs, recording the earliest relese config that has each flag
  - Prints the map of <flags, release config> to stdout
";

#[derive(Parser, Debug)]
#[clap(about=ABOUT)]
struct Cli {
    #[arg(long)]
    exception_list: Option<PathBuf>,

    #[arg(long)]
    cache: PathBuf,

    #[arg(long)]
    release_config: Option<String>,
}

pub(crate) fn load_protos_from_file<R: Read>(mut reader: R) -> Result<ProtoParsedFlags> {
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;
    parsed_flags::try_from_binary_proto(&buf)
}

fn main() -> Result<()> {
    let args = Cli::parse();

    // TODO(b/445461092): Replace hard-coded value with read of environment
    // variable TARGET_RELEASE and, if it's "next", real lookup of "next" alias.
    let mut release_config = args.release_config.unwrap_or("cp2a".to_string());
    if release_config == "next" {
        release_config = "cp2a".to_string();
    }

    // TODO(b/445461092): Investigate if this should be passed in from Soong
    // as an arg (all the time).
    let exception_list_path = args.exception_list.unwrap_or(PathBuf::from(
        "build/make/tools/aconfig/exported_flag_check/non_api_flags_list.txt",
    ));

    let file = File::open(exception_list_path).with_context(|| "open {exception_list_path}")?;
    let all_exception_flags = exception_flags::read_exception_flags(file)?;

    let cache_file = File::open(args.cache).with_context(|| "open {args.cache}")?;
    let parsed_flags = load_protos_from_file(cache_file)?;
    let finalized_flags = load_from_cache::extract_flags_from_cache(parsed_flags, &release_config);

    let finalized_exception_flags: Vec<&FlagId> =
        finalized_flags.intersection(&all_exception_flags).collect();

    for flag in finalized_exception_flags {
        println!("{}", flag);
    }
    Ok(())
}
