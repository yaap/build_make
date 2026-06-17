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

//! flag value module defines the flag value file format and methods for serialization
//! and deserialization

use crate::{read_i64_from_bytes, read_str_from_bytes, read_u32_from_bytes, read_u8_from_bytes};
use crate::{AconfigStorageError, StorageFileType, MAX_SUPPORTED_FILE_VERSION};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Flag value header struct
#[derive(PartialEq, Serialize, Deserialize)]
pub struct FlagValueHeader {
    pub version: u32,
    pub container: String,
    pub file_type: u8,
    pub file_size: u32,
    pub num_boolean_flags: u32,
    pub boolean_value_offset: u32,
    pub num_int_flags: u32,
    pub int_value_offset: u32,
}

/// Implement debug print trait for header
impl fmt::Debug for FlagValueHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "Version: {}, Container: {}, File Type: {:?}, File Size: {}",
            self.version,
            self.container,
            StorageFileType::try_from(self.file_type),
            self.file_size
        )?;
        writeln!(
            f,
            "Num of Boolean Flags: {}, Boolean Value Offset:{}",
            self.num_boolean_flags, self.boolean_value_offset
        )?;
        if self.num_int_flags > 0 {
            writeln!(
                f,
                "Num of Int Flags: {}, Int Value Offset: {}",
                self.num_int_flags, self.int_value_offset
            )?;
        }
        Ok(())
    }
}

impl FlagValueHeader {
    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        match self.version {
            1..=3 => self.to_bytes_v1(),
            4 if cfg!(enable_parse_v4) => self.to_bytes_v4(),
            // TODO(b/444251791): into_bytes should return a Result and panic
            // if version is not supported.
            _ => self.to_bytes_v1(),
        }
    }

    fn to_bytes_v1(&self) -> Vec<u8> {
        let mut result = Vec::new();
        result.extend_from_slice(&self.version.to_le_bytes());
        let container_bytes = self.container.as_bytes();
        result.extend_from_slice(&(container_bytes.len() as u32).to_le_bytes());
        result.extend_from_slice(container_bytes);
        result.extend_from_slice(&self.file_type.to_le_bytes());
        result.extend_from_slice(&self.file_size.to_le_bytes());
        result.extend_from_slice(&self.num_boolean_flags.to_le_bytes());
        result.extend_from_slice(&self.boolean_value_offset.to_le_bytes());
        // Int support not implemented in v1, so don't write those bytes.
        result
    }

    fn to_bytes_v4(&self) -> Vec<u8> {
        let mut result = Vec::new();
        result.extend_from_slice(&self.version.to_le_bytes());
        let container_bytes = self.container.as_bytes();
        result.extend_from_slice(&(container_bytes.len() as u32).to_le_bytes());
        result.extend_from_slice(container_bytes);
        result.extend_from_slice(&self.file_type.to_le_bytes());
        result.extend_from_slice(&self.file_size.to_le_bytes());
        result.extend_from_slice(&self.num_boolean_flags.to_le_bytes());
        result.extend_from_slice(&self.boolean_value_offset.to_le_bytes());
        result.extend_from_slice(&self.num_int_flags.to_le_bytes());
        result.extend_from_slice(&self.int_value_offset.to_le_bytes());
        result
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AconfigStorageError> {
        // Version is ALWAYS at the top of the file.
        let mut head = 0;
        let version_from_bytes = read_u32_from_bytes(bytes, &mut head)?;

        let list = match version_from_bytes {
            1..=3 => Self::from_bytes_v1(bytes, version_from_bytes, &mut head),
            4 if cfg!(enable_parse_v4) => Self::from_bytes_v4(bytes, version_from_bytes, &mut head),
            _ => {
                return Err(AconfigStorageError::HigherStorageFileVersion(anyhow!(
                    "Cannot read storage file with a higher version of {} with lib version {}",
                    version_from_bytes,
                    MAX_SUPPORTED_FILE_VERSION
                )))
            }
        }?;

        if list.file_type != StorageFileType::FlagVal as u8 {
            return Err(AconfigStorageError::BytesParseFail(anyhow!(
                "binary file is not a flag value file"
            )));
        }
        Ok(list)
    }

    fn from_bytes_v1(
        bytes: &[u8],
        version_from_bytes: u32,
        head: &mut usize,
    ) -> Result<Self, AconfigStorageError> {
        Ok(Self {
            version: version_from_bytes,
            container: read_str_from_bytes(bytes, head)?,
            file_type: read_u8_from_bytes(bytes, head)?,
            file_size: read_u32_from_bytes(bytes, head)?,
            num_boolean_flags: read_u32_from_bytes(bytes, head)?,
            boolean_value_offset: read_u32_from_bytes(bytes, head)?,
            num_int_flags: 0u32,
            int_value_offset: 0u32,
        })
    }

    fn from_bytes_v4(
        bytes: &[u8],
        version_from_bytes: u32,
        head: &mut usize,
    ) -> Result<Self, AconfigStorageError> {
        Ok(Self {
            version: version_from_bytes,
            container: read_str_from_bytes(bytes, head)?,
            file_type: read_u8_from_bytes(bytes, head)?,
            file_size: read_u32_from_bytes(bytes, head)?,
            num_boolean_flags: read_u32_from_bytes(bytes, head)?,
            boolean_value_offset: read_u32_from_bytes(bytes, head)?,
            num_int_flags: read_u32_from_bytes(bytes, head)?,
            int_value_offset: read_u32_from_bytes(bytes, head)?,
        })
    }

    // Helper methods for read/write.
    pub fn get_offset_for_boolean_flag(
        &self,
        flag_index: u32,
    ) -> Result<usize, AconfigStorageError> {
        let offset = (self.boolean_value_offset + flag_index) as usize;

        match self.version {
            // Before int flag support, booleans are at the end of the file, so
            //just make sure the offset is not out of bounds.
            1..=3 => {
                if offset >= self.file_size as usize {
                    return Err(AconfigStorageError::InvalidStorageFileOffset(anyhow!(
                        "Flag value offset goes beyond the end of the file."
                    )));
                }
            }
            // File format:
            // [0: header][boolean_value_offset: boolean_values][int_value_offset: int_values]
            // So the offset should be within [boolean_value_offset, int_value_offset).
            4 if cfg!(enable_parse_v4) => {
                if offset >= self.int_value_offset as usize {
                    return Err(AconfigStorageError::InvalidStorageFileOffset(anyhow!(
                        "Flag value offset goes beyond the end of the boolean section."
                    )));
                }
            }
            _ => {
                return Err(AconfigStorageError::HigherStorageFileVersion(anyhow!(
                    "Cannot read storage file with a higher version of {} with lib version {}",
                    self.version,
                    MAX_SUPPORTED_FILE_VERSION
                )))
            }
        }
        Ok(offset)
    }

    pub fn get_offset_for_int_flag(&self, flag_index: u32) -> Result<usize, AconfigStorageError> {
        if self.version < 4 {
            return Err(AconfigStorageError::HigherStorageFileVersion(anyhow!(
                "Don't support ints before version 4"
            )));
        }
        if self.version == 4 && !cfg!(enable_parse_v4) {
            return Err(AconfigStorageError::HigherStorageFileVersion(anyhow!(
                "Version 4 is not supported in this build"
            )));
        }

        let offset = (self.int_value_offset + (flag_index * 8)) as usize;

        // (current spot)+7 to account for the 8 bytes that actually contain the
        // int64 value.
        if (offset + 7) >= self.file_size as usize {
            return Err(AconfigStorageError::InvalidStorageFileOffset(anyhow!(
                "Flag value offset goes beyond the end of the file."
            )));
        }
        Ok(offset)
    }
}

/// Flag value list struct
#[derive(PartialEq, Serialize, Deserialize)]
pub struct FlagValueList {
    pub header: FlagValueHeader,
    pub booleans: Vec<bool>,
    pub ints: Vec<i64>,
}

/// Implement debug print trait for flag value
impl fmt::Debug for FlagValueList {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Header:")?;
        write!(f, "{:?}", self.header)?;
        writeln!(f, "Boolean Values:")?;
        writeln!(f, "{:?}", self.booleans)?;
        if !self.ints.is_empty() {
            writeln!(f, "Integer Values:")?;
            writeln!(f, "{:?}", self.ints)?;
        }
        Ok(())
    }
}

impl FlagValueList {
    /// Serialize to bytes
    pub fn into_bytes(&self) -> Vec<u8> {
        match self.header.version {
            1..=3 => self.as_bytes_v1(),
            4 if cfg!(enable_parse_v4) => self.as_bytes_v4(),
            // TODO(b/316357686): into_bytes should return a Result.
            _ => self.as_bytes_v1(),
        }
    }

    fn as_bytes_v1(&self) -> Vec<u8> {
        [
            self.header.to_bytes(),
            self.booleans.iter().map(|&v| u8::from(v).to_le_bytes()).collect::<Vec<_>>().concat(),
        ]
        .concat()
    }

    fn as_bytes_v4(&self) -> Vec<u8> {
        [
            self.header.to_bytes(),
            self.booleans.iter().map(|&v| u8::from(v).to_le_bytes()).collect::<Vec<_>>().concat(),
            self.ints.iter().map(|&v| v.to_le_bytes()).collect::<Vec<_>>().concat(),
        ]
        .concat()
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AconfigStorageError> {
        let bytes_len = bytes.len();
        let header = FlagValueHeader::from_bytes(bytes)?;
        // Check that the file size in the header matches the actual file size.
        // If they don't match, then the file is probably corrupt.
        if header.file_size != bytes_len as u32 {
            return Err(AconfigStorageError::BytesParseFail(anyhow!(
                "File size in header {} does not match actual file size {} for version {}",
                header.file_size,
                bytes_len,
                header.version
            )));
        }
        let list = match header.version {
            1..=3 => Self::from_bytes_v1(bytes, header),
            4 if cfg!(enable_parse_v4) => Self::from_bytes_v4(bytes, header),
            _ => {
                return Err(AconfigStorageError::BytesParseFail(anyhow!(
                    "Binary file is an unsupported version: {}",
                    header.version
                )))
            }
        }?;
        Ok(list)
    }

    fn from_bytes_v1(bytes: &[u8], header: FlagValueHeader) -> Result<Self, AconfigStorageError> {
        let num_flags = header.num_boolean_flags;
        let mut head = header.to_bytes().len();
        let booleans =
            (0..num_flags).map(|_| read_u8_from_bytes(bytes, &mut head).unwrap() == 1).collect();
        let list = Self { header, booleans, ints: vec![] };
        Ok(list)
    }

    fn from_bytes_v4(bytes: &[u8], header: FlagValueHeader) -> Result<Self, AconfigStorageError> {
        let num_boolean_flags = header.num_boolean_flags;
        let num_int_flags = header.num_int_flags;

        let mut head = header.to_bytes().len();
        let booleans = (0..num_boolean_flags)
            .map(|_| read_u8_from_bytes(bytes, &mut head).unwrap() == 1)
            .collect();
        let ints =
            (0..num_int_flags).map(|_| read_i64_from_bytes(bytes, &mut head).unwrap()).collect();
        let list = Self { header, booleans, ints };
        Ok(list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        test_utils::create_test_flag_value_list, DEFAULT_FILE_VERSION, MAX_SUPPORTED_FILE_VERSION,
    };

    #[test]
    // this test point locks down the value list serialization
    fn test_serialization() {
        for file_version in 1..=MAX_SUPPORTED_FILE_VERSION {
            let flag_value_list = create_test_flag_value_list(file_version);

            let header: &FlagValueHeader = &flag_value_list.header;
            let reinterpreted_header = FlagValueHeader::from_bytes(&header.to_bytes());
            assert!(reinterpreted_header.is_ok());
            assert_eq!(header, &reinterpreted_header.unwrap());

            let flag_value_bytes = flag_value_list.into_bytes();
            let reinterpreted_value_list = FlagValueList::from_bytes(&flag_value_bytes);
            assert!(reinterpreted_value_list.is_ok());
            assert_eq!(&flag_value_list, &reinterpreted_value_list.unwrap());
            assert_eq!(flag_value_bytes.len() as u32, header.file_size);
        }
    }

    #[test]
    // this test point locks down that version number should be at the top of serialized
    // bytes
    fn test_version_number() {
        let flag_value_list = create_test_flag_value_list(DEFAULT_FILE_VERSION);
        let bytes = &flag_value_list.into_bytes();
        let mut head = 0;
        let version_from_file = read_u32_from_bytes(bytes, &mut head).unwrap();
        assert_eq!(version_from_file, DEFAULT_FILE_VERSION);
    }

    #[test]
    // this test point locks down file type check
    fn test_file_type_check() {
        let mut flag_value_list = create_test_flag_value_list(DEFAULT_FILE_VERSION);
        flag_value_list.header.file_type = 123u8;
        let error = FlagValueList::from_bytes(&flag_value_list.into_bytes()).unwrap_err();
        assert!(format!("{:?}", error)
            .starts_with("BytesParseFail(binary file is not a flag value file"));
    }

    #[test]
    fn test_get_offset_for_boolean_flag() {
        let flag_value_list = create_test_flag_value_list(2);
        let header: &FlagValueHeader = &flag_value_list.header;

        let offset = header.get_offset_for_boolean_flag(3).unwrap();

        // See test_utils for these values.
        // Boolean offset is 27 + 3 * 1 = 30.
        assert_eq!(offset, 30usize);
        // 3rd-index boolean is false.
        assert!(!flag_value_list.booleans[3]);
    }

    #[test]
    fn test_get_offset_for_boolean_flag_error() {
        let flag_value_list = create_test_flag_value_list(2);
        let header: &FlagValueHeader = &flag_value_list.header;

        let err = header.get_offset_for_boolean_flag(99);

        assert!(format!("{:?}", err).starts_with(
            "Err(InvalidStorageFileOffset(Flag value offset goes beyond the end of the file."
        ));
    }

    #[test]
    #[cfg(enable_parse_v4)]
    fn test_get_offset_for_int_flag() {
        let flag_value_list = create_test_flag_value_list(4);
        let header: &FlagValueHeader = &flag_value_list.header;

        let offset = header.get_offset_for_int_flag(3).unwrap();

        // See test_utils for these values.
        // Int offset is 43 + 3 * 8 = 67.
        assert_eq!(offset, 67usize);
        // 3rd-index int is 3.
        assert_eq!(flag_value_list.ints[3], 3);
    }

    #[test]
    #[cfg(enable_parse_v4)]
    fn test_get_offset_for_int_flag_error() {
        let flag_value_list = create_test_flag_value_list(4);
        let header: &FlagValueHeader = &flag_value_list.header;

        let err = header.get_offset_for_int_flag(99);

        assert!(format!("{:?}", err).starts_with(
            "Err(InvalidStorageFileOffset(Flag value offset goes beyond the end of the file."
        ));
    }

    #[test]
    #[cfg(not(enable_parse_v4))]
    fn test_get_offset_for_int_flag_v4_disabled() {
        let flag_value_list = create_test_flag_value_list(4);
        let header: &FlagValueHeader = &flag_value_list.header;

        let err = header.get_offset_for_int_flag(3);

        assert!(format!("{:?}", err)
            .starts_with("Err(HigherStorageFileVersion(Version 4 is not supported in this build"));
    }
}
