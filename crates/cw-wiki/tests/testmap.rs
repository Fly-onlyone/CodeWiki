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

/// A Gradle build of three modules with Kotlin and Java side by side, a
/// Kotlin Multiplatform module, a module under a nested build, one with too
/// many test classes, and two Maven builds.
fn jvm_fixture() -> (tempfile::TempDir, Index) {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(
        r,
        "settings.gradle.kts",
        "include(\"core\", \"app\", \"kmp\", \"wide\")\n",
    );
    write(r, "gradlew", "#!/bin/sh\n");
    write(r, "core/build.gradle.kts", "plugins { kotlin(\"jvm\") }\n");
    write(
        r,
        "core/src/main/java/com/acme/core/Money.java",
        "package com.acme.core;\n\npublic class Money {\n    public int cents() { return 1; }\n}\n",
    );
    write(
        r,
        "core/src/main/kotlin/com/acme/core/Prices.kt",
        "package com.acme.core\n\nfun total(m: Money): Int = m.cents() * 2\n",
    );
    write(
        r,
        "core/src/main/kotlin/com/acme/core/Cart.kt",
        "package com.acme.core\n\nclass Cart {\n    fun count(): Int = 0\n}\n",
    );
    write(
        r,
        "core/src/test/kotlin/com/acme/core/PricesTest.kt",
        "package com.acme.core\n\nimport org.junit.jupiter.api.Test\nimport org.junit.jupiter.params.ParameterizedTest\n\nclass PricesTest {\n    @Test\n    fun `totals a price`() {\n        total(Money())\n    }\n\n    @ParameterizedTest\n    fun totalsMany() {\n        total(Money())\n    }\n\n    fun helper() {}\n\n    @org.junit.jupiter.api.Nested\n    inner class Inner {\n        @Test\n        fun nested() {\n            total(Money())\n        }\n    }\n}\n",
    );
    write(
        r,
        "core/src/test/java/com/acme/core/MoneyTest.java",
        "package com.acme.core;\n\nimport org.junit.jupiter.api.RepeatedTest;\nimport org.junit.jupiter.api.Test;\n\nclass MoneyTest {\n    @Test\n    void cents() {\n        new Money().cents();\n    }\n\n    @RepeatedTest(3)\n    void repeated() {\n        new Money().cents();\n    }\n}\n",
    );
    write(
        r,
        "core/src/test/kotlin/com/acme/core/CartSpec.kt",
        "package com.acme.core\n\nimport io.kotest.core.spec.style.StringSpec\n\nclass CartSpec : StringSpec({\n    \"counts\" { Cart().count() }\n})\n",
    );
    write(r, "app/build.gradle.kts", "plugins { kotlin(\"jvm\") }\n");
    write(
        r,
        "app/src/main/kotlin/com/acme/app/App.kt",
        "package com.acme.app\n\nimport com.acme.core.Cart\n\nfun run(): Int = Cart().count()\n",
    );
    write(
        r,
        "app/src/test/kotlin/com/acme/app/AppTest.kt",
        "package com.acme.app\n\nimport kotlin.test.Test\n\nclass AppTest {\n    @Test\n    fun runs() { run() }\n}\n",
    );
    write(
        r,
        "kmp/build.gradle.kts",
        "plugins { kotlin(\"multiplatform\") }\n",
    );
    write(
        r,
        "kmp/src/commonMain/kotlin/com/acme/kmp/Shared.kt",
        "package com.acme.kmp\n\nfun shared(): Int = 1\n\nfun jvmOnly(): Int = 2\n",
    );
    write(
        r,
        "kmp/src/commonTest/kotlin/com/acme/kmp/SharedTest.kt",
        "package com.acme.kmp\n\nimport kotlin.test.Test\n\nclass SharedTest {\n    @Test\n    fun works() { shared() }\n}\n",
    );
    write(
        r,
        "kmp/src/jvmTest/kotlin/com/acme/kmp/JvmOnlyTest.kt",
        "package com.acme.kmp\n\nimport kotlin.test.Test\n\nclass JvmOnlyTest {\n    @Test\n    fun only() { jvmOnly() }\n}\n",
    );
    write(r, "wide/build.gradle", "plugins { id 'java' }\n");
    write(
        r,
        "wide/src/main/java/com/acme/wide/Wide.java",
        "package com.acme.wide;\n\npublic class Wide {\n    public static int wide() { return 1; }\n}\n",
    );
    for n in 0..9 {
        write(
            r,
            &format!("wide/src/test/java/com/acme/wide/Wide{n}Test.java"),
            &format!(
                "package com.acme.wide;\n\nimport org.junit.Test;\n\npublic class Wide{n}Test {{\n    @Test\n    public void wide() {{ Wide.wide(); }}\n}}\n"
            ),
        );
    }
    // A nested Gradle build without a wrapper.
    write(r, "plain/settings.gradle", "include 'lib'\n");
    write(r, "plain/lib/build.gradle", "plugins { id 'java' }\n");
    write(
        r,
        "plain/lib/src/main/java/com/acme/plain/Plain.java",
        "package com.acme.plain;\n\npublic class Plain {\n    public static int plain() { return 1; }\n}\n",
    );
    write(
        r,
        "plain/lib/src/test/java/com/acme/plain/PlainTest.java",
        "package com.acme.plain;\n\nimport org.junit.Test;\n\npublic class PlainTest {\n    @Test\n    public void plain() { Plain.plain(); }\n}\n",
    );
    // A Maven build of one module.
    write(
        r,
        "single/pom.xml",
        "<project><artifactId>single</artifactId></project>\n",
    );
    write(
        r,
        "single/src/main/java/com/acme/single/Calc.java",
        "package com.acme.single;\n\npublic class Calc {\n    public static int add() { return 1; }\n}\n",
    );
    write(
        r,
        "single/src/test/java/com/acme/single/CalcTest.java",
        "package com.acme.single;\n\nimport org.junit.Test;\n\npublic class CalcTest {\n    @Test\n    public void adds() { Calc.add(); }\n}\n",
    );
    // A Maven reactor, and a module of it.
    write(
        r,
        "reactor/pom.xml",
        "<project><modules><module>svc</module></modules></project>\n",
    );
    write(
        r,
        "reactor/svc/pom.xml",
        "<project><artifactId>svc</artifactId></project>\n",
    );
    write(
        r,
        "reactor/svc/src/main/java/com/acme/svc/Svc.java",
        "package com.acme.svc;\n\npublic class Svc {\n    public static int svc() { return 1; }\n}\n",
    );
    write(
        r,
        "reactor/svc/src/test/java/com/acme/svc/SvcTest.java",
        "package com.acme.svc;\n\nimport org.junit.jupiter.api.Test;\n\nclass SvcTest {\n    @Test\n    void svc() { Svc.svc(); }\n\n    @Test\n    void again() { Svc.svc(); }\n}\n",
    );
    let mut index = Index::open_in_memory().unwrap();
    support::no_model_server();
    service::reindex(r, &mut index).unwrap();
    (dir, index)
}

#[test]
fn maps_jvm_tests_and_the_command_that_runs_them() {
    let (dir, index) = jvm_fixture();
    let map = testmap::test_map(dir.path(), &index).unwrap();
    let found = |path: &str| {
        let file = map
            .test_files
            .iter()
            .find(|t| t.path == path)
            .unwrap_or_else(|| panic!("no tests in {path}"));
        assert_eq!(file.kind, TestKind::Integration, "{path}");
        (
            file.tests
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>(),
            file.command.as_str(),
        )
    };
    // Methods with a test annotation of any framework, in either language,
    // and a spec class; a helper is no test.
    assert_eq!(
        found("core/src/test/kotlin/com/acme/core/PricesTest.kt"),
        (
            vec![
                "PricesTest.totals_a_price",
                "PricesTest.totalsMany",
                "Inner.nested"
            ],
            // The nested class has tests too, which `PricesTest` alone misses.
            "./gradlew :core:test --tests \"com.acme.core.PricesTest*\""
        )
    );
    assert_eq!(
        found("core/src/test/java/com/acme/core/MoneyTest.java"),
        (
            vec!["MoneyTest.cents", "MoneyTest.repeated"],
            "./gradlew :core:test --tests \"com.acme.core.MoneyTest\""
        )
    );
    assert_eq!(
        found("core/src/test/kotlin/com/acme/core/CartSpec.kt"),
        (
            vec!["CartSpec"],
            "./gradlew :core:test --tests \"com.acme.core.CartSpec\""
        )
    );
    assert_eq!(
        found("app/src/test/kotlin/com/acme/app/AppTest.kt"),
        (
            vec!["AppTest.runs"],
            "./gradlew :app:test --tests \"com.acme.app.AppTest\""
        )
    );
    // A Kotlin Multiplatform source set names its task; the shared tests run
    // through `allTests`, which takes no filter.
    assert_eq!(
        found("kmp/src/commonTest/kotlin/com/acme/kmp/SharedTest.kt").1,
        "./gradlew :kmp:allTests"
    );
    assert_eq!(
        found("kmp/src/jvmTest/kotlin/com/acme/kmp/JvmOnlyTest.kt").1,
        "./gradlew :kmp:jvmTest --tests \"com.acme.kmp.JvmOnlyTest\""
    );
    // A nested build is run from its own folder, with `gradle` when it has no
    // wrapper; Maven is told the module with `-pl` below an aggregator, and
    // by `-f` when the build has no other module.
    assert_eq!(
        found("plain/lib/src/test/java/com/acme/plain/PlainTest.java").1,
        "gradle -p plain :lib:test --tests \"com.acme.plain.PlainTest\""
    );
    assert_eq!(
        found("single/src/test/java/com/acme/single/CalcTest.java").1,
        "mvn -f single/pom.xml test -Dtest=CalcTest"
    );
    assert_eq!(
        found("reactor/svc/src/test/java/com/acme/svc/SvcTest.java").1,
        "mvn -f reactor/pom.xml -pl svc test -Dtest=SvcTest"
    );
    // Nine test files of Kotlin and Java, and nine more of the wide module.
    assert_eq!(map.test_files.len(), 18);
    assert_eq!(map.tests_total, 22);
    assert!(map.untested.is_empty(), "{:?}", map.untested);
}

#[test]
fn lists_the_jvm_tests_a_change_can_break() {
    let (dir, index) = jvm_fixture();
    let commands = |changed: &[&str]| {
        let changed: Vec<String> = changed.iter().map(|c| c.to_string()).collect();
        testmap::impacted_tests(dir.path(), &index, &changed).unwrap()
    };

    // A Kotlin file reaches its Kotlin test, a Java file reaches the tests of
    // both languages; one command runs a module's tests.
    let kotlin = commands(&["core/src/main/kotlin/com/acme/core/Prices.kt"]);
    assert_eq!(
        kotlin
            .test_files
            .iter()
            .map(|t| (t.path.as_str(), t.distance))
            .collect::<Vec<_>>(),
        [("core/src/test/kotlin/com/acme/core/PricesTest.kt", 1)]
    );
    assert_eq!(
        kotlin.commands,
        ["./gradlew :core:test --tests \"com.acme.core.PricesTest*\""]
    );
    let java = commands(&["core/src/main/java/com/acme/core/Money.java"]);
    assert_eq!(
        java.test_files
            .iter()
            .map(|t| t.path.as_str())
            .collect::<Vec<_>>(),
        [
            "core/src/test/java/com/acme/core/MoneyTest.java",
            "core/src/test/kotlin/com/acme/core/PricesTest.kt"
        ]
    );
    assert_eq!(
        java.commands,
        [
            "./gradlew :core:test --tests \"com.acme.core.MoneyTest\" --tests \"com.acme.core.PricesTest*\""
        ]
    );
    // Two changes of one module are one run; modules are separate runs.
    let both = commands(&[
        "core/src/main/kotlin/com/acme/core/Prices.kt",
        "core/src/main/java/com/acme/core/Money.java",
        "core/src/main/kotlin/com/acme/core/Cart.kt",
    ]);
    assert_eq!(
        both.commands,
        [
            "./gradlew :core:test --tests \"com.acme.core.MoneyTest\" --tests \"com.acme.core.CartSpec\" --tests \"com.acme.core.PricesTest*\"",
            "./gradlew :app:test --tests \"com.acme.app.AppTest\"",
        ]
    );
    // A test in another module reaches the code it imports.
    let cart = commands(&["core/src/main/kotlin/com/acme/core/Cart.kt"]);
    assert_eq!(
        cart.test_files
            .iter()
            .map(|t| (t.path.as_str(), t.distance))
            .collect::<Vec<_>>(),
        [
            ("core/src/test/kotlin/com/acme/core/CartSpec.kt", 1),
            ("app/src/test/kotlin/com/acme/app/AppTest.kt", 2),
        ]
    );

    // More than eight test classes in a module: the module runs whole.
    let wide = commands(&["wide/src/main/java/com/acme/wide/Wide.java"]);
    assert_eq!(wide.test_files.len(), 9);
    assert_eq!(wide.commands, ["./gradlew :wide:test"]);

    // Kotlin Multiplatform: each source set has its task.
    let kmp = commands(&["kmp/src/commonMain/kotlin/com/acme/kmp/Shared.kt"]);
    assert_eq!(
        kmp.commands,
        [
            "./gradlew :kmp:allTests",
            "./gradlew :kmp:jvmTest --tests \"com.acme.kmp.JvmOnlyTest\"",
        ]
    );

    // Maven names the classes, one run for the module.
    let maven = commands(&["reactor/svc/src/main/java/com/acme/svc/Svc.java"]);
    assert_eq!(maven.tests_total, 2);
    assert_eq!(
        maven.commands,
        ["mvn -f reactor/pom.xml -pl svc test -Dtest=SvcTest"]
    );
    let single = commands(&[
        "single/src/main/java/com/acme/single/Calc.java",
        "plain/lib/src/main/java/com/acme/plain/Plain.java",
    ]);
    assert_eq!(
        single.commands,
        [
            "gradle -p plain :lib:test --tests \"com.acme.plain.PlainTest\"",
            "mvn -f single/pom.xml test -Dtest=CalcTest",
        ]
    );
    assert!(single.untested_changes.is_empty());
}
