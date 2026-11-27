//! Test map and impacted tests on a fixture workspace.

mod support;

use std::fs;
use std::path::Path;

use cw_index::Index;
use cw_wiki::service;
use cw_wiki::testmap::{self, TestKind};

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// `util.rs` (unit tests) <- `area.rs` <- `tests/area.rs` and `app/main.rs`
/// (unit test); `lonely.rs` is used and tested by nothing.
fn fixture() -> (tempfile::TempDir, Index) {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(r, "Cargo.toml", "[workspace]\nmembers = [\"crates/*\"]\n");
    write(r, "crates/lib/Cargo.toml", "[package]\nname = \"shapes\"\n");
    write(
        r,
        "crates/lib/src/lib.rs",
        "pub mod area;\npub mod lonely;\npub mod util;\n",
    );
    write(
        r,
        "crates/lib/src/util.rs",
        "pub fn square(x: f64) -> f64 { x * x }\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n\n    #[test]\n    fn squares() {\n        assert_eq!(square(3.0), 9.0);\n    }\n}\n",
    );
    write(
        r,
        "crates/lib/src/area.rs",
        "use crate::util::square;\n\npub fn of_square(side: f64) -> f64 { square(side) }\n",
    );
    write(r, "crates/lib/src/lonely.rs", "pub fn nobody() {}\n");
    write(
        r,
        "crates/lib/tests/area.rs",
        "use shapes::area::of_square;\n\n#[test]\nfn square_area() {\n    assert_eq!(of_square(2.0), 4.0);\n}\n\n#[test]\nfn zero_side() {\n    assert_eq!(of_square(0.0), 0.0);\n}\n\nfn helper() {}\n",
    );
    write(
        r,
        "crates/app/Cargo.toml",
        "[package]\nname = \"app\"\n[dependencies]\nshapes = { path = \"../lib\" }\n",
    );
    write(
        r,
        "crates/app/src/main.rs",
        "use shapes::area::of_square;\n\nfn main() {\n    let _ = of_square(1.0);\n}\n\n#[cfg(test)]\nmod tests {\n    #[tokio::test]\n    async fn runs() {}\n}\n",
    );
    let mut index = Index::open_in_memory().unwrap();
    support::no_model_server();
    service::reindex(r, &mut index).unwrap();
    (dir, index)
}

#[test]
fn maps_tests_to_the_files_they_reach() {
    let (dir, index) = fixture();
    let map = testmap::test_map(dir.path(), &index).unwrap();

    assert_eq!(map.tests_total, 4);
    let found: Vec<(&str, TestKind, Vec<&str>)> = map
        .test_files
        .iter()
        .map(|t| {
            (
                t.path.as_str(),
                t.kind,
                t.tests.iter().map(|c| c.name.as_str()).collect(),
            )
        })
        .collect();
    assert_eq!(
        found,
        [
            ("crates/app/src/main.rs", TestKind::Unit, vec!["runs"]),
            ("crates/lib/src/util.rs", TestKind::Unit, vec!["squares"]),
            (
                "crates/lib/tests/area.rs",
                TestKind::Integration,
                vec!["square_area", "zero_side"]
            ),
        ]
    );
    assert_eq!(map.test_files[1].tests[0].line, 8);
    assert_eq!(
        map.test_files[2].command,
        "cargo test -p shapes --test area"
    );

    let file = |path: &str| map.files.iter().find(|f| f.path == path).unwrap();
    let area = file("crates/lib/src/area.rs");
    assert_eq!(area.unit_tests, 0);
    assert_eq!(
        area.direct,
        ["crates/app/src/main.rs", "crates/lib/tests/area.rs"]
    );
    let util = file("crates/lib/src/util.rs");
    assert_eq!(util.unit_tests, 1);
    assert!(util.direct.is_empty(), "{util:?}");
    assert_eq!(
        util.indirect,
        ["crates/app/src/main.rs", "crates/lib/tests/area.rs"]
    );
    // Test files are not listed as code to be tested.
    assert!(map.files.iter().all(|f| !f.path.contains("/tests/")));
    // `lib.rs` only declares modules, so it is not reported.
    assert_eq!(map.untested, ["crates/lib/src/lonely.rs"]);
}

#[test]
fn lists_the_tests_a_change_can_break() {
    let (dir, index) = fixture();
    let changed = ["crates\\lib\\src\\util.rs".to_string()];
    let impact = testmap::impacted_tests(dir.path(), &index, &changed).unwrap();

    assert_eq!(impact.changed, ["crates/lib/src/util.rs"]);
    let rows: Vec<(&str, u32)> = impact
        .test_files
        .iter()
        .map(|t| (t.path.as_str(), t.distance))
        .collect();
    assert_eq!(
        rows,
        [
            ("crates/lib/src/util.rs", 0),
            ("crates/app/src/main.rs", 2),
            ("crates/lib/tests/area.rs", 2),
        ],
        "nearest first"
    );
    assert_eq!(impact.tests_total, 4);
    assert_eq!(
        impact.commands,
        [
            "cargo test -p shapes --lib util::",
            "cargo test -p app --bins",
            "cargo test -p shapes --test area",
        ]
    );
    assert_eq!(impact.test_files[2].tests, ["square_area", "zero_side"]);
    assert_eq!(impact.test_files[2].because, ["crates/lib/src/util.rs"]);
    assert!(impact.untested_changes.is_empty());

    // A change nothing depends on, a changed test file, and a non-source file.
    let changed = [
        "crates/lib/src/lonely.rs".to_string(),
        "crates/lib/tests/area.rs".to_string(),
        "README.md".to_string(),
    ];
    let impact = testmap::impacted_tests(dir.path(), &index, &changed).unwrap();
    assert_eq!(impact.test_files.len(), 1);
    assert_eq!(impact.test_files[0].path, "crates/lib/tests/area.rs");
    assert_eq!(impact.test_files[0].distance, 0);
    assert_eq!(impact.untested_changes, ["crates/lib/src/lonely.rs"]);

    let none = testmap::impacted_tests(dir.path(), &index, &[]).unwrap();
    assert!(none.test_files.is_empty() && none.commands.is_empty());
}

/// Two test projects over one library: `Core.Tests` runs under VSTest, where
/// `--filter` names classes, `Mtp.Tests` has no VSTest host and runs whole.
fn csharp_fixture() -> (tempfile::TempDir, Index) {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(r, "Acme.slnx", "<Solution />\n");
    write(
        r,
        "src/Core/Core.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\" />\n",
    );
    write(
        r,
        "src/Core/Helper.cs",
        "namespace Acme.Core;\n\npublic static class Helper\n{\n    public static int Twice(int x) => x * 2;\n}\n",
    );
    write(
        r,
        "src/Core/Engine.cs",
        "namespace Acme.Core;\n\npublic class Engine\n{\n    public int Run() => Helper.Twice(2);\n}\n",
    );
    write(
        r,
        "src/Core/Lonely.cs",
        "namespace Acme.Core;\n\npublic class Lonely\n{\n    public void Nobody() { }\n}\n",
    );
    write(
        r,
        "tests/Core.Tests/Core.Tests.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\"><ItemGroup><ProjectReference Include=\"..\\..\\src\\Core\\Core.csproj\" /><PackageReference Include=\"Microsoft.NET.Test.Sdk\" /><PackageReference Include=\"xunit.v3\" /></ItemGroup></Project>\n",
    );
    write(
        r,
        "tests/Core.Tests/EngineTests.cs",
        "using Acme.Core;\n\nnamespace Acme.Core.Tests;\n\npublic class EngineTests\n{\n    [Fact]\n    public void Runs() { Assert.Equal(4, new Engine().Run()); }\n\n    [Theory]\n    [InlineData(1)]\n    public void Repeats(int n) { new Engine().Run(); }\n\n    public void NotATest() { }\n}\n",
    );
    write(
        r,
        "tests/Core.Tests/HelperTests.cs",
        "using Acme.Core;\n\nnamespace Acme.Core.Tests;\n\npublic class HelperTests\n{\n    [Fact]\n    public void Doubles() { Assert.Equal(4, Helper.Twice(2)); }\n}\n",
    );
    write(
        r,
        "tests/Mtp.Tests/Mtp.Tests.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\"><ItemGroup><ProjectReference Include=\"..\\..\\src\\Core\\Core.csproj\" /><PackageReference Include=\"xunit.v3\" /></ItemGroup></Project>\n",
    );
    write(
        r,
        "tests/Mtp.Tests/EngineMtpTests.cs",
        "using Acme.Core;\n\nnamespace Acme.Mtp.Tests;\n\npublic class EngineMtpTests\n{\n    [Fact]\n    public void Runs() { new Engine().Run(); }\n}\n",
    );
    let mut index = Index::open_in_memory().unwrap();
    support::no_model_server();
    service::reindex(r, &mut index).unwrap();
    (dir, index)
}

#[test]
fn maps_csharp_tests_and_the_command_that_runs_them() {
    let (dir, index) = csharp_fixture();
    let map = testmap::test_map(dir.path(), &index).unwrap();

    assert_eq!(map.tests_total, 4);
    let found: Vec<(&str, TestKind, Vec<&str>)> = map
        .test_files
        .iter()
        .map(|t| {
            (
                t.path.as_str(),
                t.kind,
                t.tests.iter().map(|c| c.name.as_str()).collect(),
            )
        })
        .collect();
    assert_eq!(
        found,
        [
            (
                "tests/Core.Tests/EngineTests.cs",
                TestKind::Integration,
                vec!["EngineTests.Runs", "EngineTests.Repeats"]
            ),
            (
                "tests/Core.Tests/HelperTests.cs",
                TestKind::Integration,
                vec!["HelperTests.Doubles"]
            ),
            (
                "tests/Mtp.Tests/EngineMtpTests.cs",
                TestKind::Integration,
                vec!["EngineMtpTests.Runs"]
            ),
        ]
    );
    assert_eq!(map.test_files[0].tests[0].line, 7);
    // VSTest: the class of the file; no VSTest host: the whole project.
    assert_eq!(
        map.test_files[0].command,
        "dotnet test tests/Core.Tests/Core.Tests.csproj --filter \"FullyQualifiedName~Acme.Core.Tests.EngineTests\""
    );
    assert_eq!(
        map.test_files[2].command,
        "dotnet test tests/Mtp.Tests/Mtp.Tests.csproj"
    );

    // Test files are no source files; the library files are.
    let paths: Vec<&str> = map.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "src/Core/Engine.cs",
            "src/Core/Helper.cs",
            "src/Core/Lonely.cs"
        ]
    );
    let helper = map
        .files
        .iter()
        .find(|f| f.path.ends_with("Helper.cs"))
        .unwrap();
    assert_eq!(helper.direct, ["tests/Core.Tests/HelperTests.cs"]);
    assert_eq!(
        helper.indirect,
        [
            "tests/Core.Tests/EngineTests.cs",
            "tests/Mtp.Tests/EngineMtpTests.cs"
        ]
    );
    assert_eq!(map.untested, ["src/Core/Lonely.cs"]);
}

#[test]
fn lists_the_csharp_tests_a_change_can_break() {
    let (dir, index) = csharp_fixture();
    let changed = ["src\\Core\\Helper.cs".to_string()];
    let impact = testmap::impacted_tests(dir.path(), &index, &changed).unwrap();

    assert_eq!(impact.changed, ["src/Core/Helper.cs"]);
    let rows: Vec<(&str, u32)> = impact
        .test_files
        .iter()
        .map(|t| (t.path.as_str(), t.distance))
        .collect();
    assert_eq!(
        rows,
        [
            ("tests/Core.Tests/HelperTests.cs", 1),
            ("tests/Core.Tests/EngineTests.cs", 2),
            ("tests/Mtp.Tests/EngineMtpTests.cs", 2),
        ],
        "nearest first"
    );
    assert_eq!(impact.tests_total, 4);
    // One run for each project, over the classes of every file it holds.
    assert_eq!(
        impact.commands,
        [
            "dotnet test tests/Core.Tests/Core.Tests.csproj --filter \"FullyQualifiedName~Acme.Core.Tests.HelperTests|FullyQualifiedName~Acme.Core.Tests.EngineTests\"",
            "dotnet test tests/Mtp.Tests/Mtp.Tests.csproj",
        ]
    );
    assert!(impact.untested_changes.is_empty());

    // A change nothing depends on, and a changed test file of a project with
    // no class filter: its run is the project.
    let changed = [
        "src/Core/Lonely.cs".to_string(),
        "tests/Mtp.Tests/EngineMtpTests.cs".to_string(),
    ];
    let impact = testmap::impacted_tests(dir.path(), &index, &changed).unwrap();
    assert_eq!(impact.test_files.len(), 1);
    assert_eq!(impact.test_files[0].distance, 0);
    assert_eq!(impact.untested_changes, ["src/Core/Lonely.cs"]);
    assert_eq!(
        impact.commands,
        ["dotnet test tests/Mtp.Tests/Mtp.Tests.csproj"]
    );
}

/// A Go module with a package that has an in-package test file and an external
/// test package, a package that imports it, one with ten tests, one with
/// none, and a second module in a folder of its own.
fn go_fixture() -> (tempfile::TempDir, Index) {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(r, "go.mod", "module example.com/app\n\ngo 1.22\n");
    write(
        r,
        "store/store.go",
        "package store\n\ntype Store struct{ items map[string]string }\n\nfunc New() *Store { return &Store{items: map[string]string{}} }\n\nfunc (s *Store) Get(key string) string { return s.items[key] }\n",
    );
    write(
        r,
        "store/extra.go",
        "package store\n\nfunc Extra() int { return 1 }\n",
    );
    write(
        r,
        "store/store_test.go",
        "package store\n\nimport \"testing\"\n\nfunc TestMain(m *testing.M) { m.Run() }\n\nfunc TestNew(t *testing.T) { New() }\n\nfunc TestGet(t *testing.T) { New().Get(\"a\") }\n\nfunc testHelper(t *testing.T) {}\n\nfunc Testimony(t *testing.T) {}\n\nfunc BenchmarkNew(b *testing.B) { New() }\n\nfunc ExampleNew() { New() }\n\nfunc FuzzGet(f *testing.F) { New() }\n",
    );
    write(
        r,
        "store/cache_test.go",
        "package store_test\n\nimport (\n\t\"testing\"\n\n\t\"example.com/app/store\"\n)\n\nfunc TestCache(t *testing.T) { store.New() }\n",
    );
    write(
        r,
        "words/words.go",
        "package words\n\nimport \"example.com/app/store\"\n\nfunc Shout() { store.New() }\n",
    );
    write(
        r,
        "words/words_test.go",
        "package words\n\nimport \"testing\"\n\nfunc TestShout(t *testing.T) { Shout() }\n",
    );
    write(
        r,
        "lonely/lonely.go",
        "package lonely\n\nfunc Nobody() {}\n",
    );
    write(r, "many/many.go", "package many\n\nfunc Many() {}\n");
    let mut many = String::from("package many\n\nimport \"testing\"\n");
    for n in 0..10 {
        many.push_str(&format!("\nfunc TestMany{n}(t *testing.T) {{ Many() }}\n"));
    }
    write(r, "many/many_test.go", &many);
    write(r, "tools/gen/go.mod", "module example.com/gen\n");
    write(r, "tools/gen/gen.go", "package gen\n\nfunc Gen() {}\n");
    write(
        r,
        "tools/gen/gen_test.go",
        "package gen\n\nimport \"testing\"\n\nfunc TestGen(t *testing.T) { Gen() }\n\nfunc BenchmarkGen(b *testing.B) { Gen() }\n",
    );
    write(
        r,
        "tools/gen/bench_test.go",
        "package gen\n\nimport \"testing\"\n\nfunc BenchmarkOnly(b *testing.B) { Gen() }\n",
    );
    let mut index = Index::open_in_memory().unwrap();
    support::no_model_server();
    service::reindex(r, &mut index).unwrap();
    (dir, index)
}

#[test]
fn maps_go_tests_and_the_command_that_runs_them() {
    let (dir, index) = go_fixture();
    let map = testmap::test_map(dir.path(), &index).unwrap();

    let found: Vec<(&str, TestKind, Vec<&str>)> = map
        .test_files
        .iter()
        .map(|t| {
            (
                t.path.as_str(),
                t.kind,
                t.tests.iter().map(|c| c.name.as_str()).collect(),
            )
        })
        .collect();
    // `TestMain`, a lower-case helper and `Testimony` are no tests; benchmarks,
    // fuzz targets and examples are.
    let many: Vec<String> = (0..10).map(|n| format!("TestMany{n}")).collect();
    assert_eq!(found.len(), 6);
    assert_eq!(
        found[0],
        (
            "many/many_test.go",
            TestKind::Integration,
            many.iter().map(String::as_str).collect()
        )
    );
    assert_eq!(
        found[1],
        (
            "store/cache_test.go",
            TestKind::Integration,
            vec!["TestCache"]
        )
    );
    assert_eq!(
        found[2],
        (
            "store/store_test.go",
            TestKind::Integration,
            vec![
                "TestNew",
                "TestGet",
                "BenchmarkNew",
                "ExampleNew",
                "FuzzGet"
            ]
        )
    );
    let command = |path: &str| {
        map.test_files
            .iter()
            .find(|t| t.path == path)
            .map(|t| t.command.as_str())
            .unwrap()
    };
    assert_eq!(
        command("store/store_test.go"),
        "go test ./store -run '^(TestNew|TestGet|ExampleNew|FuzzGet)$' -bench '^(BenchmarkNew)$'"
    );
    assert_eq!(
        command("store/cache_test.go"),
        "go test ./store -run '^(TestCache)$'"
    );
    // A module below the root runs from its own folder.
    assert_eq!(
        command("tools/gen/gen_test.go"),
        "go test -C tools/gen . -run '^(TestGen)$' -bench '^(BenchmarkGen)$'"
    );
    // A file of benchmarks alone runs no test.
    assert_eq!(
        command("tools/gen/bench_test.go"),
        "go test -C tools/gen . -run '^$' -bench '^(BenchmarkOnly)$'"
    );

    // Test files are no source files; a package's tests reach every file of it.
    let paths: Vec<&str> = map.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "lonely/lonely.go",
            "many/many.go",
            "store/extra.go",
            "store/store.go",
            "tools/gen/gen.go",
            "words/words.go"
        ]
    );
    let extra = map
        .files
        .iter()
        .find(|f| f.path == "store/extra.go")
        .unwrap();
    assert_eq!(extra.direct, ["store/cache_test.go", "store/store_test.go"]);
    let store = map
        .files
        .iter()
        .find(|f| f.path == "store/store.go")
        .unwrap();
    assert_eq!(store.indirect, ["words/words_test.go"]);
    assert_eq!(map.untested, ["lonely/lonely.go"]);
}

#[test]
fn lists_the_go_tests_a_change_can_break() {
    let (dir, index) = go_fixture();
    let changed = ["store\\store.go".to_string()];
    let impact = testmap::impacted_tests(dir.path(), &index, &changed).unwrap();

    assert_eq!(impact.changed, ["store/store.go"]);
    let rows: Vec<(&str, u32)> = impact
        .test_files
        .iter()
        .map(|t| (t.path.as_str(), t.distance))
        .collect();
    assert_eq!(
        rows,
        [
            ("store/cache_test.go", 1),
            ("store/store_test.go", 1),
            ("words/words_test.go", 2),
        ],
        "nearest first"
    );
    // One run for each package, over the tests of every file of it.
    assert_eq!(
        impact.commands,
        [
            "go test ./store -run '^(TestCache|TestNew|TestGet|ExampleNew|FuzzGet)$' -bench '^(BenchmarkNew)$'",
            "go test ./words -run '^(TestShout)$'",
        ]
    );
    assert!(impact.untested_changes.is_empty());

    // Many tests in one package: the package runs whole.
    let impact =
        testmap::impacted_tests(dir.path(), &index, &["many/many.go".to_string()]).unwrap();
    assert_eq!(impact.tests_total, 10);
    assert_eq!(impact.commands, ["go test ./many"]);

    // The module below the root, and the benchmarks of two files folded.
    let impact =
        testmap::impacted_tests(dir.path(), &index, &["tools/gen/gen.go".to_string()]).unwrap();
    assert_eq!(
        impact.commands,
        ["go test -C tools/gen . -run '^(TestGen)$' -bench '^(BenchmarkOnly|BenchmarkGen)$'"]
    );

    // A file no test reaches.
    let impact =
        testmap::impacted_tests(dir.path(), &index, &["lonely/lonely.go".to_string()]).unwrap();
    assert!(impact.test_files.is_empty() && impact.commands.is_empty());
    assert_eq!(impact.untested_changes, ["lonely/lonely.go"]);
}
