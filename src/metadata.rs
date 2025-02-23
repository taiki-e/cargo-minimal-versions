// SPDX-License-Identifier: Apache-2.0 OR MIT

// Adapted from https://github.com/taiki-e/cargo-hack

use std::{collections::HashMap, ffi::OsStr, path::PathBuf};

use anyhow::{Context as _, Result, format_err};
use serde_json::{Map, Value};

type Object = Map<String, Value>;
type ParseResult<T> = Result<T, &'static str>;

/// An opaque unique identifier for referring to the package.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct PackageId {
    repr: String,
}

impl From<String> for PackageId {
    fn from(repr: String) -> Self {
        Self { repr }
    }
}

pub(crate) struct Metadata {
    pub(crate) cargo_version: u32,
    /// List of all packages in the workspace and all feature-enabled dependencies.
    //
    /// This doesn't contain dependencies if cargo-metadata is run with --no-deps.
    pub(crate) packages: HashMap<PackageId, Package>,
    /// List of members of the workspace.
    pub(crate) workspace_members: Vec<PackageId>,
    /// The absolute path to the root of the workspace.
    pub(crate) workspace_root: PathBuf,
}

impl Metadata {
    pub(crate) fn new(
        manifest_path: Option<&str>,
        cargo: &OsStr,
        cargo_version: u32,
    ) -> Result<Self> {
        let mut cmd = cmd!(cargo, "metadata", "--format-version=1", "--no-deps");
        if let Some(manifest_path) = manifest_path {
            cmd.arg("--manifest-path");
            cmd.arg(manifest_path);
        }
        let json = cmd.read()?;

        let map = serde_json::from_str(&json)
            .with_context(|| format!("failed to parse output from {cmd}"))?;
        Self::from_obj(map, cargo_version)
            .map_err(|s| format_err!("failed to parse `{s}` field from metadata"))
    }

    fn from_obj(mut map: Object, cargo_version: u32) -> ParseResult<Self> {
        let workspace_members: Vec<_> = map
            .remove_array("workspace_members")?
            .into_iter()
            .map(|v| into_string(v).ok_or("workspace_members"))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            cargo_version,
            packages: map
                .remove_array("packages")?
                .into_iter()
                .map(|v| Package::from_value(v, cargo_version))
                .collect::<Result<_, _>>()?,
            workspace_members,
            workspace_root: map.remove_string("workspace_root")?,
        })
    }
}

pub(crate) struct Package {
    /// Absolute path to this package's manifest.
    pub(crate) manifest_path: PathBuf,
    /// List of registries to which this package may be published.
    ///
    /// This is always `true` if running with a version of Cargo older than 1.39.
    pub(crate) publish: bool,
}

impl Package {
    fn from_value(mut value: Value, cargo_version: u32) -> ParseResult<(PackageId, Self)> {
        let map = value.as_object_mut().ok_or("packages")?;

        let id = map.remove_string("id")?;
        Ok((id, Self {
            manifest_path: map.remove_string("manifest_path")?,
            // This field was added in Rust 1.39.
            publish: if cargo_version >= 39 {
                // Publishing is unrestricted if null, and forbidden if an empty array.
                map.remove_nullable("publish", into_array)?.map_or(true, |a| !a.is_empty())
            } else {
                true
            },
        }))
    }
}

#[allow(clippy::option_option)]
fn allow_null<T>(value: Value, f: impl FnOnce(Value) -> Option<T>) -> Option<Option<T>> {
    if value.is_null() { Some(None) } else { f(value).map(Some) }
}

fn into_string<S: From<String>>(value: Value) -> Option<S> {
    if let Value::String(string) = value { Some(string.into()) } else { None }
}
fn into_array(value: Value) -> Option<Vec<Value>> {
    if let Value::Array(array) = value { Some(array) } else { None }
}
// fn into_object(value: Value) -> Option<Object> {
//     if let Value::Object(object) = value {
//         Some(object)
//     } else {
//         None
//     }
// }

trait ObjectExt {
    fn remove_string<S: From<String>>(&mut self, key: &'static str) -> ParseResult<S>;
    fn remove_array(&mut self, key: &'static str) -> ParseResult<Vec<Value>>;
    // fn remove_object(&mut self, key: &'static str) -> ParseResult<Object>;
    fn remove_nullable<T>(
        &mut self,
        key: &'static str,
        f: impl FnOnce(Value) -> Option<T>,
    ) -> ParseResult<Option<T>>;
}

impl ObjectExt for Object {
    fn remove_string<S: From<String>>(&mut self, key: &'static str) -> ParseResult<S> {
        self.remove(key).and_then(into_string).ok_or(key)
    }
    fn remove_array(&mut self, key: &'static str) -> ParseResult<Vec<Value>> {
        self.remove(key).and_then(into_array).ok_or(key)
    }
    // fn remove_object(&mut self, key: &'static str) -> ParseResult<Object> {
    //     self.remove(key).and_then(into_object).ok_or(key)
    // }
    fn remove_nullable<T>(
        &mut self,
        key: &'static str,
        f: impl FnOnce(Value) -> Option<T>,
    ) -> ParseResult<Option<T>> {
        self.remove(key).and_then(|v| allow_null(v, f)).ok_or(key)
    }
}
