# 目錄結構

這裡說明 Sylvia-IoT 各個元件的目錄和檔案的編排結構。

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

這邊列出幾個要點：

- `bin`: 要有一個和專案同名的 rs 檔案。
- `doc`: 完整的文件要放在這。
- `libs`: 資料庫、API 以外的放在這裡。
- `models`: 以表格為主的設計，並使用資料庫引擎區隔。
- `routes`: HTTP API 的實作。
    - 除了實作標準 API，如 OAuth2，其他都需要用版本來區隔。
- `tests`: 和 `src` 一一對應。

## 相依性

- libs、models 不相依其他資料夾。
- routes
    - 整個專案程式碼的初始化集中在 `routes/mod.rs`。
    - 除了可以讓 main.rs 做最少的事情，也能增加整合測試的覆蓋範圍。
- models 內部的模組彼此不相依。如有共用功能，請在父模組實現然後引用。routes 內部的模組亦同。
