# Change Log

## Unreleased:

## 2023-08-03: Version 0.1.1

* Start using `raw-dylib` for linking to `kernel32.dll` on Windows.
* Switched to randomized skip list for verification of uniqueness of `TypeId`s
on non-Linux and non-Windows platforms to improve performance.
* Added autogeneration of documentation for Cargo features.
