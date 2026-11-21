//! End-to-end tests of the generation pipeline on a small fixture repository.

use std::fs;
use std::path::Path;

use cw_parse::scan_repo;
use cw_vault::{Origin, ProposalStatus, VaultMeta, proposal};
use cw_wiki::{Report, analyze, generate};

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn read(root: &Path, rel: &str) -> String {
    fs::read_to_string(root.join(rel)).unwrap()
}

fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(r, "Cargo.toml", "[workspace]\nmembers = [\"crates/*\"]\n");
    write(
        r,
        "crates/engine/Cargo.toml",
        "[package]\nname = \"engine\"\ndescription = \"Order matching engine\"\n",
    );
    write(
        r,
        "crates/engine/src/lib.rs",
        "//! Order matching engine.\npub mod book;\npub mod pricing;\n",
    );
    write(
        r,
        "crates/engine/src/book.rs",
        "//! The order book. Orders rest here until matched.\nuse crate::pricing::Price;\n\n/// Holds resting orders, best price first.\npub struct OrderBook { best: Price }\n\nimpl OrderBook {\n    /// Create an empty book.\n    pub fn new() -> Self { Self { best: Price::zero() } }\n    /// Insert an order at `price`.\n    pub fn insert(&mut self, price: Price) { self.best = price; }\n    /// Best resting price.\n    pub fn best(&self) -> Price { self.best }\n}\n",
    );
    write(
        r,
        "crates/engine/src/pricing.rs",
        "//! Price arithmetic.\n\n/// A price in integer ticks.\n#[derive(Clone, Copy)]\npub struct Price(pub u64);\n\nimpl Price {\n    /// The zero price.\n    pub fn zero() -> Self { Price(0) }\n}\n",
    );
    write(
        r,
        "crates/cli/Cargo.toml",
        "[package]\nname = \"cli\"\n[dependencies]\nengine = { path = \"../engine\" }\n",
    );
    write(
        r,
        "crates/cli/src/main.rs",
        "use engine::book::OrderBook;\nuse engine::pricing::Price;\n\nfn main() {\n    let mut book = OrderBook::new();\n    book.insert(Price::zero());\n    let _ = book.best();\n}\n",
    );
    dir
}

fn run(root: &Path) -> Report {
    let facts = scan_repo(root).unwrap();
    let metrics = analyze(&facts);
    generate(root, &facts, &metrics, Some("commit-1".into())).unwrap()
}

#[test]
fn first_run_writes_a_linked_vault() {
    let dir = fixture();
    let report = run(dir.path());
    assert!(report.written.len() >= 10, "{report:?}");
    assert_eq!(report.link_rate, 1.0);
    assert!(report.proposed.is_empty() && report.removed.is_empty());

    let vault = dir.path().join("obsidian");
    let home = read(&vault, "_HOME.md");
    assert!(home.contains("tags: [moc, home]"));
    assert!(home.contains("## 🧭 Start here — Architecture flows"));
    assert!(home.contains("[[Crate Dependency Flow]]"));

    let book = read(&vault, "01-Core/engine/OrderBook.md");
    assert!(book.starts_with("---\ntags: [core, engine]\n---\n\n# OrderBook\n\n> Holds resting orders, best price first.\n"));
    assert!(book.contains("- `crates/engine/src/book.rs` — definition of `OrderBook`"));
    assert!(book.contains("- [[Price]] — uses"));
    assert!(book.contains("## See also\n\n- [[_index]]\n- [[Book Module]]"));

    let flow = read(&vault, "00-Architecture/Crate Dependency Flow.md");
    assert!(flow.contains("```mermaid\nflowchart LR"));

    let meta = VaultMeta::load(&vault).unwrap();
    assert_eq!(
        meta.manifest.last_indexed_commit.as_deref(),
        Some("commit-1")
    );
    assert!(vault.join(".codewiki/registry.json").exists());
    assert!(
        !book.contains("codewiki"),
        "machine metadata stays out of notes"
    );
}

#[test]
fn second_run_changes_nothing() {
    let dir = fixture();
    let first = run(dir.path());
    let second = run(dir.path());
    assert!(second.written.is_empty(), "{:?}", second.written);
    assert_eq!(second.kept, first.written.len());
}

#[test]
fn source_change_regenerates_only_affected_notes() {
    let dir = fixture();
    run(dir.path());
    let pricing = read(dir.path(), "crates/engine/src/pricing.rs").replace(
        "A price in integer ticks.",
        "A price in integer ticks of one cent.",
    );
    write(dir.path(), "crates/engine/src/pricing.rs", &pricing);
    let report = run(dir.path());
    assert!(
        report.written.iter().any(|p| p.ends_with("Price.md")),
        "{:?}",
        report.written
    );
    assert!(
        !report
            .written
            .iter()
            .any(|p| p.ends_with("OrderBook.md") || p.ends_with("_HOME.md")),
        "{:?}",
        report.written
    );
    assert!(read(dir.path(), "obsidian/01-Core/engine/Price.md").contains("one cent"));
}

#[test]
fn human_edited_notes_get_a_proposal_not_an_overwrite() {
    let dir = fixture();
    run(dir.path());
    let vault = dir.path().join("obsidian");
    let note = "01-Core/engine/Price.md";
    let edited = read(&vault, note) + "\n## Gotchas\n\n- Ticks are **never** negative.\n";
    write(&vault, note, &edited);

    // Editing alone raises nothing.
    let quiet = run(dir.path());
    assert!(quiet.proposed.is_empty() && !quiet.written.iter().any(|p| p == note));

    let pricing = read(dir.path(), "crates/engine/src/pricing.rs").replace(
        "A price in integer ticks.",
        "A price in integer ticks of one cent.",
    );
    write(dir.path(), "crates/engine/src/pricing.rs", &pricing);
    let report = run(dir.path());
    assert_eq!(report.proposed, [note]);
    assert_eq!(read(&vault, note), edited, "the edited note is untouched");

    let pending = proposal::list(&vault).unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].status, ProposalStatus::Pending);
    assert!(
        proposal::patch_text(&vault, &pending[0].id)
            .unwrap()
            .contains("one cent")
    );

    // Running again does not duplicate the proposal.
    run(dir.path());
    assert_eq!(proposal::list(&vault).unwrap().len(), 1);

    // Rejecting records the input so the same change is not proposed again.
    let rejected = proposal::reject(&vault, &pending[0].id).unwrap();
    let mut meta = VaultMeta::load(&vault).unwrap();
    meta.notes
        .get_mut(&rejected.note_id)
        .unwrap()
        .rejected_inputs
        .push(rejected.input_hash);
    meta.save(&vault).unwrap();
    let after = run(dir.path());
    assert!(after.proposed.is_empty(), "{:?}", after.proposed);
    assert_eq!(read(&vault, note), edited);
}

#[test]
fn existing_vault_is_adopted_without_changes() {
    let dir = fixture();
    let vault = dir.path().join("obsidian");
    let home = "---\ntags: [moc, home]\n---\n\n# My Vault\n\nHand written.\n";
    let book = "---\ntags: [backend]\n---\n\n# Order Book\n\n> My own description.\n\n## Source\n\n- `crates/engine/src/book.rs` — primary implementation\n\n## See also\n\n- [[_index]]\n";
    write(&vault, "_HOME.md", home);
    write(&vault, "01-Backend/Order Book.md", book);

    let report = run(dir.path());
    assert_eq!(report.adopted.len(), 2);
    assert_eq!(read(&vault, "_HOME.md"), home);
    assert_eq!(read(&vault, "01-Backend/Order Book.md"), book);
    assert!(
        report.proposed.is_empty(),
        "first contact raises no proposals"
    );

    // The file the adopted note covers gets no generated module or type note.
    assert!(!vault.join("01-Core/engine/OrderBook.md").exists());
    assert!(!vault.join("01-Core/engine/Book Module.md").exists());
    // Uncovered code is still documented, and links reach the adopted note.
    let price = read(&vault, "01-Core/engine/Price.md");
    assert!(price.contains("[[Order Book]]"), "{price}");

    let meta = VaultMeta::load(&vault).unwrap();
    let adopted = meta.by_path("01-Backend/Order Book.md").unwrap();
    assert_eq!(adopted.origin, Origin::Adopted);
    assert_eq!(adopted.sources[0].path, "crates/engine/src/book.rs");
    assert_eq!(meta.by_path("_HOME.md").unwrap().origin, Origin::Adopted);

    // A later change to the home map arrives as a proposal.
    write(
        dir.path(),
        "crates/engine/src/lib.rs",
        "//! Order matching engine, second edition.\npub mod book;\npub mod pricing;\n",
    );
    write(
        dir.path(),
        "crates/engine/Cargo.toml",
        "[package]\nname = \"engine\"\ndescription = \"Order matching engine, second edition\"\n",
    );
    let later = run(dir.path());
    assert!(later.proposed.iter().any(|p| p == "_HOME.md"), "{later:?}");
    assert_eq!(read(&vault, "_HOME.md"), home);
}

#[test]
fn a_type_that_moves_between_crates_keeps_the_vault_consistent() {
    let dir = fixture();
    run(dir.path());
    let vault = dir.path().join("obsidian");
    assert!(vault.join("01-Core/engine/Price.md").exists());

    // Move `Price` from the engine crate into a new crate.
    write(
        dir.path(),
        "crates/money/Cargo.toml",
        "[package]\nname = \"money\"\n",
    );
    write(
        dir.path(),
        "crates/money/src/lib.rs",
        "//! Money types.\n\n/// A price in integer ticks.\n#[derive(Clone, Copy)]\npub struct Price(pub u64);\n\nimpl Price {\n    /// The zero price.\n    pub fn zero() -> Self { Price(0) }\n}\n",
    );
    fs::remove_file(dir.path().join("crates/engine/src/pricing.rs")).unwrap();
    write(
        dir.path(),
        "crates/engine/src/lib.rs",
        "//! Order matching engine.\npub mod book;\n",
    );
    write(
        dir.path(),
        "crates/engine/src/book.rs",
        "//! The order book.\n\n/// Holds resting orders.\npub struct OrderBook;\n",
    );
    write(dir.path(), "crates/cli/src/main.rs", "fn main() {}\n");

    let moved = run(dir.path());
    assert!(
        !vault.join("01-Core/engine/Price.md").exists(),
        "the old note is removed, not orphaned: {moved:?}"
    );
    assert_eq!(
        moved.link_rate, 1.0,
        "no note links to one this run deleted"
    );
    // Every later run must succeed and settle.
    let again = run(dir.path());
    assert!(
        again.written.is_empty(),
        "one run is enough: {:?}",
        again.written
    );
    assert!(
        again.skipped.is_empty() && again.adopted.is_empty(),
        "{again:?}"
    );
    let settled = run(dir.path());
    assert!(settled.written.is_empty(), "{:?}", settled.written);
    let notes: Vec<String> = VaultMeta::load(&vault)
        .unwrap()
        .notes
        .values()
        .filter(|e| e.path.contains("Price"))
        .map(|e| e.path.clone())
        .collect();
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert!(notes[0].starts_with("01-Core/money/"), "{notes:?}");
}

#[test]
fn duplicate_names_in_an_existing_vault_do_not_abort() {
    let dir = fixture();
    let vault = dir.path().join("obsidian");
    let body = "---\ntags: [x]\n---\n\n# Setup\n\n> Mine.\n";
    write(&vault, "01-Backend/Setup.md", body);
    write(&vault, "05-Operations/setup.md", body);
    let report = run(dir.path());
    assert_eq!(report.adopted.len(), 1, "{report:?}");
    assert_eq!(report.skipped.len(), 1, "{report:?}");
    assert_eq!(read(&vault, "01-Backend/Setup.md"), body);
    assert_eq!(read(&vault, "05-Operations/setup.md"), body);
}

#[test]
fn notes_for_deleted_code_are_removed_unless_edited() {
    let dir = fixture();
    run(dir.path());
    let vault = dir.path().join("obsidian");
    let kept = "01-Core/engine/Price.md";
    write(&vault, kept, &(read(&vault, kept) + "\nMy note.\n"));

    fs::remove_file(dir.path().join("crates/engine/src/pricing.rs")).unwrap();
    write(
        dir.path(),
        "crates/engine/src/lib.rs",
        "//! Order matching engine.\npub mod book;\n",
    );
    write(
        dir.path(),
        "crates/engine/src/book.rs",
        "//! The order book.\n\n/// Holds resting orders.\npub struct OrderBook;\n",
    );
    write(dir.path(), "crates/cli/src/main.rs", "fn main() {}\n");
    let report = run(dir.path());
    assert!(
        report
            .removed
            .iter()
            .any(|p| p.ends_with("Pricing Module.md")),
        "{report:?}"
    );
    assert!(!vault.join("01-Core/engine/Pricing Module.md").exists());
    assert!(
        vault.join(kept).exists(),
        "a human-edited note is never deleted"
    );
}

/// A repository in a folder with a known name, so notes named after the
/// repository have a known path.
fn named_repo(name: &str, files: &[(&str, &str)]) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join(name);
    for (rel, text) in files {
        write(&root, rel, text);
    }
    (dir, root)
}

/// Run twice: the first run must write a vault that lints clean with every
/// link resolving, the second must write nothing.
fn run_settled(root: &Path) -> Report {
    let first = run(root);
    assert_eq!(first.link_rate, 1.0, "{first:?}");
    assert!(first.proposed.is_empty() && first.removed.is_empty());

    let vault = root.join("obsidian");
    let meta = VaultMeta::load(&vault).unwrap();
    let registry = meta.registry().unwrap();
    for entry in meta.notes.values() {
        let text = read(&vault, &entry.path);
        let errors: Vec<_> = cw_vault::lint(
            &text,
            &entry.path,
            entry.kind,
            &registry,
            cw_vault::Mode::Strict,
        )
        .into_iter()
        .filter(|i| i.severity == cw_vault::Severity::Error)
        .collect();
        assert!(errors.is_empty(), "{}: {errors:?}", entry.path);
    }

    let second = run(root);
    assert!(second.written.is_empty(), "{:?}", second.written);
    assert_eq!(second.kept, first.written.len());
    first
}

fn assert_notes(root: &Path, paths: &[&str]) {
    let vault = root.join("obsidian");
    for path in paths {
        assert!(
            vault.join(path).exists(),
            "missing {path}; have {:?}",
            VaultMeta::load(&vault)
                .unwrap()
                .notes
                .values()
                .map(|e| e.path.clone())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn a_python_package_gets_a_full_vault() {
    let (_dir, root) = named_repo(
        "pyshop",
        &[
            (
                "pyproject.toml",
                "[project]\nname = \"shop\"\ndependencies = [\"requests\"]\n",
            ),
            (
                "shop/__init__.py",
                "\"\"\"Order handling for a small shop.\"\"\"\n",
            ),
            (
                "shop/__main__.py",
                "from shop.orders import OrderBook\nfrom shop.pricing import total\n\n\ndef main():\n    book = OrderBook()\n    book.add(3)\n    print(total(book.items))\n\n\nmain()\n",
            ),
            (
                "shop/orders.py",
                "\"\"\"Orders and where they are kept.\"\"\"\nimport requests\n\n\nclass OrderBook:\n    \"\"\"Holds the orders of one customer.\"\"\"\n\n    def __init__(self):\n        self.items = []\n\n    def add(self, quantity):\n        \"\"\"Add an order line.\"\"\"\n        self.items.append(quantity)\n\n    def sync(self, url):\n        return requests.post(url, json=self.items)\n\n\ndef load_orders(url):\n    return requests.get(url).json()\n\n\ndef save_orders(book, url):\n    return book.sync(url)\n",
            ),
            (
                "shop/pricing.py",
                "\"\"\"Price arithmetic.\"\"\"\nimport requests\n\n\ndef total(items):\n    \"\"\"Sum of all order lines.\"\"\"\n    return sum(items)\n\n\ndef tax(amount):\n    return amount * 0.2\n\n\ndef rate(url):\n    return requests.get(url).json()\n",
            ),
        ],
    );
    run_settled(&root);
    assert_notes(
        &root,
        &[
            "01-Core/shop/shop.md",
            "01-Core/shop/Orders Module.md",
            "01-Core/shop/Pricing Module.md",
            "01-Core/shop/OrderBook.md",
            "00-Architecture/shop Startup Flow.md",
            "03-Integrations/requests.md",
            "05-Operations/Build and Test.md",
        ],
    );
    let vault = root.join("obsidian");
    let home = read(&vault, "_HOME.md");
    assert!(
        home.contains("**pyshop** is a Python codebase of 1 package and 5 source files."),
        "{home}"
    );
    assert!(
        home.contains("[[01-Core/shop/_index|shop/]] — order handling for a small shop."),
        "{home}"
    );

    let unit = read(&vault, "01-Core/shop/shop.md");
    assert!(unit.starts_with(
        "---\ntags: [core, shop]\n---\n\n# shop\n\n> Order handling for a small shop.\n"
    ));
    assert!(unit.contains("- `pyproject.toml` — Python project manifest"));
    assert!(unit.contains("- `shop/__init__.py` — package root"));
    assert!(unit.contains("Runs from `__main__.py`."), "{unit}");
    assert!(unit.contains("- [[Orders Module]] — orders and where they are kept."));

    let book = read(&vault, "01-Core/shop/OrderBook.md");
    assert!(book.contains("> Holds the orders of one customer."));
    assert!(book.contains("Declared as `class OrderBook`."));
    assert!(book.contains("## See also\n\n- [[_index]]\n- [[Orders Module]]"));

    let flow = read(&vault, "00-Architecture/shop Startup Flow.md");
    assert!(flow.contains("```mermaid\nsequenceDiagram"));
    assert!(flow.contains("from the `__main__.py` entry point of `shop`"));
    assert!(flow.contains("- [[OrderBook]]") && flow.contains("- [[Pricing Module]]"));

    let requests = read(&vault, "03-Integrations/requests.md");
    assert!(requests.contains("> External package `requests`, imported by 2 source files."));
    assert!(requests.contains("- `pyproject.toml` — declares the dependency"));

    let ops = read(&vault, "05-Operations/Build and Test.md");
    assert!(ops.contains("- `pyproject.toml` — Python project manifest"));
    assert!(
        ops.contains("python -m pip install -e .\npython -m shop\n"),
        "{ops}"
    );
    assert!(!ops.contains("cargo"), "{ops}");
}

#[test]
fn a_typescript_package_gets_a_full_vault() {
    let (_dir, root) = named_repo(
        "storefront",
        &[
            (
                "package.json",
                "{\n  \"name\": \"webshop\",\n  \"description\": \"Storefront for the shop\",\n  \"main\": \"dist/index.js\",\n  \"scripts\": { \"build\": \"tsc\", \"start\": \"node dist/index.js\" },\n  \"dependencies\": { \"express\": \"^4.0.0\" }\n}\n",
            ),
            (
                "src/index.ts",
                "import express from \"express\";\nimport { Cart } from \"./cart\";\nimport { render } from \"./view\";\n\nconst app = express();\n\nfunction main(): void {\n  const cart = new Cart();\n  cart.add(2);\n  render(cart.total());\n  app.listen(3000);\n}\n\nmain();\n",
            ),
            (
                "src/cart.ts",
                "import express from \"express\";\n\n/** Something with a price. */\nexport interface Priced {\n  price(): number;\n}\n\n/** The items a visitor wants to buy. */\nexport class Cart implements Priced {\n  items: number[] = [];\n  add(n: number): void {\n    this.items.push(n);\n  }\n  total(): number {\n    return this.items.length;\n  }\n  price(): number {\n    return this.total();\n  }\n}\n\nexport function cartRouter() {\n  return express.Router();\n}\n",
            ),
            (
                "src/view.ts",
                "export function header(): string {\n  return \"shop\";\n}\nexport function footer(): string {\n  return \"bye\";\n}\nexport function render(total: number): string {\n  return header() + total + footer();\n}\n",
            ),
        ],
    );
    run_settled(&root);
    assert_notes(
        &root,
        &[
            "01-Core/webshop/webshop.md",
            "01-Core/webshop/Cart Module.md",
            "01-Core/webshop/View Module.md",
            "01-Core/webshop/Cart.md",
            "00-Architecture/webshop Startup Flow.md",
            "03-Integrations/express.md",
            "05-Operations/Build and Test.md",
        ],
    );
    let vault = root.join("obsidian");
    let home = read(&vault, "_HOME.md");
    assert!(
        home.contains("**storefront** is a TypeScript codebase of 1 package and 4 source files."),
        "{home}"
    );
    let unit = read(&vault, "01-Core/webshop/webshop.md");
    assert!(unit.contains("> Storefront for the shop."), "{unit}");
    assert!(unit.contains("- `package.json` — package manifest"));
    assert!(unit.contains("Runs from `index.ts`."), "{unit}");
    assert!(unit.contains("- [[Cart Module]]") && unit.contains("- [[View Module]]"));

    let cart = read(&vault, "01-Core/webshop/Cart.md");
    assert!(cart.contains("> The items a visitor wants to buy."));
    assert!(cart.contains("- `src/cart.ts` — definition of `Cart`"));

    let express = read(&vault, "03-Integrations/express.md");
    assert!(express.contains("- `package.json` — declares the dependency"));
    let ops = read(&vault, "05-Operations/Build and Test.md");
    assert!(
        ops.contains("npm install\nnpm run build\nnpm run start\n"),
        "{ops}"
    );
}

#[test]
fn a_mixed_repository_documents_every_language() {
    let (_dir, root) = named_repo(
        "mixed",
        &[
            (
                "Cargo.toml",
                "[package]\nname = \"engine\"\ndescription = \"Order matching engine\"\n",
            ),
            ("src/lib.rs", "//! Order matching engine.\npub mod book;\n"),
            (
                "src/book.rs",
                "//! The order book.\n\n/// Holds resting orders.\npub struct OrderBook { pub depth: u64 }\n\nimpl OrderBook {\n    /// Create an empty book.\n    pub fn new() -> Self { Self { depth: 0 } }\n}\n",
            ),
            ("requirements.txt", "requests>=2\n"),
            (
                "analysis/__init__.py",
                "\"\"\"Reports on recorded trades.\"\"\"\n",
            ),
            (
                "analysis/report.py",
                "\"\"\"Daily trade reports.\"\"\"\n\n\nclass Report:\n    \"\"\"One day of trades.\"\"\"\n\n    def __init__(self):\n        self.rows = []\n\n    def add(self, row):\n        self.rows.append(row)\n\n    def total(self):\n        return len(self.rows)\n\n\ndef build(rows):\n    report = Report()\n    for row in rows:\n        report.add(row)\n    return report\n\n\nif __name__ == \"__main__\":\n    print(build([1, 2]).total())\n",
            ),
        ],
    );
    run_settled(&root);
    assert_notes(
        &root,
        &[
            "01-Core/engine/engine.md",
            "01-Core/engine/OrderBook.md",
            "01-Core/analysis/analysis.md",
            "01-Core/analysis/Report Module.md",
            "01-Core/analysis/Report.md",
            "00-Architecture/Package Dependency Flow.md",
            "05-Operations/Build and Test.md",
        ],
    );
    let vault = root.join("obsidian");
    let home = read(&vault, "_HOME.md");
    assert!(
        home.contains(
            "**mixed** is a Rust and Python codebase of 1 crate, 1 Python package and 6 source files."
        ),
        "{home}"
    );
    assert!(home.contains("[[01-Core/engine/_index|engine/]]"));
    assert!(home.contains("[[01-Core/analysis/_index|analysis/]]"));

    // Rust notes keep their wording next to the Python ones.
    let engine = read(&vault, "01-Core/engine/engine.md");
    assert!(engine.contains("- `Cargo.toml` — package manifest"));
    assert!(engine.contains("Builds a library."));
    let report = read(&vault, "01-Core/analysis/Report.md");
    assert!(
        report
            .starts_with("---\ntags: [core, analysis]\n---\n\n# Report\n\n> One day of trades.\n")
    );

    let ops = read(&vault, "05-Operations/Build and Test.md");
    assert!(ops.contains("The repository holds 1 crate and 1 Python package."));
    assert!(ops.contains("cargo test --workspace"));
    assert!(ops.contains("python -m pip install -r requirements.txt"));
    assert!(ops.contains("python analysis/report.py"), "{ops}");
}

#[test]
fn a_java_module_gets_a_full_vault() {
    let pkg = "src/main/java/com/acme";
    let (_dir, root) = named_repo(
        "billing",
        &[
            (
                "pom.xml",
                "<project>\n  <artifactId>billing</artifactId>\n  <dependencies>\n    <dependency><groupId>org.slf4j</groupId></dependency>\n  </dependencies>\n</project>\n",
            ),
            (
                &format!("{pkg}/text/Named.java"),
                "package com.acme.text;\n\n/** Anything with a display name. */\npublic interface Named {\n    String name();\n}\n",
            ),
            (
                &format!("{pkg}/text/Label.java"),
                "package com.acme.text;\n\nimport org.slf4j.Logger;\n\n/** A printable label. */\npublic class Label implements Named {\n    private final String text;\n\n    public Label(String text) {\n        this.text = text;\n    }\n\n    public String name() {\n        return text;\n    }\n\n    public String upper() {\n        return text.toUpperCase();\n    }\n}\n",
            ),
            (
                &format!("{pkg}/text/Title.java"),
                "package com.acme.text;\n\nimport org.slf4j.Logger;\n\npublic class Title implements Named {\n    public String name() {\n        return \"title\";\n    }\n}\n",
            ),
            (
                &format!("{pkg}/app/App.java"),
                "package com.acme.app;\n\nimport com.acme.text.Label;\nimport com.acme.text.Title;\n\npublic class App {\n    public static void main(String[] args) {\n        Label label = new Label(\"x\");\n        Title title = new Title();\n        System.out.println(label.upper() + title.name());\n    }\n}\n",
            ),
        ],
    );
    run_settled(&root);
    assert_notes(
        &root,
        &[
            "01-Core/billing/billing.md",
            "01-Core/billing/Text Module.md",
            "01-Core/billing/Label.md",
            "03-Integrations/org.slf4j.md",
            "04-Patterns/Implementing Named Pattern.md",
            "05-Operations/Build and Test.md",
        ],
    );
    let vault = root.join("obsidian");
    let home = read(&vault, "_HOME.md");
    assert!(
        home.contains("**billing** is a Java codebase of 1 module and 5 source files."),
        "{home}"
    );
    let pattern = read(&vault, "04-Patterns/Implementing Named Pattern.md");
    assert!(
        pattern.contains("```java\nclass NewType implements Named {\n    String name() {"),
        "{pattern}"
    );
    let ops = read(&vault, "05-Operations/Build and Test.md");
    assert!(ops.contains("- `pom.xml` — Maven build") && ops.contains("mvn package"));
}

/// A repository whose code says enough for a note to explain it: a module
/// comment of several parts, calls in a known order, items with and without
/// a comment, and a package from outside.
fn explained() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(r, "Cargo.toml", "[workspace]\nmembers = [\"crates/*\"]\n");
    write(
        r,
        "crates/app/Cargo.toml",
        "[package]\nname = \"app\"\n[dependencies]\nstore = { path = \"../store\" }\nserde = { version = \"1\", features = [\"derive\"] }\n",
    );
    write(
        r,
        "crates/app/src/main.rs",
        "use serde::Serialize;\nuse store::settings;\n\nfn main() {\n    if settings::enabled() {\n        settings::set(false);\n    }\n}\n",
    );
    write(
        r,
        "crates/store/Cargo.toml",
        "[package]\nname = \"store\"\n[dependencies]\nserde = \"1\"\n",
    );
    write(
        r,
        "crates/store/src/lib.rs",
        "//! Storage.\npub mod many;\npub mod settings;\n",
    );
    write(
        r,
        "crates/store/src/settings.rs",
        "//! Keeps the one setting. It is a file beside the program.\n//!\n//! Only an installed copy has it.\n//!\n//! # Errors\n//!\n//! A missing file reads as off, see `127.0.0.1:8080`.\n//!\n//! ```\n//! # use store::settings;\n//! settings::set(true);\n//! ```\nuse serde::Deserialize;\n\n/// Name of the file.\nconst FILE: &str = \"setting.txt\";\n\n/// Whether the setting is on.\n/// `false` when the file is gone.\npub fn enabled() -> bool {\n    parse(&read(FILE))\n}\n\n/// Turn the setting on or off.\npub fn set(on: bool) {\n    if on {\n        write(FILE)\n    } else {\n        remove(FILE)\n    }\n}\n\nfn read(name: &str) -> String {\n    name.to_string()\n}\n\nfn parse(text: &str) -> bool {\n    text == \"on\"\n}\n\nfn write(name: &str) {\n    let _ = name;\n}\n\nfn remove(name: &str) {\n    let _ = read(name);\n}\n\nfn unused_helper() {}\n",
    );
    let many: String = (0..23)
        .map(|i| format!("/// Step {i}.\npub fn step_{i:02}() {{}}\n"))
        .collect();
    write(
        r,
        "crates/store/src/many.rs",
        &format!("//! Many steps.\n{many}"),
    );
    dir
}

#[test]
fn a_module_note_explains_before_it_lists() {
    let dir = explained();
    run(dir.path());
    let vault = dir.path().join("obsidian");
    let note = read(&vault, "01-Core/store/Settings Module.md");

    // The whole comment after its first sentence, made safe for a note: no
    // heading of the comment's own, no hidden doctest line, no code span
    // that reads as a line number.
    assert!(
        note.contains(
            "> Keeps the one setting.\n\n## What it does\n\nIt is a file beside the program.\n\nOnly an installed copy has it.\n\nErrors:\n\nA missing file reads as off, see 127.0.0.1:8080.\n\n```rust\nsettings::set(true);\n```\n\n## Source\n"
        ),
        "{note}"
    );

    // Who calls in, and what each call does next, in the order of the
    // source: `parse(&read(FILE))` names `parse` first.
    assert!(
        note.contains(
            "## How it works\n\n1. `main` in [[app]] calls `enabled`. `enabled` calls `parse`, then `read`.\n2. `main` in [[app]] calls `set`. `set` calls `write`, then `remove`. `remove` calls `read`.\n\nCalls are listed in the order of the source"
        ),
        "{note}"
    );

    // Every item in source order: its signature and its whole comment, or
    // what the code says of one that has no comment.
    let items = [
        "- `const FILE: &str` — name of the file.",
        "- `pub fn enabled() -> bool` — whether the setting is on. `false` when the file is gone.",
        "- `pub fn set(on: bool)` — turn the setting on or off.",
        "- `fn read(name: &str) -> String` — private function, returns `String`; called by `enabled`, `remove`.",
        "- `fn parse(text: &str) -> bool` — private function, returns `bool`; called by `enabled`.",
        "- `fn write(name: &str)` — private function; called by `set`.",
        "- `fn remove(name: &str)` — private function; called by `set`.",
    ];
    let listed = note.split("## Items\n\n").nth(1).unwrap();
    assert!(listed.starts_with(&items.join("\n")), "{listed}");
    assert!(
        listed.contains("\n\nAnd 1 more private item with no comment.\n"),
        "{listed}"
    );
    assert!(!note.contains("unused_helper"));

    let at = |heading: &str| note.find(&format!("\n## {heading}\n")).unwrap();
    assert!(at("What it does") < at("Source"));
    assert!(at("Source") < at("How it works"));
    assert!(at("How it works") < at("Items"));
    assert!(at("Items") < at("Used by"));

    // A long list stops, and says how much it leaves out.
    let many = read(&vault, "01-Core/store/Many Module.md");
    assert_eq!(many.matches("\n- `pub fn step_").count(), 20, "{many}");
    assert!(many.contains("\n\nAnd 3 more items.\n"), "{many}");
}

/// Says what `serde` is, as a machine with cargo's cache would.
struct KnowsSerde;

impl cw_wiki::manifests::PackageInfo for KnowsSerde {
    fn describe(
        &self,
        _language: cw_core::Language,
        package: &str,
        _version: Option<&str>,
    ) -> Option<String> {
        (package == "serde").then(|| "A serialization framework".to_string())
    }
}

#[test]
fn an_integration_note_says_what_is_taken_and_at_which_version() {
    let dir = explained();
    let root = dir.path();
    run(root);
    let vault = root.join("obsidian");
    let path = "03-Integrations/serde.md";
    let note = read(&vault, path);
    assert!(
        note.contains("- [[Settings Module]] — imports `Deserialize`"),
        "{note}"
    );
    assert!(note.contains("- [[app]] — imports `Serialize`"), "{note}");
    // Each build file with what it asks for; two that differ are two lines.
    assert!(
        note.contains(
            "- `crates/app/Cargo.toml` — declares the dependency: version `1`, features `derive`\n"
        ),
        "{note}"
    );
    assert!(
        note.contains("- `crates/store/Cargo.toml` — declares the dependency: version `1`\n"),
        "{note}"
    );
    // Nothing in the repository says what the package is.
    assert!(!note.contains("## What it does"));
    assert!(!vault.join(".codewiki/packages.json").exists());

    // A machine that knows writes it, and keeps it in the vault.
    let facts = scan_repo(root).unwrap();
    let metrics = analyze(&facts);
    let commit = Some("commit-1".to_string());
    let report =
        cw_wiki::generate_with(root, &facts, &metrics, commit.clone(), &KnowsSerde).unwrap();
    assert_eq!(report.written, [path]);
    let described = read(&vault, path);
    assert!(
        described.contains("\n## What it does\n\nA serialization framework.\n\n## Used for\n"),
        "{described}"
    );
    // A machine that does not know then writes the same note.
    let report = generate(root, &facts, &metrics, commit).unwrap();
    assert!(report.written.is_empty(), "{report:?}");
    assert_eq!(read(&vault, path), described);
}

#[test]
fn rules_draw_no_picture_and_remove_the_ones_they_drew_before() {
    let dir = fixture();
    run(dir.path());
    let vault = dir.path().join("obsidian");
    // A picture explains what a note says, which rules cannot know.
    let pictures = |folder: &str| {
        fs::read_dir(vault.join(folder))
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().is_some_and(|e| e == "openui"))
            .count()
    };
    assert_eq!(pictures("01-Core/engine"), 0);
    assert_eq!(pictures("00-Architecture"), 0);

    // An earlier version drew tiles and charts for every note. Such a file
    // would still be served with its note, drawn with components the app
    // no longer has.
    let old = "01-Core/engine/engine.openui";
    write(
        &vault,
        old,
        "root = Card([f0])\nf0 = Stat(\"Modules\", \"2\")\n",
    );
    let mut meta = VaultMeta::load(&vault).unwrap();
    let id = meta.by_path("01-Core/engine/engine.md").unwrap().id.clone();
    meta.notes.get_mut(&id).unwrap().illustration = Some(old.to_string());
    meta.save(&vault).unwrap();

    let report = run(dir.path());
    assert!(report.written.is_empty(), "{report:?}");
    assert!(!vault.join(old).exists());
    let meta = VaultMeta::load(&vault).unwrap();
    assert_eq!(
        meta.by_path("01-Core/engine/engine.md")
            .unwrap()
            .illustration,
        None
    );
}

#[test]
fn every_explainable_note_splits_and_joins_to_itself() {
    // An agent's sections are woven into a note by its sections. A note
    // that did not come back to the letter would be rewritten by a weave
    // that changed nothing.
    for dir in [fixture(), explained()] {
        run(dir.path());
        let vault = dir.path().join("obsidian");
        let meta = VaultMeta::load(&vault).unwrap();
        let mut checked = 0;
        for entry in meta.notes.values().filter(|e| e.kind.explainable()) {
            let text = read(&vault, &entry.path);
            let (head, sections) = cw_vault::note::split(&text);
            assert_eq!(
                cw_vault::note::join(&head, &sections),
                text,
                "{}",
                entry.path
            );
            // Woven with nothing of an agent's, it is the same note.
            assert_eq!(cw_vault::weave(&text, &[], entry.kind).text, text);
            // Every heading is one of its kind: nothing would be moved.
            for section in &sections {
                assert!(
                    entry.kind.sections().contains(&&*section.heading),
                    "{}: {}",
                    entry.path,
                    section.heading
                );
            }
            checked += 1;
        }
        assert!(checked > 3, "{checked}");
    }
}

/// The picture an agent gave the section `How it works` of the note about
/// `Price`.
const PRICE_PICTURE: &str = "# section: How it works\nroot = Picture([f])\nf = Flow([Step(\"A number of ticks\"), Step(\"One price\")])\n";

/// What a submission does to a note, done by hand: weave the agent's
/// sections into the note on disk, save its pictures beside it, and record
/// in the manifest who explained it and from which files.
fn explain(root: &Path, note: &str, sections: &[(&str, &str)], pictures: &str) {
    let vault = root.join("obsidian");
    let mut meta = VaultMeta::load(&vault).unwrap();
    let id = meta.by_path(note).unwrap().id.clone();
    let entry = meta.notes.get_mut(&id).unwrap();
    let agent: Vec<cw_vault::Section> = sections
        .iter()
        .map(|(heading, body)| cw_vault::Section {
            heading: heading.to_string(),
            body: body.to_string(),
        })
        .collect();
    let woven = cw_vault::weave(&read(&vault, note), &agent, entry.kind);
    assert!(woven.folded.is_empty(), "{:?}", woven.folded);
    write(&vault, note, &woven.text);
    entry.body_hash = cw_vault::body_hash(&woven.text);
    entry.model = Some("test-agent".into());
    entry.explained = Some(cw_vault::Explained {
        sections: woven.written,
        sources: entry.sources.clone(),
        commit: Some("commit-1".into()),
    });
    if !pictures.is_empty() {
        let file = note.replace(".md", ".openui");
        write(&vault, &file, pictures);
        entry.illustration = Some(file);
    }
    meta.save(&vault).unwrap();
}

const PRICE: &str = "01-Core/engine/Price.md";
const PRICE_WORKS: &str =
    "A price counts ticks, and one tick is the smallest step a price can move.";
const PRICE_GOTCHA: &str = "- A price is never negative: the type holds an unsigned number.";

/// The fixture with a first run done and the note about `Price` explained.
fn explained_price() -> tempfile::TempDir {
    let dir = fixture();
    run(dir.path());
    explain(
        dir.path(),
        PRICE,
        &[("How it works", PRICE_WORKS), ("Gotchas", PRICE_GOTCHA)],
        PRICE_PICTURE,
    );
    dir
}

fn entry_of(root: &Path, note: &str) -> cw_vault::NoteEntry {
    VaultMeta::load(&root.join("obsidian"))
        .unwrap()
        .by_path(note)
        .unwrap()
        .clone()
}

fn change_the_price_comment(root: &Path) {
    let pricing = read(root, "crates/engine/src/pricing.rs").replace(
        "A price in integer ticks.",
        "A price in integer ticks of one cent.",
    );
    write(root, "crates/engine/src/pricing.rs", &pricing);
}

#[test]
fn a_run_counts_the_notes_no_agent_has_explained() {
    let dir = fixture();
    let first = run(dir.path());
    let meta = VaultMeta::load(&dir.path().join("obsidian")).unwrap();
    let explainable = meta.notes.values().filter(|e| e.kind.explainable()).count();
    assert!(explainable > 3);
    assert_eq!(first.unexplained, explainable);
    assert!(first.merged.is_empty() && first.behind.is_empty());
    assert_eq!(run(dir.path()).unexplained, explainable);

    explain(dir.path(), PRICE, &[("Gotchas", PRICE_GOTCHA)], "");
    let after = run(dir.path());
    assert_eq!(after.unexplained, explainable - 1);
    // An explanation is not a change by hand: nothing is proposed or
    // written, and the note is not behind.
    assert!(
        after.written.is_empty() && after.proposed.is_empty(),
        "{after:?}"
    );
    assert!(
        after.merged.is_empty() && after.behind.is_empty(),
        "{after:?}"
    );

    // A note its owner changed leaves the agents' queue.
    let book = "01-Core/engine/OrderBook.md";
    let vault = dir.path().join("obsidian");
    write(&vault, book, &(read(&vault, book) + "\nMy own line.\n"));
    assert_eq!(run(dir.path()).unexplained, explainable - 2);
}

#[test]
fn a_code_change_refreshes_the_facts_and_keeps_the_explanation() {
    let dir = explained_price();
    let vault = dir.path().join("obsidian");
    change_the_price_comment(dir.path());
    let report = run(dir.path());

    assert_eq!(report.merged, [PRICE], "{report:?}");
    assert!(!report.written.iter().any(|p| p == PRICE), "{report:?}");
    assert!(report.proposed.is_empty(), "{report:?}");
    let note = read(&vault, PRICE);
    // The rules' part follows the code.
    assert!(
        note.contains("> A price in integer ticks of one cent."),
        "{note}"
    );
    // The agent's part is its own, to the letter, and in its place.
    assert!(note.contains(&format!("## How it works\n\n{PRICE_WORKS}\n")));
    assert!(note.contains(&format!("## Gotchas\n\n{PRICE_GOTCHA}\n\n## See also")));
    assert_eq!(read(&vault, "01-Core/engine/Price.openui"), PRICE_PICTURE);

    let entry = entry_of(dir.path(), PRICE);
    assert_eq!(entry.model.as_deref(), Some("test-agent"));
    let explained = entry.explained.clone().unwrap();
    assert_eq!(explained.sections, ["How it works", "Gotchas"]);
    assert_eq!(explained.commit.as_deref(), Some("commit-1"));
    assert_eq!(
        entry.illustration.as_deref(),
        Some("01-Core/engine/Price.openui")
    );
    // The explanation was written from the file as it was: it may be
    // behind now, and the note says so until an agent writes again.
    assert_eq!(report.behind, [PRICE]);
    assert_eq!(entry.behind, ["crates/engine/src/pricing.rs changed"]);
    // The note is as CodeWiki left it: not one a person edited.
    assert_eq!(entry.body_hash, cw_vault::body_hash(&note));

    let again = run(dir.path());
    assert!(
        again.merged.is_empty() && again.written.is_empty(),
        "{again:?}"
    );
    assert_eq!(again.behind, [PRICE]);
    assert_eq!(read(&vault, PRICE), note);
    assert_eq!(entry_of(dir.path(), PRICE), entry);
}

#[test]
fn a_change_the_rules_do_not_show_still_marks_the_explanation_behind() {
    let dir = explained_price();
    let vault = dir.path().join("obsidian");
    let before = read(&vault, PRICE);
    // A function's body: no fact of the note changes.
    let pricing =
        read(dir.path(), "crates/engine/src/pricing.rs").replace("{ Price(0) }", "{ Price(0u64) }");
    write(dir.path(), "crates/engine/src/pricing.rs", &pricing);
    let report = run(dir.path());
    assert!(
        report.merged.is_empty() && report.written.is_empty(),
        "{report:?}"
    );
    assert_eq!(read(&vault, PRICE), before);
    assert_eq!(report.behind, [PRICE]);
    assert_eq!(
        entry_of(dir.path(), PRICE).behind,
        ["crates/engine/src/pricing.rs changed"]
    );

    // The file as it was again: the explanation is not behind any more.
    let pricing = pricing.replace("{ Price(0u64) }", "{ Price(0) }");
    write(dir.path(), "crates/engine/src/pricing.rs", &pricing);
    let report = run(dir.path());
    assert!(report.behind.is_empty(), "{report:?}");
    assert!(entry_of(dir.path(), PRICE).behind.is_empty());
}

#[test]
fn a_new_way_of_writing_the_facts_keeps_the_explanation() {
    let dir = explained_price();
    let vault = dir.path().join("obsidian");
    let before = read(&vault, PRICE);
    // As after an update of CodeWiki that writes the facts otherwise: the
    // note was made from another input than a run would use now.
    let mut meta = VaultMeta::load(&vault).unwrap();
    let id = meta.by_path(PRICE).unwrap().id.clone();
    meta.notes.get_mut(&id).unwrap().input_hash = "written by an older version".into();
    meta.save(&vault).unwrap();

    let report = run(dir.path());
    assert_eq!(report.merged, [PRICE], "{report:?}");
    assert_eq!(read(&vault, PRICE), before);
    let entry = entry_of(dir.path(), PRICE);
    assert!(entry.explained.is_some() && entry.behind.is_empty());
    assert!(report.behind.is_empty());
    assert!(vault.join("01-Core/engine/Price.openui").exists());
    assert!(run(dir.path()).merged.is_empty());
}

#[test]
fn an_explained_note_that_is_deleted_is_written_by_rules_again() {
    let dir = explained_price();
    let vault = dir.path().join("obsidian");
    fs::remove_file(vault.join(PRICE)).unwrap();
    let report = run(dir.path());
    assert!(report.written.iter().any(|p| p == PRICE), "{report:?}");
    let note = read(&vault, PRICE);
    assert!(!note.contains(PRICE_WORKS) && !note.contains("## Gotchas"));
    let entry = entry_of(dir.path(), PRICE);
    assert_eq!(entry.explained, None);
    assert_eq!(entry.model, None);
    assert!(entry.behind.is_empty());
    // Its pictures explained text that is gone.
    assert_eq!(entry.illustration, None);
    assert!(!vault.join("01-Core/engine/Price.openui").exists());
}

#[test]
fn an_explained_note_goes_with_its_subject() {
    let dir = explained_price();
    let vault = dir.path().join("obsidian");
    fs::remove_file(dir.path().join("crates/engine/src/pricing.rs")).unwrap();
    write(
        dir.path(),
        "crates/engine/src/lib.rs",
        "//! Order matching engine.\npub mod book;\n",
    );
    write(
        dir.path(),
        "crates/engine/src/book.rs",
        "//! The order book.\n\n/// Holds resting orders.\npub struct OrderBook;\n",
    );
    write(dir.path(), "crates/cli/src/main.rs", "fn main() {}\n");
    let report = run(dir.path());
    assert!(report.removed.iter().any(|p| p == PRICE), "{report:?}");
    assert!(!vault.join(PRICE).exists());
    assert!(!vault.join("01-Core/engine/Price.openui").exists());
}

#[test]
fn an_explained_note_changed_by_hand_gets_a_proposal_with_the_explanation() {
    let dir = explained_price();
    let vault = dir.path().join("obsidian");
    let edited =
        read(&vault, PRICE).replace(PRICE_GOTCHA, &format!("{PRICE_GOTCHA}\n- My own warning."));
    write(&vault, PRICE, &edited);
    assert!(run(dir.path()).proposed.is_empty());

    change_the_price_comment(dir.path());
    let report = run(dir.path());
    assert_eq!(report.proposed, [PRICE], "{report:?}");
    assert!(report.merged.is_empty());
    assert_eq!(read(&vault, PRICE), edited, "the note is untouched");

    let pending = proposal::list(&vault).unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].kind, cw_vault::ProposalKind::Sync);
    let proposed = proposal::proposed_text(&vault, &pending[0].id).unwrap();
    // The new fact, the agent's words and the owner's own line.
    assert!(proposed.contains("of one cent"), "{proposed}");
    assert!(proposed.contains(PRICE_WORKS));
    assert!(proposed.contains("- My own warning."));
    // The explanation is behind, whoever edited the note.
    assert_eq!(report.behind, [PRICE]);
}

#[test]
fn a_link_of_the_agents_to_a_note_that_is_gone_becomes_plain_words() {
    let dir = fixture();
    run(dir.path());
    let vault = dir.path().join("obsidian");
    let book = "01-Core/engine/OrderBook.md";
    explain(
        dir.path(),
        book,
        &[(
            "How it works",
            "The book holds one [[Price]], the best one. See [[Pricing Module|the price rules]].",
        )],
        "",
    );
    // The price type and its module go; the book stays.
    fs::remove_file(dir.path().join("crates/engine/src/pricing.rs")).unwrap();
    write(
        dir.path(),
        "crates/engine/src/lib.rs",
        "//! Order matching engine.\npub mod book;\n",
    );
    write(
        dir.path(),
        "crates/engine/src/book.rs",
        "//! The order book.\n\n/// Holds resting orders.\npub struct OrderBook;\n",
    );
    write(dir.path(), "crates/cli/src/main.rs", "fn main() {}\n");
    let report = run(dir.path());
    assert_eq!(report.merged, [book], "{report:?}");
    let note = read(&vault, book);
    assert!(
        note.contains("The book holds one Price, the best one. See the price rules."),
        "{note}"
    );
    // No link of the vault leads nowhere.
    assert_eq!(report.link_rate, 1.0);
    assert_eq!(
        entry_of(dir.path(), book).behind,
        ["crates/engine/src/book.rs changed"]
    );
}

#[test]
fn text_of_a_section_the_kind_lost_moves_and_its_picture_goes() {
    let dir = explained_price();
    let vault = dir.path().join("obsidian");
    // As an older version left the note: a section this kind of note has
    // no more, with a picture of its own.
    let old = read(&vault, PRICE).replace(
        "## Gotchas",
        "## Design notes\n\nTicks keep the arithmetic exact.\n\n## Gotchas",
    );
    write(&vault, PRICE, &old);
    let pictures = format!(
        "{PRICE_PICTURE}\n# section: Design notes\nroot = Picture([c])\nc = Callout(\"Exact.\")\n"
    );
    write(&vault, "01-Core/engine/Price.openui", &pictures);
    let mut meta = VaultMeta::load(&vault).unwrap();
    let id = meta.by_path(PRICE).unwrap().id.clone();
    let entry = meta.notes.get_mut(&id).unwrap();
    entry.body_hash = cw_vault::body_hash(&old);
    entry.input_hash = "written by an older version".into();
    entry
        .explained
        .as_mut()
        .unwrap()
        .sections
        .push("Design notes".into());
    meta.save(&vault).unwrap();

    let report = run(dir.path());
    assert_eq!(report.merged, [PRICE], "{report:?}");
    let note = read(&vault, PRICE);
    assert!(!note.contains("## Design notes"), "{note}");
    // Kept, under the first section the agent wrote.
    assert!(
        note.contains(&format!(
            "## How it works\n\n{PRICE_WORKS}\n\nTicks keep the arithmetic exact.\n"
        )),
        "{note}"
    );
    assert_eq!(read(&vault, "01-Core/engine/Price.openui"), PRICE_PICTURE);
    let entry = entry_of(dir.path(), PRICE);
    assert_eq!(
        entry.explained.clone().unwrap().sections,
        ["How it works", "Gotchas"]
    );
    assert_eq!(report.behind, [PRICE]);
    assert_eq!(entry.behind.len(), 1, "{:?}", entry.behind);
    assert!(
        entry.behind[0].starts_with("the section `Design notes` is no section"),
        "{:?}",
        entry.behind
    );
    // The reason stays until an agent writes the note again.
    let again = run(dir.path());
    assert!(again.merged.is_empty(), "{again:?}");
    assert_eq!(entry_of(dir.path(), PRICE).behind, entry.behind);
}
