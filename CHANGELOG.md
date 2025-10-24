# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## Elizabeth Board - [0.3.0](https://github.com/YuniqueUnic/elizabeth/releases/tag/v0.3.0) - 2025-10-23

### Added

- _(chunked-upload)_ implement database schema and models for chunked uploads
- _(board)_ add admin token verification for room deletion
- _(board)_ add slug column to rooms table and update related queries
- _(board)_ add room tokens table and related queries
- _(board)_ add RoomPermission type annotation in SQL queries and models
- _(board)_ refine room status enum and remove unused attributes
- _(board)_ unify Room model for DB and API layers
- _(board)_ update room status to use enum with sqlx type mapping
- _(board)_ enable logging feature and fix related compilation issues
- _(board)_ add health check endpoint
- _(board)_ add axum_responses dependency

### Fixed

- _(board)_ replace numeric permission with RoomPermission enum
- _(board)_ rename default database file from 'database.db' to 'app.db'
- _(board)_ update default database path and use config-provided URL
- _(board)_ add sqlx query files for room operations
- _(board)_ add OpenAPI documentation and scalar UI
- _(board)_ add status endpoint

### Other

- _(deps)_ update rust dependencies and adjust axum versions
- _(board)_ refactor token claim construction using builder pattern
- _(board)_ implement refresh token and blacklist mechanism
- _(board)_ implement room expiration logic and update migration structure
- _(board)_ implement database schema for room-based file sharing service
- _(board)_ implement room-centric data model and enhance database schema
- _(board)_ implement content handler module and update module exports
- _(board)_ optimize room and content repositories with transactional writes and
  shared queries
- _(board)_ refactor room content storage and repository interface
- _(board)_ add bon crate and refactor room content model
- use bitflags to implement the permissions..
- _(workspace)_ rename packages to elizabeth-* and update references
- _(db)_ extract database constants and update model enums
- _(board)_ update dependencies and adjust versions
- _(board)_ rename CustomDateTime to NativeDateTimeWrapper
- _(room)_ implement Room CRUD API with password protection and expiration
- _(pre-commit)_ reorder and stage-specific rust checks
- _(board)_ remove unused axum-macros dependency and update routing structure
- _(prek)_ add pre-commit hooks configuration with Rust support
- _(board)_ add health and status endpoints with OpenAPI integration
- _(board)_ modularize routing and API documentation
- _(workspace)_ centralize package metadata in workspace
- _(release)_ add release binaries workflow and update release-plz config
- _(deps)_ update convert_case and clap dependencies
- _(board)_ structure the core application named board.
- _(project)_ initialize project configuration for Elizabeth
- _(workspace)_ rename crate from elizabeth to board

## Elizabeth Board - [0.3.0](https://github.com/YuniqueUnic/elizabeth/releases/tag/v0.3.0) - 2025-10-20

### Added

- _(board)_ refine room status enum and remove unused attributes
- _(board)_ unify Room model for DB and API layers
- _(board)_ update room status to use enum with sqlx type mapping
- _(board)_ enable logging feature and fix related compilation issues
- _(board)_ add health check endpoint
- _(board)_ add axum_responses dependency

### Fixed

- _(board)_ rename default database file from 'database.db' to 'app.db'
- _(board)_ update default database path and use config-provided URL
- _(board)_ add sqlx query files for room operations
- _(board)_ add OpenAPI documentation and scalar UI
- _(board)_ add status endpoint

### Other

- _(workspace)_ rename packages to elizabeth-* and update references
- _(db)_ extract database constants and update model enums
- _(board)_ update dependencies and adjust versions
- _(board)_ rename CustomDateTime to NativeDateTimeWrapper
- _(room)_ implement Room CRUD API with password protection and expiration
- _(pre-commit)_ reorder and stage-specific rust checks
- _(board)_ remove unused axum-macros dependency and update routing structure
- _(prek)_ add pre-commit hooks configuration with Rust support
- _(board)_ add health and status endpoints with OpenAPI integration
- _(board)_ modularize routing and API documentation
- _(workspace)_ centralize package metadata in workspace
- _(release)_ add release binaries workflow and update release-plz config
- _(deps)_ update convert_case and clap dependencies
- _(board)_ structure the core application named board.
- _(project)_ initialize project configuration for Elizabeth
- _(workspace)_ rename crate from elizabeth to board

## Elizabeth Board - [0.3.0](https://github.com/YuniqueUnic/elizabeth/releases/tag/v0.3.0) - 2025-10-16

### Added

- _(board)_ refine room status enum and remove unused attributes
- _(board)_ unify Room model for DB and API layers
- _(board)_ update room status to use enum with sqlx type mapping
- _(board)_ enable logging feature and fix related compilation issues
- _(board)_ add health check endpoint
- _(board)_ add axum_responses dependency

### Fixed

- _(board)_ rename default database file from 'database.db' to 'app.db'
- _(board)_ update default database path and use config-provided URL
- _(board)_ add sqlx query files for room operations
- _(board)_ add OpenAPI documentation and scalar UI
- _(board)_ add status endpoint

### Other

- _(workspace)_ rename packages to elizabeth-* and update references
- _(db)_ extract database constants and update model enums
- _(board)_ update dependencies and adjust versions
- _(board)_ rename CustomDateTime to NativeDateTimeWrapper
- _(room)_ implement Room CRUD API with password protection and expiration
- _(pre-commit)_ reorder and stage-specific rust checks
- _(board)_ remove unused axum-macros dependency and update routing structure
- _(prek)_ add pre-commit hooks configuration with Rust support
- _(board)_ add health and status endpoints with OpenAPI integration
- _(board)_ modularize routing and API documentation
- _(workspace)_ centralize package metadata in workspace
- _(release)_ add release binaries workflow and update release-plz config
- _(deps)_ update convert_case and clap dependencies
- _(board)_ structure the core application named board.
- _(project)_ initialize project configuration for Elizabeth
- _(workspace)_ rename crate from elizabeth to board

## Elizabeth Board - [0.3.0](https://github.com/YuniqueUnic/elizabeth/compare/v0.2.0...v0.3.0) - 2025-10-12

### Added

- _(board)_ add health check endpoint
- _(board)_ add axum_responses dependency

### Fixed

- _(board)_ add OpenAPI documentation and scalar UI
- _(board)_ add status endpoint

### Other

- _(board)_ modularize routing and API documentation
- _(workspace)_ centralize package metadata in workspace
- _(release)_ add release binaries workflow and update release-plz config
- _(deps)_ update convert_case and clap dependencies
- _(board)_ structure the core application named board.
- _(project)_ initialize project configuration for Elizabeth
- _(workspace)_ rename crate from elizabeth to board

## Elizabeth Board - [0.2.0](https://github.com/YuniqueUnic/elizabeth/compare/v0.1.0...v0.2.0) - 2025-10-12

### Added

- _(board)_ add health check endpoint
- _(board)_ add axum_responses dependency

### Fixed

- _(board)_ add status endpoint

### Other

- _(workspace)_ centralize package metadata in workspace
- _(release)_ add release binaries workflow and update release-plz config
- _(deps)_ update convert_case and clap dependencies
- _(board)_ structure the core application named board.
- _(project)_ initialize project configuration for Elizabeth
- _(workspace)_ rename crate from elizabeth to board

## Elizabeth Board - [0.2.0](https://github.com/YuniqueUnic/elizabeth/compare/logrs-v0.1.0...logrs-v0.2.0) - 2025-10-12

### Other

- _(workspace)_ centralize package metadata in workspace
- _(board)_ structure the core application named board.

## Elizabeth Board - [0.2.0](https://github.com/YuniqueUnic/elizabeth/compare/configrs-v0.1.0...configrs-v0.2.0) - 2025-10-12

### Other

- _(workspace)_ centralize package metadata in workspace
- _(release)_ add release binaries workflow and update release-plz config
- _(deps)_ update convert_case and clap dependencies
- _(board)_ structure the core application named board.

### Added

- 初始化 elizabeth Rust 项目
- 配置 release-plz 用于 workspace 级别的版本管理
- 添加 board 模块作为项目的第一个组件

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.1.0] - 2024-10-11

### Added

- 项目初始结构
- 基本的 workspace 配置
- release-plz 集成

[Unreleased]: https://github.com/your-username/elizabeth/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/your-username/elizabeth/releases/tag/v0.1.0
