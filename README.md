This tool provides a convenient interface for inspecting ELF binaries and their dependencies. Think of it as a combination of `ldd` and `readelf`.

Compared to `ldd`, this tool is somewhat more flexible in that it supports multiple architectures and custom system root specifications to enable searching for shared libraries in filesystem images, rather than in the host system root filesystem.

# Building

The tool is implemented in Rust and uses the standard Cargo build system:

```
cargo build
```

# Usage

To run the tool:

```
binary-walkr /path/to/binary
```

It supports the following options:

- `--sysroot`: Specify an alternative root to search for shared libraries from

## Shared Library Search

This tool attempts to resolve shared library dependencies in the same way as the dynamic loader, including consulting the default system library directories and `LD_LIBRARY_PATH`. It will eventually support `DT_RPATH` and `DT_RUNPATH`, but does not yet.
