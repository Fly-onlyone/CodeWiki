//! Source parsing and symbol extraction for CodeWiki.

pub mod c;
pub mod cdeps;
pub mod cfamily;
pub mod cmake;
pub mod csharp;
pub mod go;
pub mod gomod;
pub mod gradle;
pub mod java;
pub mod kotlin;
pub mod lang;
pub mod python;
pub mod resolve;
pub mod rust;
pub mod scan;
pub mod typescript;
pub mod workspace;

pub use c::{
    CInput, ENTRY_NAMES, IMPL_ATTR, Include, TEST_ATTR, TEST_FRAMEWORK_ATTR, TEST_NAME_ATTR,
    parse_c,
};
pub use cfamily::{c_module_map, is_header as is_c_header};
pub use csharp::parse_csharp;
pub use go::{GENERATED_ATTR, is_exported, package_name, parse_go};
pub use gomod::{
    GoMod, GoModules, GoReplace, GoRequire, go_sum_versions, guess_package_name, parse_go_mod,
    parse_go_work,
};
pub use gradle::{CatalogLibrary, parse_version_catalog};
pub use java::parse_java;
pub use kotlin::{EXTENSION_ATTR, KOTEST_ATTR, parse_kotlin};
pub use lang::{python_module, resolve_ts_import, ts_module};
pub use python::{MAIN_GUARD_ATTR, parse_python};
pub use resolve::{ExternalUse, Resolver};
pub use rust::{CallSite, ImplFact, ModDecl, ParsedFile, TypeRef, UseImport, ValueRef, parse_rust};
pub use scan::{RepoFacts, list_files, scan_repo};
pub use typescript::parse_typescript;
pub use workspace::{CrateInfo, DepSpec, Locked, module_path, owning_crate};
