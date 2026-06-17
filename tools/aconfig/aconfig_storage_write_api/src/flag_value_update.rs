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

//! flag value update module defines the flag value file write to mapped bytes

use aconfig_storage_file::{AconfigStorageError, FlagValueHeader, MAX_SUPPORTED_FILE_VERSION};
use anyhow::anyhow;

/// Set flag value
pub fn update_boolean_flag_value(
    buf: &mut [u8],
    flag_index: u32,
    flag_value: bool,
) -> Result<usize, AconfigStorageError> {
    let interpreted_header = read_header_and_check_version(buf)?;
    let head = interpreted_header.get_offset_for_boolean_flag(flag_index)?;

    buf[head] = u8::from(flag_value).to_le_bytes()[0];
    Ok(head)
}

// Returns the byte offset to the flag value that was written.
pub fn update_int64_flag_value(
    buf: &mut [u8],
    flag_index: u32,
    flag_value: i64,
) -> Result<usize, AconfigStorageError> {
    if !cfg!(enable_parse_v4) {
        return Err(AconfigStorageError::HigherStorageFileVersion(anyhow!(
            "Int64 not supported for flag value files."
        )));
    }
    let interpreted_header = read_header_and_check_version(buf)?;
    let head = interpreted_header.get_offset_for_int_flag(flag_index)?;

    // 8 bytes per int flag.
    let flag_value_bytes: [u8; 8] = flag_value.to_le_bytes();
    buf[head..head + 8].copy_from_slice(&flag_value_bytes);
    Ok(head)
}

fn read_header_and_check_version(buf: &mut [u8]) -> Result<FlagValueHeader, AconfigStorageError> {
    let interpreted_header = FlagValueHeader::from_bytes(buf)?;
    if interpreted_header.version > MAX_SUPPORTED_FILE_VERSION {
        return Err(AconfigStorageError::HigherStorageFileVersion(anyhow!(
            "Cannot write to storage file with a higher version of {} with lib version {}",
            interpreted_header.version,
            MAX_SUPPORTED_FILE_VERSION
        )));
    }
    Ok(interpreted_header)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aconfig_storage_file::{test_utils::create_test_flag_value_list, DEFAULT_FILE_VERSION};

    #[test]
    // this test point locks down flag value update
    fn test_boolean_flag_value_update() {
        let flag_value_list = create_test_flag_value_list(DEFAULT_FILE_VERSION);
        let value_offset = flag_value_list.header.boolean_value_offset;
        let mut content = flag_value_list.into_bytes();
        let true_byte = u8::from(true).to_le_bytes()[0];
        let false_byte = u8::from(false).to_le_bytes()[0];

        for i in 0..flag_value_list.header.num_boolean_flags {
            let offset = (value_offset + i) as usize;
            update_boolean_flag_value(&mut content, i, true).unwrap();
            assert_eq!(content[offset], true_byte);
            update_boolean_flag_value(&mut content, i, false).unwrap();
            assert_eq!(content[offset], false_byte);
        }
    }

    #[cfg(enable_parse_v4)]
    #[test]
    fn test_int_flag_value_update() {
        let flag_value_list = create_test_flag_value_list(4);
        let value_offset = flag_value_list.header.int_value_offset;
        let mut content = flag_value_list.into_bytes();

        let flag_value: i64 = 99;
        for i in 0..flag_value_list.header.num_int_flags {
            let offset = (value_offset + (i * 8)) as usize;
            update_int64_flag_value(&mut content, i, flag_value).unwrap();
            let updated_bytes: [u8; 8] = content[offset..offset + 8].try_into().unwrap();
            assert_eq!(i64::from_le_bytes(updated_bytes), flag_value);
        }
    }

    #[cfg(not(enable_parse_v4))]
    #[test]
    fn test_int_flag_value_update_not_supported() {
        let flag_value_list = create_test_flag_value_list(4);
        let mut content = flag_value_list.into_bytes();

        let error = update_int64_flag_value(
            &mut content,
            /* flag_index= */ 4,
            /* flag_value= */ 8,
        )
        .unwrap_err();
        assert!(format!("{:?}", error)
            .starts_with("HigherStorageFileVersion(Int64 not supported for flag value files."));
    }

    #[test]
    // this test point locks down update beyond the end of boolean section
    fn test_boolean_out_of_range() {
        let mut flag_value_list = create_test_flag_value_list(DEFAULT_FILE_VERSION).into_bytes();
        let error = update_boolean_flag_value(&mut flag_value_list[..], 8, true).unwrap_err();
        assert!(format!("{:?}", error).starts_with(
            "InvalidStorageFileOffset(Flag value offset goes beyond the end of the file."
        ));
    }

    #[cfg(enable_parse_v4)]
    #[test]
    // this test point locks down update beyond the end of int section
    fn test_int_out_of_range() {
        let mut flag_value_list = create_test_flag_value_list(4).into_bytes();
        let error = update_int64_flag_value(&mut flag_value_list[..], 8, 1).unwrap_err();
        assert!(format!("{:?}", error).starts_with(
            "InvalidStorageFileOffset(Flag value offset goes beyond the end of the file."
        ));
    }

    #[test]
    // this test point locks down query error when file has a higher version
    fn test_higher_version_storage_file() {
        let mut value_list = create_test_flag_value_list(DEFAULT_FILE_VERSION);
        value_list.header.version = MAX_SUPPORTED_FILE_VERSION + 8;
        let mut flag_value = value_list.into_bytes();
        let error = update_boolean_flag_value(&mut flag_value[..], 4, true).unwrap_err();
        assert!(
            format!("{:?}", error).starts_with(
            &format!(
                "HigherStorageFileVersion(Cannot read storage file with a higher version of {} with lib version {}",
                MAX_SUPPORTED_FILE_VERSION + 8,
                MAX_SUPPORTED_FILE_VERSION
            ))
        );
    }
}
