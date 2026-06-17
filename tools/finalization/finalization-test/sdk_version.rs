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

use std::fmt;
use std::str::FromStr;

#[allow(dead_code)]
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SdkVersion {
    // The order of the fields is significant for the derived implementation of Ord
    pub major: u32,
    pub minor: u32,
}

impl FromStr for SdkVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let error = Err(format!("failed to convert '{}' to SdkVersion", s));

        if let Some((major, minor)) = s.split_once('.') {
            let Ok(major) = major.parse::<u32>() else {
                return error;
            };
            let Ok(minor) = minor.parse::<u32>() else {
                return error;
            };
            Ok(SdkVersion { major, minor })
        } else {
            let Ok(major) = s.parse::<u32>() else {
                return error;
            };
            Ok(SdkVersion { major, minor: 0 })
        }
    }
}

impl fmt::Display for SdkVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sdk_version_try_from_string() {
        assert_eq!(("32").parse::<SdkVersion>().unwrap(), SdkVersion { major: 32, minor: 0 });
        assert_eq!("32.0".parse::<SdkVersion>().unwrap(), SdkVersion { major: 32, minor: 0 });
        assert_eq!("48.64".parse::<SdkVersion>().unwrap(), SdkVersion { major: 48, minor: 64 });

        assert_eq!(
            "48.64.0".parse::<SdkVersion>().unwrap_err(),
            "failed to convert '48.64.0' to SdkVersion".to_string()
        );
        assert_eq!(
            "".parse::<SdkVersion>().unwrap_err(),
            "failed to convert '' to SdkVersion".to_string()
        );
        assert_eq!(
            "foo".parse::<SdkVersion>().unwrap_err(),
            "failed to convert 'foo' to SdkVersion".to_string()
        );
    }

    #[test]
    fn sdk_version_ordering() {
        assert!("10".parse::<SdkVersion>().unwrap() < "11".parse::<SdkVersion>().unwrap());
        assert!("10".parse::<SdkVersion>().unwrap() < "11.1".parse::<SdkVersion>().unwrap());
        assert!("32.1".parse::<SdkVersion>().unwrap() < "32.2".parse::<SdkVersion>().unwrap());

        assert!("32".parse::<SdkVersion>().unwrap() == "32".parse::<SdkVersion>().unwrap());
        assert!("32".parse::<SdkVersion>().unwrap() == "32.0".parse::<SdkVersion>().unwrap());
        assert!("32.1".parse::<SdkVersion>().unwrap() == "32.1".parse::<SdkVersion>().unwrap());
    }

    #[test]
    fn sdk_version_display() {
        assert_eq!(format!("{}", "10".parse::<SdkVersion>().unwrap()), "10.0");
        assert_eq!(format!("{}", "32.64".parse::<SdkVersion>().unwrap()), "32.64");
    }
}
