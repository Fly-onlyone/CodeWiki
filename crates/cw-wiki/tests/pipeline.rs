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
    generate(root, &facts, &metrics, Some("commit-1".into()), None).unwrap()
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
fn crate_and_flow_notes_get_valid_illustrations() {
    let dir = fixture();
    let report = run(dir.path());
    let vault = dir.path().join("obsidian");
    assert!(
        report
            .illustrated
            .contains(&"01-Core/engine/engine.openui".to_string()),
        "{:?}",
        report.illustrated
    );
    let source = read(&vault, "01-Core/engine/engine.openui");
    assert!(
        source.starts_with("root = Card([summary, figures, chart], \"engine\")\n"),
        "{source}"
    );
    assert!(source.contains("Stat(\"Modules\", \"2\")"), "{source}");
    assert!(
        source.contains("BarChart([\"book\", \"pricing\"]"),
        "{source}"
    );
    assert!(cw_illustrate::check(&source).errors.is_empty());
    let meta = VaultMeta::load(&vault).unwrap();
    assert_eq!(
        meta.by_path("01-Core/engine/engine.md")
            .unwrap()
            .illustration
            .as_deref(),
        Some("01-Core/engine/engine.openui")
    );

    // Unchanged facts rewrite nothing; a hand-edited note keeps its illustration as is.
    assert!(run(dir.path()).illustrated.is_empty());
    let note = "01-Core/engine/engine.md";
    write(&vault, note, &(read(&vault, note) + "\nMine.\n"));
    write(
        dir.path(),
        "crates/engine/src/extra.rs",
        "pub fn extra() {}\n",
    );
    write(
        dir.path(),
        "crates/engine/src/lib.rs",
        "//! Order matching engine.\npub mod book;\npub mod extra;\npub mod pricing;\n",
    );
    let later = run(dir.path());
    assert!(
        !later
            .illustrated
            .iter()
            .any(|p| p.ends_with("engine.openui")),
        "{:?}",
        later.illustrated
    );
    assert_eq!(read(&vault, "01-Core/engine/engine.openui"), source);
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
    assert!(second.illustrated.is_empty(), "{:?}", second.illustrated);
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
