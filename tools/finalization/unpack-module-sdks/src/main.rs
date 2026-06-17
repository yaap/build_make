/*
 * Copyright (C) 2026 The Android Open Source Project
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

//! `unpack-module-sdks` is a tool to copy selected parts of the artifacts from
//! mainline_modules_sdks.sh to prebuilts/sdk and prebuilts/module_sdk/$module as part of
//! finalizing a new SDK extension version

// TODO(b/481981256): refactor code to more readable

use ::zip::ZipArchive;
use anyhow::{anyhow, Context, Result};
use clap::Parser;
use glob::glob;
use regex::Regex;
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io,
    path::{Path, PathBuf},
};

mod mainline_modules_info;
mod test;
mod zip;

const ABOUT: &str = "Tool to extract some of the artifacts of mainline_modules_sdks.sh into prebuilts/sdk and prebuilts/module_sdks/$module.";

#[derive(Parser, Debug)]
#[clap(about=ABOUT)]
struct Cli {
    #[arg(long)]
    mainline_sdks_top: PathBuf,

    #[arg(long)]
    android_top: PathBuf,

    #[arg(long)]
    sdk_ext_version: usize,
}

#[derive(Debug, Default)]
struct Operations {
    zip_archives: HashMap<PathBuf, HashSet<(String, String)>>,
    projects: HashSet<String>,
}

fn decide_what_to_unpack<P: AsRef<Path>>(
    mainline_sdks_top: P,
    android_top: P,
    sdk_ext_version: usize,
) -> Result<Operations> {
    let mainline_sdks_top = mainline_sdks_top.as_ref();
    let android_top = android_top.as_ref();
    let mut ops =
        Operations { projects: HashSet::from(["prebuilts/sdk".to_string()]), ..Default::default() };

    let module_info_path = mainline_sdks_top.join("mainline-modules-info.json");
    let file =
        File::open(&module_info_path).with_context(|| format!("open {module_info_path:?}"))?;
    let module_info = mainline_modules_info::parse_mainline_module_info_json(file)?;

    let mut modified_projects: Vec<String> = vec![];
    let regex = Regex::new(r"/mainline-sdks/for-next-build/current/(.*?)/sdk/.*\.zip").unwrap();
    for zip_path in glob(&format!(
        "{}/mainline-sdks/for-next-build/current/*/sdk/*.zip",
        mainline_sdks_top.display()
    ))? {
        let zip_path = zip_path.context("glob")?;
        let zip_str = zip_path.to_string_lossy();
        let caps = regex
            .captures(&zip_str)
            .ok_or_else(|| anyhow!("regex does not match string {zip_str:?}"))?;
        let project = module_info.get(&caps[1]).ok_or_else(|| {
            anyhow!(
                "{} found in module SDK dir but not listed in mainline-modules-info.json",
                &caps[1]
            )
        })?;
        modified_projects.push(project.to_owned());

        let zip_file = File::open(&zip_path)?;

        let dest_dir = android_top.join(project).join(sdk_ext_version.to_string());
        let mut unpack_ops: HashSet<(String, String)> = zip::process_module_sdk(&zip_file)?
            .into_iter()
            .map(|(src, dest)| (src, dest_dir.join(dest).to_string_lossy().to_string()))
            .collect();

        let dest_dir =
            android_top.join("prebuilts/sdk/extensions").join(sdk_ext_version.to_string());
        unpack_ops.extend(
            zip::process_platform_sdk_extensions(&zip_file)?
                .into_iter()
                .map(|(src, dest)| (src, dest_dir.join(dest).to_string_lossy().to_string())),
        );

        ops.zip_archives.insert(zip_path, unpack_ops);
        ops.projects.insert(project.into());
    }

    Ok(ops)
}

fn main() -> Result<()> {
    let args = Cli::parse();

    let ops = decide_what_to_unpack(
        &args.mainline_sdks_top,
        &args.android_top,
        args.sdk_ext_version
        ).with_context(|| {
        format!(
            "unpacking mainline sdks from directory {} to Android tree at {} with SDK extension version {}",
            args.mainline_sdks_top.display(),
            args.android_top.display(),
            args.sdk_ext_version
        )
    })?;

    for (zip_path, unpack_ops) in ops.zip_archives {
        let zip_file = File::open(&zip_path)?;
        let mut zip_archive = ZipArchive::new(zip_file)?;
        for (src, dest) in unpack_ops {
            let mut zip_entry = zip_archive.by_name(&src)?;

            let dest_dir = PathBuf::from(&dest);
            let dest_dir = dest_dir
                .parent()
                .ok_or_else(|| anyhow!("failed to get parent dir of path {}", &dest))?;
            fs::create_dir_all(dest_dir)?;

            let mut dest_file = fs::File::create(&dest)?;
            io::copy(&mut zip_entry, &mut dest_file)?;
        }
    }

    let mut modified_projects: Vec<_> = ops.projects.into_iter().collect();
    modified_projects.sort_unstable();
    println!("{}", modified_projects.join("\n"));

    Ok(())
}
