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

//! flag info module defines the flag info file format and methods for serialization
//! and deserialization

use crate::{
    read_str_from_bytes, read_u32_from_bytes, read_u8_from_bytes, MAX_SUPPORTED_FILE_VERSION,
};
use crate::{AconfigStorageError, StorageFileType};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Flag info header struct
#[derive(PartialEq, Serialize, Deserialize)]
pub struct FlagInfoHeader {
    pub version: u32,
    pub container: String,
    pub file_type: u8,
    pub file_size: u32,
    pub num_boolean_flags: u32,
    pub boolean_flag_offset: u32,
    pub num_int_flags: u32,
    pub int_flag_offset: u32,
}

/// Implement debug print trait for header
impl fmt::Debug for FlagInfoHeader {
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
            "Num of Boolean Flags: {}, Boolean Flag Offset:{}",
            self.num_boolean_flags, self.boolean_flag_offset
        )?;
        if self.version >= 4 && cfg!(enable_parse_v4) {
            writeln!(
                f,
                "Num of Int Flags: {}, Int Value Offset: {}",
                self.num_int_flags, self.int_flag_offset
            )?;
        }
        Ok(())
    }
}

impl FlagInfoHeader {
    /// Serialize to bytes
    pub fn into_bytes(&self) -> Vec<u8> {
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
        result.extend_from_slice(&self.boolean_flag_offset.to_le_bytes());
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
        result.extend_from_slice(&self.boolean_flag_offset.to_le_bytes());
        result.extend_from_slice(&self.num_int_flags.to_le_bytes());
        result.extend_from_slice(&self.int_flag_offset.to_le_bytes());
        result
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AconfigStorageError> {
        let mut head = 0;
        let version = read_u32_from_bytes(bytes, &mut head)?;
        let header = match version {
            1..=3 => Self::from_bytes_v1(bytes, version, &mut head),
            4 if cfg!(enable_parse_v4) => Self::from_bytes_v4(bytes, version, &mut head),
            _ => {
                return Err(AconfigStorageError::HigherStorageFileVersion(anyhow!(
                    "Cannot read storage file with a higher version of {} with lib version {}",
                    version,
                    MAX_SUPPORTED_FILE_VERSION
                )))
            }
        }?;
        if header.file_type != StorageFileType::FlagInfo as u8 {
            return Err(AconfigStorageError::BytesParseFail(anyhow!(
                "binary file is not a flag info file"
            )));
        }
        Ok(header)
    }

    fn from_bytes_v1(
        bytes: &[u8],
        version: u32,
        head: &mut usize,
    ) -> Result<Self, AconfigStorageError> {
        Ok(Self {
            version,
            container: read_str_from_bytes(bytes, head)?,
            file_type: read_u8_from_bytes(bytes, head)?,
            file_size: read_u32_from_bytes(bytes, head)?,
            num_boolean_flags: read_u32_from_bytes(bytes, head)?,
            boolean_flag_offset: read_u32_from_bytes(bytes, head)?,
            num_int_flags: 0,
            int_flag_offset: 0,
        })
    }

    fn from_bytes_v4(
        bytes: &[u8],
        version: u32,
        head: &mut usize,
    ) -> Result<Self, AconfigStorageError> {
        Ok(Self {
            version,
            container: read_str_from_bytes(bytes, head)?,
            file_type: read_u8_from_bytes(bytes, head)?,
            file_size: read_u32_from_bytes(bytes, head)?,
            num_boolean_flags: read_u32_from_bytes(bytes, head)?,
            boolean_flag_offset: read_u32_from_bytes(bytes, head)?,
            num_int_flags: read_u32_from_bytes(bytes, head)?,
            int_flag_offset: read_u32_from_bytes(bytes, head)?,
        })
    }
}

/// bit field for flag info
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlagInfoBit {
    HasServerOverride = 1 << 0,
    IsReadWrite = 1 << 1,
    HasLocalOverride = 1 << 2,
}

/// Flag info node struct
#[derive(PartialEq, Clone, Serialize, Deserialize)]
pub struct FlagInfoNode {
    pub attributes: u8,
}

/// Implement debug print trait for node
impl fmt::Debug for FlagInfoNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "readwrite: {}, server override: {}, local override: {}",
            self.attributes & (FlagInfoBit::IsReadWrite as u8) != 0,
            self.attributes & (FlagInfoBit::HasServerOverride as u8) != 0,
            self.attributes & (FlagInfoBit::HasLocalOverride as u8) != 0,
        )?;
        Ok(())
    }
}

impl FlagInfoNode {
    /// Serialize to bytes
    pub fn into_bytes(&self) -> Vec<u8> {
        let mut result = Vec::new();
        result.extend_from_slice(&self.attributes.to_le_bytes());
        result
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AconfigStorageError> {
        let mut head = 0;
        let node = Self { attributes: read_u8_from_bytes(bytes, &mut head)? };
        Ok(node)
    }

    /// Create flag info node
    pub fn create(is_flag_rw: bool) -> Self {
        Self { attributes: if is_flag_rw { FlagInfoBit::IsReadWrite as u8 } else { 0u8 } }
    }
}

/// Flag info list struct
#[derive(PartialEq, Serialize, Deserialize)]
pub struct FlagInfoList {
    pub header: FlagInfoHeader,
    pub boolean_nodes: Vec<FlagInfoNode>,
    pub int_nodes: Vec<FlagInfoNode>,
}

/// Implement debug print trait for flag info list
impl fmt::Debug for FlagInfoList {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Header:")?;
        write!(f, "{:?}", self.header)?;
        writeln!(f, "Boolean Flag Info:")?;
        for node in self.boolean_nodes.iter() {
            write!(f, "{node:?}")?;
        }
        if self.header.version >= 4 && cfg!(enable_parse_v4) {
            writeln!(f, "Integer Flag Info:")?;
            for node in self.int_nodes.iter() {
                write!(f, "{node:?}")?;
            }
        }
        Ok(())
    }
}

impl FlagInfoList {
    /// Serialize to bytes
    pub fn into_bytes(&self) -> Vec<u8> {
        [
            self.header.into_bytes(),
            self.boolean_nodes.iter().map(|v| v.into_bytes()).collect::<Vec<_>>().concat(),
            self.int_nodes.iter().map(|v| v.into_bytes()).collect::<Vec<_>>().concat(),
        ]
        .concat()
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AconfigStorageError> {
        let header = FlagInfoHeader::from_bytes(bytes)?;
        let mut boolean_info = Vec::new();
        let mut int_info = Vec::new();

        let mut head = header.boolean_flag_offset as usize;
        for _ in 0..header.num_boolean_flags {
            let node = FlagInfoNode::from_bytes(&bytes[head..])?;
            head += node.into_bytes().len();
            boolean_info.push(node);
        }

        if header.version >= 4 && cfg!(enable_parse_v4) {
            let mut head = header.int_flag_offset as usize;
            for _ in 0..header.num_int_flags {
                let node = FlagInfoNode::from_bytes(&bytes[head..])?;
                head += node.into_bytes().len();
                int_info.push(node);
            }
        }

        let list = Self { header, boolean_nodes: boolean_info, int_nodes: int_info };
        Ok(list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        test_utils::create_test_flag_info_list, DEFAULT_FILE_VERSION, MAX_SUPPORTED_FILE_VERSION,
    };

    // this test point locks down the value list serialization
    #[test]
    fn test_serialization() {
        for file_version in 1..=MAX_SUPPORTED_FILE_VERSION {
            let flag_info_list = create_test_flag_info_list(file_version);

            let header: &FlagInfoHeader = &flag_info_list.header;
            let reinterpreted_header = FlagInfoHeader::from_bytes(&header.into_bytes());
            assert!(reinterpreted_header.is_ok());
            assert_eq!(header, &reinterpreted_header.unwrap());

            let boolean_nodes: &Vec<FlagInfoNode> = &flag_info_list.boolean_nodes;
            for node in boolean_nodes.iter() {
                let reinterpreted_node = FlagInfoNode::from_bytes(&node.into_bytes()).unwrap();
                assert_eq!(node, &reinterpreted_node);
            }

            let int_nodes = &flag_info_list.int_nodes;
            for node in int_nodes.iter() {
                let reinterpreted_node = FlagInfoNode::from_bytes(&node.into_bytes()).unwrap();
                assert_eq!(node, &reinterpreted_node);
            }

            let flag_info_bytes = flag_info_list.into_bytes();
            let reinterpreted_info_list = FlagInfoList::from_bytes(&flag_info_bytes);
            assert!(reinterpreted_info_list.is_ok());
            assert_eq!(&flag_info_list, &reinterpreted_info_list.unwrap());
            assert_eq!(flag_info_bytes.len() as u32, header.file_size);
        }
    }

    // this test point locks down that version number should be at the top of serialized
    // bytes
    #[test]
    fn test_version_number() {
        let flag_info_list = create_test_flag_info_list(DEFAULT_FILE_VERSION);
        let bytes = &flag_info_list.into_bytes();
        let mut head = 0;
        let version_from_file = read_u32_from_bytes(bytes, &mut head).unwrap();
        assert_eq!(version_from_file, DEFAULT_FILE_VERSION);
    }

    // this test point locks down file type check
    #[test]
    fn test_file_type_check() {
        let mut flag_info_list = create_test_flag_info_list(DEFAULT_FILE_VERSION);
        flag_info_list.header.file_type = 123u8;
        let error = FlagInfoList::from_bytes(&flag_info_list.into_bytes()).unwrap_err();
        assert!(format!("{:?}", error)
            .starts_with("BytesParseFail(binary file is not a flag info file"));
    }
}
