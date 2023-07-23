# Directory Structure

Here, we explain the directory and file arrangement structure for the various components of
Sylvia-IoT.

```
[project]/
├── doc/
│   ├── api.md
│   ├── cache.md
│   ├── message.md
│   └── schema.md
├── src/
│   ├── bin/
│   │   ├── [project].rs
│   │   ├── [bin1].rs
│   │   ├── [bin2].rs
│   │   └── ...
│   ├── libs/
│   │   ├── config..rs
│   │   ├── [lib1]/
│   │   ├── [lib2].rs
│   │   └── ...
│   ├── models/
│   │   ├── [engine1]/
│   │   │   ├── [table1].rs
│   │   │   ├── [table2].rs
│   │   │   └── ...
│   │   ├── [engine2]/
│   │   │   ├── [table1].rs
│   │   │   ├── [table2].rs
│   │   │   └── ...
│   │   ├── [table1].rs
│   │   ├── [table2].rs
│   │   └── ...
│   └── routes/
│       ├── v1/
│       │   ├── [api1]/
│       │   │   ├── api.rs
│       │   │   ├── request.rs
│       │   │   └── response.rs
│       │   ├── [api2]/
│       │   │   ├── api.rs
│       │   │   ├── request.rs
│       │   │   └── response.rs
│       │   └── ...
│       ├── v2/
│       ├── [non-versioned-api]/
│       ├── ...
│       └── middleware.rs
├── tests/
│   ├── libs/
│   │   ├── config..rs
│   │   ├── [lib1]/
│   │   ├── [lib2].rs
│   │   └── ...
│   ├── models/
│   │   ├── [engine1]/
│   │   │   ├── [table1].rs
│   │   │   ├── [table2].rs
│   │   │   └── ...
│   │   ├── [engine2]/
│   │   │   ├── [table1].rs
│   │   │   ├── [table2].rs
│   │   │   └── ...
│   │   ├── [table1].rs
│   │   ├── [table2].rs
│   │   └── ...
│   └── routes/
│       ├── v1/
│       │   ├── [api1]/
│       │   │   ├── api.rs
│       │   │   ├── request.rs
│       │   │   └── response.rs
│       │   ├── [api2]/
│       │   │   ├── api.rs
│       │   │   ├── request.rs
│       │   │   └── response.rs
│       │   └── ...
│       ├── v2/
│       ├── [non-versioned-api]/
│       ├── ...
│       └── middleware.rs
├── Cargo.toml
├── LICENSE
└── README.md
```

Here are several key points to note:

- `bin`: Contains a rs file with the same name as the project.
- `doc`: This directory is intended for complete documentation.
- `libs`: Contains files other than the database and API-related components.
- `models`: Designed primarily for table-based structures, and uses the database engine for
  separation.
- `routes`: Contains the implementation of HTTP APIs.
    - Apart from implementing standard APIs, such as OAuth2, others should be versioned to
      differentiate them.
- `tests`: Corresponds one-to-one with the `src` directory.

## Dependencies

- `libs` and `models` do not depend on any other folders.
- In `routes`
    - The entire project's code initialization is centralized in `routes/mod.rs`.
    - This approach reduces the workload for `main.rs` and increases the coverage of integration
      testing.
- Modules inside `models` should not depend on each other. If there are shared functionalities,
  implement them in the parent module and reference them as needed. The same applies to modules
  within `routes`.
