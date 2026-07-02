// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::hooks::package_path::read_package_name;
use std::{fs, path::PathBuf};
use tempfile::TempDir;

fn write_manifest(contents: &str) -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let toml = dir.path().join("Move.toml");
    fs::write(&toml, contents).unwrap();
    (dir, toml)
}

#[test]
fn test_read_package_name() {
    let (_dir, toml) = write_manifest("[package]\nname = \"my_package\"\n");
    assert_eq!(read_package_name(&toml), Some("my_package".to_string()));
}

#[test]
fn test_read_package_name_single_quotes() {
    let (_dir, toml) = write_manifest("[package]\nname = 'my_pkg'\n");
    assert_eq!(read_package_name(&toml), Some("my_pkg".to_string()));
}

#[test]
fn test_read_package_name_missing() {
    let (_dir, toml) = write_manifest("[dependencies]\n");
    assert_eq!(read_package_name(&toml), None);
}

#[test]
fn test_read_package_name_tolerant_section_header() {
    let (_dir, toml) = write_manifest("[ package ] # metadata\nname = \"pkg\"\n");
    assert_eq!(read_package_name(&toml), Some("pkg".to_string()));
}

#[test]
fn test_read_package_name_ignores_other_sections() {
    let (_dir, toml) =
        write_manifest("[addresses]\nnamespace = \"wrong\"\n[package]\nname = \"right\"\n");
    assert_eq!(read_package_name(&toml), Some("right".to_string()));
}

#[test]
fn test_read_package_name_strips_inline_comment() {
    let (_dir, toml) = write_manifest("[package]\nname = \"pkg\"  # legacy\n");
    assert_eq!(read_package_name(&toml), Some("pkg".to_string()));
}

#[test]
fn test_read_package_name_no_package_section() {
    let (_dir, toml) = write_manifest("[dependencies]\nsome_dep = {}\n");
    assert_eq!(read_package_name(&toml), None);
}

#[test]
fn test_read_package_name_names_key_not_matched() {
    let (_dir, toml) = write_manifest("[package]\nnames = \"x\"\n");
    assert_eq!(read_package_name(&toml), None);
}
