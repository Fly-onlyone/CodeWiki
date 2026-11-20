---
tags: [integration]
---

# windows_sys

> External crate `windows_sys`, imported by 3 source files.

## What it does

Rust for Windows.

## Used for

Imported by 3 source files. Main users:

- [[Game Module]] — imports `Win32::System::Performance::PDH_FMT_COUNTERVALUE_ITEM_W`, `Win32::System::Performance::PDH_FMT_DOUBLE`, `Win32::System::Performance::PDH_HCOUNTER`, `Win32::System::Performance::PDH_HQUERY` and 9 more
- [[Secret Module]] — imports `Win32::Foundation::LocalFree`, `Win32::Security::Cryptography::CRYPTPROTECT_UI_FORBIDDEN`, `Win32::Security::Cryptography::CRYPT_INTEGER_BLOB`, `Win32::Security::Cryptography::CryptProtectData` and 1 more
- [[Win32 Module]] — imports `Win32::Foundation::HWND`, `Win32::Foundation::LPARAM`, `Win32::Foundation::LRESULT`, `Win32::Foundation::WPARAM` and 18 more

## Configuration

- `crates/codewiki-app/Cargo.toml` — declares the dependency: version `0.61`, locked `0.61.2`, features `Win32_Foundation`, `Win32_System_Console`, `Win32_UI_WindowsAndMessaging`
- `crates/cw-core/Cargo.toml` — declares the dependency: version `0.61.2`, locked `0.61.2`, features `Win32_Security_Cryptography`
- `crates/cw-models/Cargo.toml` — declares the dependency: version `0.61.2`, locked `0.61.2`, features `Win32_Foundation`, `Win32_System_Performance`, `Win32_UI_Shell`

## See also

- [[_index]]
