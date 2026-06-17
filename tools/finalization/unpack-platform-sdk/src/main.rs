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

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use regex::Regex;
use std::{
    fs::{self, File},
    io::{self, Read, Seek, Write},
    os::unix::fs::symlink as unix_symlink,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;
use zip::{read::ZipFile, ZipArchive};

const ABOUT: &str = "Tool to extract some of the artifacts of an SDK build into prebuilts/sdk as part of platform finalization.";

#[derive(Parser, Debug)]
#[clap(about=ABOUT)]
struct Cli {
    #[arg(long)]
    dist_dir: PathBuf,

    #[arg(long)]
    android_top: PathBuf,

    #[arg(long)]
    version: String,
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Operation {
    // src is relative to out/dist
    // dest is relative to prebuilts/sdk/$version
    Copy { src: String, dest: String },

    // src is the zip entry path inside the SDK zip
    // dest is relative prebuilts/sdk/$version
    Extract { src: String, dest: String },

    // link_target is relative prebuilts/sdk
    // link_file is relative prebuilts/sdk
    Symlink { link_target: String, link_file: String },
}

fn copy<S: Into<String>, T: Into<String>>(src: S, dest: T) -> Operation {
    Operation::Copy { src: src.into(), dest: dest.into() }
}

fn extract<S: Into<String>, T: Into<String>>(src: S, dest: T) -> Operation {
    Operation::Extract { src: src.into(), dest: dest.into() }
}

fn symlink<S: Into<String>, T: Into<String>>(link_target: S, link_file: T) -> Operation {
    Operation::Symlink { link_target: link_target.into(), link_file: link_file.into() }
}

fn decide_what_to_do<R: Read + Seek>(
    sdk_files: &[String],
    sdk_zip_reader: R,
) -> Result<Vec<Operation>> {
    let mut ops = vec![];

    let re_api_txt = Regex::new(
        r#"^apistubs/android/(public|system|test|module-lib|system-server)/api/(.*\.txt)$"#,
    )
    .unwrap();
    let re_jar =
        Regex::new(r#"^apistubs/android/(public|system|test|module-lib|system-server)/(.*\.jar)$"#)
            .unwrap();
    let re_core_for_system_modules_jar =
        Regex::new(r#"^system-modules/(public|module-lib)/core-for-system-modules\.jar$"#).unwrap();
    let re_data = Regex::new(
        r#"^(|system|module-lib|system-server)-?data/(api-versions\.xml|annotations\.zip)$"#,
    )
    .unwrap();
    for path in sdk_files {
        // for scope in 'public', 'system', 'test', 'module-lib', 'system-server':
        //     apistubs/android/${scope}/api/*.txt -> ${scope}/api/*.txt
        if let Some(caps) = re_api_txt.captures(path) {
            ops.push(copy(&caps[0], format!("{}/api/{}", &caps[1], &caps[2])));
            continue;
        }

        // for scope in 'public', 'system', 'test', 'module-lib', 'system-server':
        //     apistubs/android/${scope}/api/*.jar -> ${scope}/*.jar
        if let Some(caps) = re_jar.captures(path) {
            ops.push(copy(&caps[0], format!("{}/{}", &caps[1], &caps[2])));
            continue;
        }

        // special case for system-modules/{public,module-lib}/core-for-system-modules.jar
        if let Some(caps) = re_core_for_system_modules_jar.captures(path) {
            ops.push(copy(&caps[0], format!("{}/core-for-system-modules.jar", &caps[1])));
            continue;
        }

        // for scope in 'public', 'system', 'module-lib', 'system-server':
        //     {data,system-data,module-lib-data,system-server-data}/{api-versions.xml,annotations.zip} ->
        //     ${scope}/data/{api-versions.xml,annotations.zip}
        if let Some(caps) = re_data.captures(path) {
            if caps[1].is_empty() {
                ops.push(copy(&caps[0], format!("public/data/{}", &caps[2])));
            } else {
                ops.push(copy(&caps[0], format!("{}/data/{}", &caps[1], &caps[2])));
            }
            continue;
        }

        if path == "finalized-flags.txt" {
            ops.push(copy(path, path));
            continue;
        }
    }

    // note: this will potentially overwrite some copied sdk_files so must be done after all
    // Operation::Copy
    let zip_archive = ZipArchive::new(sdk_zip_reader).context("parse input as zip archive")?;
    let re_zip =
        Regex::new(r#"^android-.*/(android\.jar|framework\.aidl|uiautomator\.jar)$"#).unwrap();
    for path in zip_archive.file_names() {
        // extract "android-.*/{android.jar,framework.aidl,uiautomator.jar}" -> public/*
        if let Some(caps) = re_zip.captures(path) {
            ops.push(extract(&caps[0], format!("public/{}", &caps[1])));
        }
    }

    Ok(ops)
}

fn copy_file<P: AsRef<Path>, Q: AsRef<Path>>(src_path: P, dest_path: Q) -> Result<()> {
    let src_path = src_path.as_ref();
    let dest_path = dest_path.as_ref();

    let dest_dir = dest_path
        .parent()
        .ok_or_else(|| anyhow!("failed to get parent dir of path {}", dest_path.display()))?;
    fs::create_dir_all(dest_dir)?;

    let mut src_file =
        File::open(src_path).with_context(|| format!("open {}", src_path.display()))?;
    let mut dest_file =
        File::create(dest_path).with_context(|| format!("open {}", dest_path.display()))?;

    io::copy(&mut src_file, &mut dest_file)?;

    Ok(())
}

fn extract_file<P: AsRef<Path>>(zip_entry: &mut ZipFile, dest_path: P) -> Result<()> {
    let dest_path = dest_path.as_ref();

    let dest_dir = dest_path
        .parent()
        .ok_or_else(|| anyhow!("failed to get parent dir of path {}", dest_path.display()))?;
    fs::create_dir_all(dest_dir)?;

    let mut dest_file =
        File::create(dest_path).with_context(|| format!("create {}", dest_path.display()))?;

    io::copy(zip_entry, &mut dest_file)?;

    Ok(())
}

fn symlink_file<P: AsRef<Path>>(link_target: &str, link_file: P) -> Result<()> {
    let link_file = link_file.as_ref();
    if fs::exists(link_file)? {
        fs::remove_file(link_file)?;
    }
    unix_symlink(link_target, link_file)
        .with_context(|| format!("symlink {} -> {}", link_file.display(), link_target))?;
    Ok(())
}

fn update_android_bp(contents: &str, version: &str) -> Option<String> {
    let replacement = format!("\"{version}\",\n        \"current\"");
    if contents.contains(&replacement) {
        return None;
    }
    Some(contents.replace(r#""current""#, &replacement))
}

fn main() -> Result<()> {
    let args = Cli::parse();

    let mut files = vec![];
    for entry in WalkDir::new(&args.dist_dir) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.into_path();
        let path = path.strip_prefix(&args.dist_dir)?;
        files.push(path.to_string_lossy().to_string());
    }

    let zip_filename = files
        .iter()
        .find(|f| f.starts_with("sdk-repo-linux-platforms-") && f.ends_with(".zip"))
        .ok_or_else(|| {
            anyhow!("failed to find an SDK zip in dist dir {}", args.dist_dir.display())
        })?;
    let zip_file = File::open(args.dist_dir.join(zip_filename))?;

    let mut ops = decide_what_to_do(&files, &zip_file)?;
    ops.push(symlink(&args.version, "latest"));

    let mut zip_archive = ZipArchive::new(zip_file).context("parse input as zip archive")?;
    let src_dir = args.dist_dir;
    let dest_dir = args.android_top.join("prebuilts").join("sdk").join(&args.version);
    for op in ops.into_iter() {
        match op {
            Operation::Copy { src, dest } => {
                let src = src_dir.join(src);
                let dest = dest_dir.join(dest);
                copy_file(&src, &dest)
                    .with_context(|| format!("copy {} -> {}", src.display(), dest.display()))?;
            }
            Operation::Extract { src, dest } => {
                let mut zip_entry = zip_archive.by_name(&src)?;
                let dest = dest_dir.join(dest);
                extract_file(&mut zip_entry, &dest)
                    .with_context(|| format!("extract {src} -> {}", dest.display()))?;
            }
            Operation::Symlink { link_target, link_file } => {
                let link_file = args.android_top.join("prebuilts").join("sdk").join(link_file);
                symlink_file(&link_target, &link_file)
                    .with_context(|| format!("symlink {} -> {link_target}", link_file.display()))?;
            }
        }
    }

    let android_bp_path = args.android_top.join("prebuilts").join("sdk").join("Android.bp");
    let contents = fs::read_to_string(&android_bp_path)
        .with_context(|| format!("open {}", android_bp_path.display()))?;
    if let Some(replacement) = update_android_bp(&contents, &args.version) {
        let mut android_bp_file = File::create(&android_bp_path)
            .with_context(|| format!("create {}", android_bp_path.display()))?;
        android_bp_file
            .write_all(replacement.as_bytes())
            .with_context(|| format!("write {}", android_bp_path.display()))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use zip::{write::FileOptions, ZipWriter};

    #[test]
    fn test_decide_what_to_do() {
        let mut zip_data = &mut Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(&mut zip_data);
        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o755);
        for path in [
            "android-SOMETHING/build.prop",                 // do not extract
            "android-SOMETHING/templates/strings.template", // do not extract
            "android-SOMETHING/android.jar",
            "android-SOMETHING/framework.aidl",
            "android-SOMETHING/uiautomator.jar",
        ] {
            writer.start_file(path, options).unwrap();
        }
        writer.finish().unwrap();
        drop(writer);

        let files: Vec<_> = [
            "apistubs/android/public/api/art.txt",
            "apistubs/android/public/api/art-removed.txt",
            "apistubs/android/system/api/framework-nfc.txt",
            "apistubs/android/system-server/api/service-art.txt",
            "apistubs/android/module-lib/api/framework-bluetooth.txt",
            "apistubs/android/test/api/android.txt",
            "apistubs/android/private/android.jar", // do not copy
            "apistubs/android/public/android.jar", // expected but will be overwritten by jar from zip
            "apistubs/android/system/android.jar",
            "apistubs/android/system-server/android.jar",
            "apistubs/android/module-lib/android.jar",
            "licensetexts/apistubs/android/system/api/android.txt", // do not copy
            "system-modules/public/core-for-system-modules.jar",
            "system-modules/module-lib/core-for-system-modules.jar",
            "sdk-repo-linux-platforms-BUILD-NUMBER.zip", // do not copy
            "data/api-versions.xml",
            "data/annotations.zip",
            "system-data/api-versions.xml",
            "system-data/annotations.zip",
            "module-lib-data/api-versions.xml",
            "module-lib-data/annotations.zip",
            "system-server-data/api-versions.xml",
            "system-server-data/annotations.zip",
            "finalized-flags.txt",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect();

        let mut expected = vec![
            // for scope in 'public', 'system', 'test', 'module-lib', 'system-server':
            //     apistubs/android/${scope}/api/*.txt -> ${scope}/api/*.txt
            copy("apistubs/android/public/api/art.txt", "public/api/art.txt"),
            copy("apistubs/android/public/api/art-removed.txt", "public/api/art-removed.txt"),
            copy("apistubs/android/system/api/framework-nfc.txt", "system/api/framework-nfc.txt"),
            copy(
                "apistubs/android/system-server/api/service-art.txt",
                "system-server/api/service-art.txt",
            ),
            copy(
                "apistubs/android/module-lib/api/framework-bluetooth.txt",
                "module-lib/api/framework-bluetooth.txt",
            ),
            copy("apistubs/android/test/api/android.txt", "test/api/android.txt"),
            // for scope in 'public', 'system', 'test', 'module-lib', 'system-server':
            //     apistubs/android/${scope}/api/*.jar -> ${scope}/*.jar
            copy("apistubs/android/public/android.jar", "public/android.jar"),
            copy("apistubs/android/system/android.jar", "system/android.jar"),
            copy("apistubs/android/system-server/android.jar", "system-server/android.jar"),
            copy("apistubs/android/module-lib/android.jar", "module-lib/android.jar"),
            // special case for system-modules/{public,module-lib}/core-for-system-modules.jar
            copy(
                "system-modules/public/core-for-system-modules.jar",
                "public/core-for-system-modules.jar",
            ),
            copy(
                "system-modules/module-lib/core-for-system-modules.jar",
                "module-lib/core-for-system-modules.jar",
            ),
            // for scope in 'public', 'system', 'module-lib', 'system-server':
            //     {data,system-data,module-lib-data,system-server-data}/{api-versions.xml,annotations.zip} ->
            //     ${scope}/data/{api-versions.xml,annotations.zip}
            copy("data/api-versions.xml", "public/data/api-versions.xml"),
            copy("data/annotations.zip", "public/data/annotations.zip"),
            copy("system-data/api-versions.xml", "system/data/api-versions.xml"),
            copy("system-data/annotations.zip", "system/data/annotations.zip"),
            copy("module-lib-data/api-versions.xml", "module-lib/data/api-versions.xml"),
            copy("module-lib-data/annotations.zip", "module-lib/data/annotations.zip"),
            copy("system-server-data/api-versions.xml", "system-server/data/api-versions.xml"),
            copy("system-server-data/annotations.zip", "system-server/data/annotations.zip"),
            // finalized-flags.txt
            copy("finalized-flags.txt", "finalized-flags.txt"),
            // files extracted from the SDK zip
            extract("android-SOMETHING/android.jar", "public/android.jar"),
            extract("android-SOMETHING/framework.aidl", "public/framework.aidl"),
            extract("android-SOMETHING/uiautomator.jar", "public/uiautomator.jar"),
        ];
        expected.sort_unstable();

        let mut actual = decide_what_to_do(&files, zip_data).unwrap();
        actual.sort_unstable();

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_update_android_bp() {
        let before = r#"
prebuilt_apis {
    name: "sdk",
    api_dirs: [
        "1",
        "2",
        "3",
        "4",
        "4.1",
        "current",
    ],
    extensions_dir: "extensions",
    next_api_dir: "37",
    imports_sdk_version: "none",
    imports_compile_dex: true,
}"#;

        let after = r#"
prebuilt_apis {
    name: "sdk",
    api_dirs: [
        "1",
        "2",
        "3",
        "4",
        "4.1",
        "5",
        "current",
    ],
    extensions_dir: "extensions",
    next_api_dir: "37",
    imports_sdk_version: "none",
    imports_compile_dex: true,
}"#;

        assert_eq!(update_android_bp(before, "5"), Some(after.to_string()));
        assert!(update_android_bp(after, "5").is_none());
    }
}
