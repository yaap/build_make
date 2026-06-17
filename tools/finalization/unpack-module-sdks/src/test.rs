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

#[cfg(test)]
use std::io::{Seek, Write};

#[cfg(test)]
pub static MAINLINE_MODULES_INFO: &str = r#"
    {
    "com.android.appsearch": {
        "module_sdk_project": "prebuilts/module_sdk/AppSearch",
        "module_proto_key": "APPSEARCH",
        "sdk_name": "appsearch-sdk"
    },
    "com.android.sdkext": {
        "module_sdk_project": "prebuilts/module_sdk/SdkExtensions",
        "module_proto_key": "SDK_EXTENSIONS",
        "sdk_name": "sdkextensions-sdk"
    }
}
"#;

#[cfg(test)]
pub fn write_test_zip_archive<W: Write + Seek>(buf: W) {
    use zip::{write::FileOptions, ZipWriter};

    let mut zip = ZipWriter::new(buf);
    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .unix_permissions(0o755);

    for path in [
        "Android.bp",
        "hiddenapi/annotation-flags.csv",
        "hiddenapi/filtered-flags.csv",
        "hiddenapi/filtered-stub-flags.csv",
        "hiddenapi/index.csv",
        "hiddenapi/metadata.csv",
        "hiddenapi/signature-patterns.csv",
        "licenses/build/soong/licenses/LICENSE",
        "licenses/build/soong/licenses/opensourcerequest",
        "sdk_library/module-lib/framework-sdkextensions-removed.txt",
        "sdk_library/module-lib/framework-sdkextensions-stubs.jar",
        "sdk_library/module-lib/framework-sdkextensions.srcjar",
        "sdk_library/module-lib/framework-sdkextensions.txt",
        "sdk_library/module-lib/framework-sdkextensions_annotations.zip",
        "sdk_library/public/framework-sdkextensions-removed.txt",
        "sdk_library/public/framework-sdkextensions-stubs.jar",
        "sdk_library/public/framework-sdkextensions.srcjar",
        "sdk_library/public/framework-sdkextensions.txt",
        "sdk_library/public/framework-sdkextensions_annotations.zip",
        "sdk_library/system/framework-sdkextensions-removed.txt",
        "sdk_library/system/framework-sdkextensions-stubs.jar",
        "sdk_library/system/framework-sdkextensions.srcjar",
        "sdk_library/system/framework-sdkextensions.txt",
        "sdk_library/system/framework-sdkextensions_annotations.zip",
        "snapshot-creation-build-number.txt",
    ] {
        zip.start_file(path, options).unwrap();
    }

    zip.finish().unwrap();
}
