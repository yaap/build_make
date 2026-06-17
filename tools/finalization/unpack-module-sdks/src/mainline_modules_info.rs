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
use serde::Deserialize;
use std::{collections::HashMap, io::Read};

#[derive(Debug, Deserialize)]
struct ModuleInfo {
    // mainline-modules-info.json entries contain more fields but we only care about this one
    module_sdk_project: String,
}

// Parse a mainline-modules-info.json file. Return a mapping of module ID in mainline SDK
// directory to mainline SDK path in the Android tree.
pub fn parse_mainline_module_info_json<R: Read>(reader: R) -> Result<HashMap<String, String>> {
    let map: HashMap<String, ModuleInfo> =
        serde_json::from_reader(reader).context("deserialize json")?;
    Ok(map.into_iter().map(|(k, v)| (k, v.module_sdk_project)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mainline_module_info_json() {
        let expected = HashMap::from([
            ("com.android.appsearch".to_string(), "prebuilts/module_sdk/AppSearch".to_string()),
            ("com.android.sdkext".to_string(), "prebuilts/module_sdk/SdkExtensions".to_string()),
        ]);
        let actual =
            parse_mainline_module_info_json(crate::test::MAINLINE_MODULES_INFO.as_bytes()).unwrap();
        assert_eq!(actual, expected);
    }
}
