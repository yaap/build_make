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

use anyhow::{Context, Result};
use regex::Regex;
use std::{
    collections::HashSet,
    io::{Read, Seek},
};
use zip::ZipArchive;

pub fn process_module_sdk<R: Read + Seek>(reader: R) -> Result<HashSet<(String, String)>> {
    let archive = ZipArchive::new(reader).context("parse input as zip archive")?;
    Ok(archive
        .file_names()
        .map(|s| {
            let src = s.to_string();
            let mut dest = src.clone();
            if src == "Android.bp" || src.ends_with("/Android.bp") {
                dest.push_str(".auto");
            }
            (src, dest)
        })
        .collect())
}

pub fn process_platform_sdk_extensions<R: Read + Seek>(
    reader: R,
) -> Result<HashSet<(String, String)>> {
    let archive = ZipArchive::new(reader).context("parse input as zip archive")?;
    let regex = Regex::new(r"sdk_library/(.*?)/(.*?\.(?:txt|jar))").unwrap();

    let mut out = HashSet::new();
    for caps in archive.file_names().filter_map(|entry| regex.captures(entry)) {
        let entry_path = &caps[0];
        let api_type = &caps[1];
        let entry_filename = &caps[2];
        if is_ignored(entry_filename) {
            continue;
        }
        let filename = tweak_filename(entry_filename);
        let dest = if entry_filename.ends_with(".txt") {
            format!("{api_type}/api/{filename}")
        } else {
            format!("{api_type}/{filename}")
        };

        out.insert((entry_path.to_string(), dest));
    }

    Ok(out)
}

fn is_ignored(filename: &str) -> bool {
    // conscrypt has some legacy API tracking files that we don't consider for extensions
    ["conscrypt.module.intra.core.api", "conscrypt.module.platform.api"]
        .iter()
        .any(|prefix| filename.starts_with(prefix))
}

fn tweak_filename(filename: &str) -> String {
    // for legacy reasons, art and conscrypt file names in the SDKs
    // (*.module.public.api) do not match their expected filename in prebuilts/sdk
    // (art, conscrypt), so rename them
    //
    // the stub jar artifacts from official builds are named '*-stubs.jar', but the
    // convention for the copies in prebuilts/sdk is just '*.jar', so fix that
    filename
        .replace("art.module.public.api", "art")
        .replace("conscrypt.module.public.api", "conscrypt")
        .replace("-stubs", "")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_process_module_sdk() {
        let expected = HashSet::from([
            ("Android.bp", "Android.bp.auto"),
            ("hiddenapi/annotation-flags.csv", "hiddenapi/annotation-flags.csv"),
            ("hiddenapi/filtered-flags.csv", "hiddenapi/filtered-flags.csv"),
            ("hiddenapi/filtered-stub-flags.csv", "hiddenapi/filtered-stub-flags.csv"),
            ("hiddenapi/index.csv", "hiddenapi/index.csv"),
            ("hiddenapi/metadata.csv", "hiddenapi/metadata.csv"),
            ("hiddenapi/signature-patterns.csv", "hiddenapi/signature-patterns.csv"),
            ("licenses/build/soong/licenses/LICENSE", "licenses/build/soong/licenses/LICENSE"),
            (
                "licenses/build/soong/licenses/opensourcerequest",
                "licenses/build/soong/licenses/opensourcerequest",
            ),
            (
                "sdk_library/module-lib/framework-sdkextensions-removed.txt",
                "sdk_library/module-lib/framework-sdkextensions-removed.txt",
            ),
            (
                "sdk_library/module-lib/framework-sdkextensions-stubs.jar",
                "sdk_library/module-lib/framework-sdkextensions-stubs.jar",
            ),
            (
                "sdk_library/module-lib/framework-sdkextensions.srcjar",
                "sdk_library/module-lib/framework-sdkextensions.srcjar",
            ),
            (
                "sdk_library/module-lib/framework-sdkextensions.txt",
                "sdk_library/module-lib/framework-sdkextensions.txt",
            ),
            (
                "sdk_library/module-lib/framework-sdkextensions_annotations.zip",
                "sdk_library/module-lib/framework-sdkextensions_annotations.zip",
            ),
            (
                "sdk_library/public/framework-sdkextensions-removed.txt",
                "sdk_library/public/framework-sdkextensions-removed.txt",
            ),
            (
                "sdk_library/public/framework-sdkextensions-stubs.jar",
                "sdk_library/public/framework-sdkextensions-stubs.jar",
            ),
            (
                "sdk_library/public/framework-sdkextensions.srcjar",
                "sdk_library/public/framework-sdkextensions.srcjar",
            ),
            (
                "sdk_library/public/framework-sdkextensions.txt",
                "sdk_library/public/framework-sdkextensions.txt",
            ),
            (
                "sdk_library/public/framework-sdkextensions_annotations.zip",
                "sdk_library/public/framework-sdkextensions_annotations.zip",
            ),
            (
                "sdk_library/system/framework-sdkextensions-removed.txt",
                "sdk_library/system/framework-sdkextensions-removed.txt",
            ),
            (
                "sdk_library/system/framework-sdkextensions-stubs.jar",
                "sdk_library/system/framework-sdkextensions-stubs.jar",
            ),
            (
                "sdk_library/system/framework-sdkextensions.srcjar",
                "sdk_library/system/framework-sdkextensions.srcjar",
            ),
            (
                "sdk_library/system/framework-sdkextensions.txt",
                "sdk_library/system/framework-sdkextensions.txt",
            ),
            (
                "sdk_library/system/framework-sdkextensions_annotations.zip",
                "sdk_library/system/framework-sdkextensions_annotations.zip",
            ),
            ("snapshot-creation-build-number.txt", "snapshot-creation-build-number.txt"),
        ]);
        let mut zip = &mut Cursor::new(Vec::new());
        crate::test::write_test_zip_archive(&mut zip);
        let actual = process_module_sdk(zip).unwrap();
        let actual: HashSet<(&str, &str)> =
            actual.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_process_platform_sdk_extensions() {
        let expected: HashSet<(&str, &str)> = HashSet::from([
            (
                "sdk_library/module-lib/framework-sdkextensions-removed.txt",
                "module-lib/api/framework-sdkextensions-removed.txt",
            ),
            (
                "sdk_library/module-lib/framework-sdkextensions.txt",
                "module-lib/api/framework-sdkextensions.txt",
            ),
            (
                "sdk_library/module-lib/framework-sdkextensions-stubs.jar",
                "module-lib/framework-sdkextensions.jar",
            ),
            (
                "sdk_library/public/framework-sdkextensions-removed.txt",
                "public/api/framework-sdkextensions-removed.txt",
            ),
            (
                "sdk_library/public/framework-sdkextensions.txt",
                "public/api/framework-sdkextensions.txt",
            ),
            (
                "sdk_library/public/framework-sdkextensions-stubs.jar",
                "public/framework-sdkextensions.jar",
            ),
            (
                "sdk_library/system/framework-sdkextensions-removed.txt",
                "system/api/framework-sdkextensions-removed.txt",
            ),
            (
                "sdk_library/system/framework-sdkextensions.txt",
                "system/api/framework-sdkextensions.txt",
            ),
            (
                "sdk_library/system/framework-sdkextensions-stubs.jar",
                "system/framework-sdkextensions.jar",
            ),
        ]);
        let mut zip = &mut Cursor::new(Vec::new());
        crate::test::write_test_zip_archive(&mut zip);
        let actual = process_platform_sdk_extensions(zip).unwrap();
        let actual: HashSet<(&str, &str)> =
            actual.iter().map(|(src, dest)| (src.as_str(), dest.as_str())).collect();

        dbg!(&actual);
        dbg!(&expected);
        assert_eq!(actual, expected);
    }
}
