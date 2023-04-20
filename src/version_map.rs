// Copyright 2020 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! A helper to map struct and enum versions to a sequence of root versions.
//! This helper is required to support the versioning of a hierarchy of
//! structures composed of individually versioned structures or enums.
//!
//! ```rust
//! extern crate versionize;
//! extern crate versionize_derive;
//!
//! use versionize::{VersionMap, Versionize, VersionizeResult};
//! use versionize_derive::Versionize;
//!
//! #[derive(Versionize)]
//! pub struct Struct1 {
//!     a: u32,
//!     #[version(start = 2)]
//!     b: u8,
//! }
//!
//! #[derive(Versionize)]
//! pub struct Struct2 {
//!     x: u32,
//!     #[version(start = 2)]
//!     y: u8,
//! }
//!
//! #[derive(Versionize)]
//! pub struct State {
//!     struct1: Struct1,
//!     struct2: Struct2,
//! }
//!
//! let mut version_map = VersionMap::new(); //
//! version_map
//!     .new_version()
//!     .set_type_version(Struct1::type_id(), 2)
//!     .new_version()
//!     .set_type_version(Struct2::type_id(), 2);
//!
//! // Check that there are 3 root versions.
//! assert_eq!(version_map.latest_version(), 3);
//!
//! // Check that root version 1 has all structs at version 1.
//! assert_eq!(version_map.get_type_version(1, Struct1::type_id()), 1);
//! assert_eq!(version_map.get_type_version(1, Struct2::type_id()), 1);
//! assert_eq!(version_map.get_type_version(1, State::type_id()), 1);
//!
//! // Check that root version 2 has Struct1 at version 2 and Struct2
//! // at version 1.
//! assert_eq!(version_map.get_type_version(2, Struct1::type_id()), 2);
//! assert_eq!(version_map.get_type_version(2, Struct2::type_id()), 1);
//! assert_eq!(version_map.get_type_version(2, State::type_id()), 1);
//!
//! // Check that root version 3 has Struct1 and Struct2 at version 2.
//! assert_eq!(version_map.get_type_version(3, Struct1::type_id()), 2);
//! assert_eq!(version_map.get_type_version(3, Struct2::type_id()), 2);
//! assert_eq!(version_map.get_type_version(3, State::type_id()), 1);
//! ```

use std::collections::HashMap;
use std::fmt::Debug;

use crate::{Versionize, VersionizeError, VersionizeResult};

pub const MAX_VERSION_NUM: u64 = u16::MAX as u64;

///
/// The VersionMap API provides functionality to define the version for each
/// type and attach them to specific root versions.
#[derive(Clone, Debug)]
pub struct VersionMap {
    crates: HashMap<String, semver::Version>,
}

impl Default for VersionMap {
    fn default() -> Self {
        VersionMap {
            crates: HashMap::new(),
        }
    }
}

impl VersionMap {
    /// Create a new version map initialized at version 1.
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get_crate_version(&self, crate_name: &str) -> VersionizeResult<semver::Version> {
        self.crates
            .get(crate_name)
            .ok_or(VersionizeError::NotFoundCrate(crate_name.to_string()))
            .cloned()
    }

    pub fn set_crate_version(
        &mut self,
        crate_name: &str,
        ver: &str,
    ) -> VersionizeResult<semver::Version> {
        let sem_ver = semver::Version::parse(ver)
            .map_err(|e| VersionizeError::ParseVersion(ver.to_string(), e.to_string()))?;

        if let Some(exist) = self.crates.get(crate_name) {
            if *exist != sem_ver {
                return Err(VersionizeError::MultipleVersion(
                    crate_name.to_string(),
                    exist.to_string(),
                    ver.to_string(),
                ));
            }
        } else {
            self.crates.insert(crate_name.to_owned(), sem_ver.clone());
        }

        Ok(sem_ver)
    }
}

impl Versionize for semver::Version {
    fn serialize<W: std::io::Write>(
        &self,
        mut writer: W,
        _version_map: &mut VersionMap,
    ) -> VersionizeResult<()> {
        // Only support release version.
        if !self.pre.is_empty() || !self.build.is_empty() {
            return Err(VersionizeError::UnsuportVersion(self.to_string()));
        }
        // To reduce snapshot size, only u16::MAX is supported, which should be enough.
        if self.major > MAX_VERSION_NUM
            || self.minor > MAX_VERSION_NUM
            || self.patch > MAX_VERSION_NUM
        {
            return Err(VersionizeError::UnsuportVersion(self.to_string()));
        }
        bincode::serialize_into(&mut writer, &(self.major as u16))
            .map_err(|err| VersionizeError::Serialize(format!("{:?}", err)))?;
        bincode::serialize_into(&mut writer, &(self.minor as u16))
            .map_err(|err| VersionizeError::Serialize(format!("{:?}", err)))?;
        bincode::serialize_into(&mut writer, &(self.patch as u16))
            .map_err(|err| VersionizeError::Serialize(format!("{:?}", err)))?;
        Ok(())
    }

    fn deserialize<R: std::io::Read>(
        mut reader: R,
        _version_map: &VersionMap,
    ) -> VersionizeResult<Self>
    where
        Self: Sized,
    {
        let major: u16 = bincode::deserialize_from(&mut reader)
            .map_err(|err| VersionizeError::Deserialize(format!("{:?}", err)))?;
        let minor: u16 = bincode::deserialize_from(&mut reader)
            .map_err(|err| VersionizeError::Deserialize(format!("{:?}", err)))?;
        let patch: u16 = bincode::deserialize_from(&mut reader)
            .map_err(|err| VersionizeError::Deserialize(format!("{:?}", err)))?;
        Ok(semver::Version::new(
            major as u64,
            minor as u64,
            patch as u64,
        ))
    }
}

#[cfg(test)]
mod tests {
    use byteorder::{NativeEndian, ReadBytesExt};

    use super::*;
    use crate::{Versionize, VersionizeError};

    #[test]
    fn test_ser_de_semver_err() {
        let mut vm = VersionMap::new();
        let mut snapshot_mem = vec![0u8; 48];
        let sem_ver = semver::Version::new(1, 1, MAX_VERSION_NUM + 1);
        assert_eq!(
            sem_ver
                .serialize(snapshot_mem.as_mut_slice(), &mut vm)
                .unwrap_err(),
            VersionizeError::UnsuportVersion("1.1.65536".to_string())
        );

        let sem_ver = semver::Version::parse("1.0.0-alpha").unwrap();
        assert_eq!(
            sem_ver
                .serialize(snapshot_mem.as_mut_slice(), &mut vm)
                .unwrap_err(),
            VersionizeError::UnsuportVersion("1.0.0-alpha".to_string())
        );

        let sem_ver = semver::Version::parse("1.0.0+alpha").unwrap();
        assert_eq!(
            sem_ver
                .serialize(snapshot_mem.as_mut_slice(), &mut vm)
                .unwrap_err(),
            VersionizeError::UnsuportVersion("1.0.0+alpha".to_string())
        );
    }

    #[test]
    fn test_ser_de_semver() {
        let mut vm = VersionMap::new();
        let mut snapshot_mem = vec![0u8; 6];
        let sem_ver = semver::Version::new(3, 0, 14);
        sem_ver
            .serialize(&mut snapshot_mem.as_mut_slice(), &mut vm)
            .unwrap();

        assert_eq!(3, (&snapshot_mem[..2]).read_u16::<NativeEndian>().unwrap());
        assert_eq!(0, (&snapshot_mem[2..4]).read_u16::<NativeEndian>().unwrap());
        assert_eq!(
            14,
            (&snapshot_mem[4..6]).read_u16::<NativeEndian>().unwrap()
        );

        let de_ver: semver::Version =
            Versionize::deserialize(snapshot_mem.as_slice(), &vm).unwrap();
        assert_eq!(de_ver, semver::Version::parse("3.0.14").unwrap());
    }
}
