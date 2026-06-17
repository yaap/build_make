/*
 * Copyright (C) 2024 The Android Open Source Project
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

//! flag value query module defines the flag value file read from mapped bytes

use crate::AconfigStorageError;
use aconfig_storage_file::{
    flag_value::FlagValueHeader, read_i64_from_bytes, read_u8_from_bytes,
    MAX_SUPPORTED_FILE_VERSION,
};
use anyhow::anyhow;

/// Query flag value
pub fn find_boolean_flag_value(buf: &[u8], flag_index: u32) -> Result<bool, AconfigStorageError> {
    let interpreted_header = read_header_and_check_version(buf)?;
    let mut head = interpreted_header.get_offset_for_boolean_flag(flag_index)?;

    let val = read_u8_from_bytes(buf, &mut head)?;
    Ok(val == 1)
}

pub fn find_int64_flag_value(buf: &[u8], flag_index: u32) -> Result<i64, AconfigStorageError> {
    if !cfg!(enable_parse_v4) {
        return Err(AconfigStorageError::HigherStorageFileVersion(anyhow!(
            "Int64 not supported for flag value files."
        )));
    }

    let interpreted_header = read_header_and_check_version(buf)?;
    let mut head = interpreted_header.get_offset_for_int_flag(flag_index)?;

    let val = read_i64_from_bytes(buf, &mut head)?;
    Ok(val)
}

fn read_header_and_check_version(buf: &[u8]) -> Result<FlagValueHeader, AconfigStorageError> {
    let interpreted_header = FlagValueHeader::from_bytes(buf)?;
    if interpreted_header.version > MAX_SUPPORTED_FILE_VERSION {
        return Err(AconfigStorageError::HigherStorageFileVersion(anyhow!(
            "Cannot read storage file with a higher version of {} with lib version {}",
            interpreted_header.version,
            MAX_SUPPORTED_FILE_VERSION
        )));
    }
    Ok(interpreted_header)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aconfig_storage_file::test_utils::create_test_flag_value_list;

    #[test]
    // this test point locks down flag value query
    fn test_flag_value_query() {
        for version in 1..=MAX_SUPPORTED_FILE_VERSION {
            let flag_value_list = create_test_flag_value_list(version).into_bytes();
            let baseline: Vec<bool> = vec![false, true, true, false, true, true, true, true];
            for (offset, expected_value) in baseline.into_iter().enumerate() {
                let flag_value =
                    find_boolean_flag_value(&flag_value_list[..], offset as u32).unwrap();
                assert_eq!(flag_value, expected_value);
            }
        }
    }

    #[test]
    #[cfg(enable_parse_v4)]
    fn test_int64_flag_value_query() {
        let flag_value_list = create_test_flag_value_list(4).into_bytes();
        let baseline: Vec<i64> = vec![0, 1, 2, 3, 4, 5, 6, 7];
        for (offset, expected_value) in baseline.into_iter().enumerate() {
            let flag_value = find_int64_flag_value(&flag_value_list[..], offset as u32).unwrap();
            assert_eq!(flag_value, expected_value);
        }
    }

    #[test]
    // this test point locks down query beyond the end of boolean section
    fn test_boolean_out_of_range() {
        for version in 1..=MAX_SUPPORTED_FILE_VERSION {
            let flag_value_list = create_test_flag_value_list(version).into_bytes();
            let error = find_boolean_flag_value(&flag_value_list[..], 8).unwrap_err();
            assert!(format!("{:?}", error)
                .starts_with("InvalidStorageFileOffset(Flag value offset goes beyond"));
        }
    }

    #[test]
    #[cfg(enable_parse_v4)]
    // this test point locks down query beyond the end of int64 section
    fn test_int64_out_of_range() {
        let flag_value_list = create_test_flag_value_list(4).into_bytes();
        let error = find_int64_flag_value(&flag_value_list[..], 8).unwrap_err();
        assert!(format!("{:?}", error).starts_with(
            "InvalidStorageFileOffset(Flag value offset goes beyond the end of the file."
        ));
    }

    #[test]
    #[cfg(not(enable_parse_v4))]
    fn test_int64_disabled() {
        let flag_value_list = create_test_flag_value_list(4).into_bytes();
        let error = find_int64_flag_value(&flag_value_list[..], 3).unwrap_err();
        assert!(format!("{:?}", error)
            .starts_with("HigherStorageFileVersion(Int64 not supported for flag value files."));
    }

    #[test]
    // this test point locks down query error when file has a higher version
    fn test_higher_version_storage_file() {
        for version in 1..=MAX_SUPPORTED_FILE_VERSION {
            let mut value_list = create_test_flag_value_list(version);
            value_list.header.version = MAX_SUPPORTED_FILE_VERSION + 1;
            let flag_value = value_list.into_bytes();
            let error = find_boolean_flag_value(&flag_value[..], 4).unwrap_err();
            assert!(
            format!("{:?}", error).starts_with(
                &format!(
                    "HigherStorageFileVersion(Cannot read storage file with a higher version of {} with lib version {}",
                    MAX_SUPPORTED_FILE_VERSION + 1,
                    MAX_SUPPORTED_FILE_VERSION
                ))
            );
        }
    }
}
