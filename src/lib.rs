// Copyright 2020 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#![deny(missing_docs)]

//! Provides version tolerant serialization and deserialization facilities and
//! implements a persistent storage format for Firecracker state snapshots.
//! The `Versionize` trait defines a generic interface that serializable state structures
//! need to implement.
//!
//! `VersionMap` exposes an API that maps the individual structure versions to
//! a root version. This mapping is required both when serializing or deserializing structures as
//! it needs to know which version of structure to serialize for a given target data version.
//!
//! The Versionize proc macro supports structures and enums.
//! Supported primitives: u8, u16, u32, u64, usize, i8, i16, i32, i64, isize, char, f32, f64,
//! String, Vec<T>, Arrays up to 32 elements, Box<T>, Wrapping<T>, Option<T>, FamStructWrapper<T>,
//! and (T, U).
//!
//! Known issues and limitations:
//! - Union serialization is not supported via the `Versionize` proc macro.
//! - Implementing Versionize for non-repr(C) unions can result in undefined behaviour
//! and MUST be avoided.
//! - Versionize trait implementations for repr(C) unions must be backed by extensive testing.
//! - Semantic serialization and deserialization is available only for structures.
//!
extern crate bincode;
extern crate crc64;
extern crate serde;
extern crate serde_derive;
extern crate versionize_derive;
extern crate vmm_sys_util;

pub mod crc;
pub mod primitives;
pub mod version_map;

use std::any::TypeId;
use std::io::{Read, Write};
pub use version_map::VersionMap;
use versionize_derive::Versionize;

/// Versioned serialization error definitions.
#[derive(Debug, PartialEq)]
pub enum VersionizeError {
    /// An IO error occured.
    Io(i32),
    /// A serialization error.
    Serialize(String),
    /// A deserialization error.
    Deserialize(String),
    /// A user generated semantic error.
    Semantic(String),
    /// String length exceeded.
    StringLength(usize),
    /// Vector length exceeded.
    VecLength(usize),
}

impl std::fmt::Display for VersionizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        use VersionizeError::*;

        match self {
            Io(e) => write!(f, "An IO error occured: {}", e),
            Serialize(e) => write!(f, "A serialization error occured: {}", e),
            Deserialize(e) => write!(f, "A deserialization error occured: {}", e),
            Semantic(e) => write!(f, "A user generated semantic error occured: {}", e),
            StringLength(bad_len) => write!(
                f,
                "String length exceeded {} > {} bytes",
                bad_len,
                primitives::MAX_STRING_LEN
            ),
            VecLength(bad_len) => write!(
                f,
                "Vec of length {} exceeded maximum size of {} bytes",
                bad_len,
                primitives::MAX_VEC_SIZE
            ),
        }
    }
}

/// Versioned serialization/deserialization result.
pub type VersionizeResult<T> = std::result::Result<T, VersionizeError>;

/// Trait that provides an interface for version aware serialization and deserialization.
pub trait Versionize {
    /// Serializes `self` to `target_verion` using the specficifed `writer` and `version_map`.
    fn serialize<W: Write>(
        &self,
        writer: &mut W,
        version_map: &VersionMap,
        target_version: u16,
    ) -> VersionizeResult<()>;

    /// Returns a new instance of `Self` by deserialzing from `source_version` using the
    /// specficifed `reader` and `version_map`.
    fn deserialize<R: Read>(
        reader: &mut R,
        version_map: &VersionMap,
        source_version: u16,
    ) -> VersionizeResult<Self>
    where
        Self: Sized;

    /// Returns the `Self` type id.
    fn type_id() -> std::any::TypeId
    where
        Self: 'static,
    {
        TypeId::of::<Self>()
    }

    /// Returns latest `Self` version number.
    fn version() -> u16;
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_error_debug_display() {
        // Validates Debug and Display are implemented.
        use VersionizeError::*;
        let str = String::from("test");
        format!("{:?}{}", Io(0), Io(0));
        format!("{:?}{}", Serialize(str.clone()), Serialize(str.clone()));
        format!("{:?}{}", Deserialize(str.clone()), Deserialize(str.clone()));
        format!("{:?}{}", Semantic(str.clone()), Semantic(str));
    }
}
