# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v1.1.0.html).

## [1.2.1](https://github.com/YuniqueUnic/elizabeth/compare/v1.2.0...v1.2.1) (2026-06-05)


### Bug Fixes

* **api:** export buildURL function for external usage ([e1aace7](https://github.com/YuniqueUnic/elizabeth/commit/e1aace7a5650a642ea74c90dfc30168267dfafe5))

## [1.2.0](https://github.com/YuniqueUnic/elizabeth/compare/v1.1.0...v1.2.0) (2026-06-04)


### Features

* **chunked-upload:** add cancel endpoint and fix size calculation ([ab6c957](https://github.com/YuniqueUnic/elizabeth/commit/ab6c95773ff2cfc7340761d33e8781414981804c))
* **file-service:** implement abort support and progress tracking ([ab6c957](https://github.com/YuniqueUnic/elizabeth/commit/ab6c95773ff2cfc7340761d33e8781414981804c))
* **format:** update date formatting logic ([164b354](https://github.com/YuniqueUnic/elizabeth/commit/164b35422b977dcd60a209371228c97e29583689))
* **ui:** add transfer progress panel component ([ab6c957](https://github.com/YuniqueUnic/elizabeth/commit/ab6c95773ff2cfc7340761d33e8781414981804c))
* **url-viewer:** add external URL preview with iframe support ([5dbbf48](https://github.com/YuniqueUnic/elizabeth/commit/5dbbf4816f663e63587dded2adf3be8a079baa10))


### Bug Fixes

* **chat:** add file click handling to minimal tiptap viewer ([6452180](https://github.com/YuniqueUnic/elizabeth/commit/645218020d167dbefa8778ede45e7c3284a3633e))
* **chunked_upload:** correct code formatting in cleanup logic ([2e3b71d](https://github.com/YuniqueUnic/elizabeth/commit/2e3b71dc09bab0ca004bcf231e55cc300868a978))
* **content:** improve code formatting for header insertion ([2e3b71d](https://github.com/YuniqueUnic/elizabeth/commit/2e3b71dc09bab0ca004bcf231e55cc300868a978))
* enhance message copy/download functionality with error handling ([a78c8fa](https://github.com/YuniqueUnic/elizabeth/commit/a78c8fadc7edccce5e1ccb19859c6ba9dc4e51c6))
* **file-service:** enhance file download and sharing functionality ([4d9584d](https://github.com/YuniqueUnic/elizabeth/commit/4d9584d9f238073ba9bb8a534891d817f587cb59))
* **files:** correct file URL path in preview modal ([5dbbf48](https://github.com/YuniqueUnic/elizabeth/commit/5dbbf4816f663e63587dded2adf3be8a079baa10))
* **files:** update file preview URL format in modal ([6452180](https://github.com/YuniqueUnic/elizabeth/commit/645218020d167dbefa8778ede45e7c3284a3633e))
* **i18n:** add internationalization support with next-intl ([9c34f5c](https://github.com/YuniqueUnic/elizabeth/commit/9c34f5cef7dfa6a6e8d6bd0eac8401a36c66c4ca))
* **pre-commit:** update regex pattern for generated directory exclusion ([e3d02ad](https://github.com/YuniqueUnic/elizabeth/commit/e3d02add31866e5100bf77561c2f726bcb9958a1))
* **types:** correct formatting inconsistencies in generated TypeScript types ([54c09c1](https://github.com/YuniqueUnic/elizabeth/commit/54c09c1ec45b3640fd6589b854901c13224e10dc))
* **ui:** improve component styling and layout responsiveness ([164b354](https://github.com/YuniqueUnic/elizabeth/commit/164b35422b977dcd60a209371228c97e29583689))

## [1.1.0](https://github.com/YuniqueUnic/elizabeth/compare/v1.0.1...v1.1.0) (2026-05-28)


### ⚠ BREAKING CHANGES

* **backend:** Remove separate frontend container and nginx gateway, consolidating into single backend container with embedded SPA served via rust-embed. All endpoints now accessible through port 4092 instead of separate frontend port 4001.

### Features

* **arch:** consolidate frontend SPA and backend Axum into a single monolithic Rust service, eliminating gateway and frontend containers ([801011a](https://github.com/YuniqueUnic/elizabeth/commit/801011a8400d29258b7287ed51fad1d058db4f1a))


### Bug Fixes

* **e2e:** add wait time and toast dismissal for room tests ([8645971](https://github.com/YuniqueUnic/elizabeth/commit/86459715f6800a4b628cdd0bf9d80a063264779b))
* embed SPA frontend into Rust backend binary ([9622ab1](https://github.com/YuniqueUnic/elizabeth/commit/9622ab12768c1b69d1cd96d230627e330408460a))


### Code Refactoring

* **backend:** merge frontend SPA into single container ([9bb8aba](https://github.com/YuniqueUnic/elizabeth/commit/9bb8abad3ccdeb0df24ad8a5a70ee55747d9feae))

## [1.0.1](https://github.com/YuniqueUnic/elizabeth/compare/v1.0.0...v1.0.1) (2026-05-28)


### Bug Fixes

* **board-protocol:** remove unused sqlx::Type import to fix strict CD binary builds ([57c36d8](https://github.com/YuniqueUnic/elizabeth/commit/57c36d826aa6bbb5a62d20c031214c610b5707bf))
* **ci:** globally suppress compiler warnings across all crates and tests to fix CD compilation with RUSTFLAGS=-D warnings ([89d3f92](https://github.com/YuniqueUnic/elizabeth/commit/89d3f928580025944a797660e2ea314b8631127e))

## [1.0.0](https://github.com/YuniqueUnic/elizabeth/compare/v0.3.0...v1.0.0) (2026-05-28)


### ⚠ BREAKING CHANGES

* **board-protocol:** API endpoint paths have been modified for content downloads to use global identifiers instead of room-based paths.
* **database:** Existing configurations using "sqlite:" format will be automatically normalized to "sqlite://" format.
* **board:** This removes the public API that exposed these generated type definitions, which may affect consumers relying on these bindings.

### Features

* add host bind mounts configuration and improve room update handling ([de310c4](https://github.com/YuniqueUnic/elizabeth/commit/de310c46870fa9e058e07076ce16d5b0e7d9be2f))
* add room garbage collection service with admin API ([d9ed783](https://github.com/YuniqueUnic/elizabeth/commit/d9ed783d829f98e9a7e70c92ac311fc8b80b7626))
* Add room identifier validation and enhance token management ([07be38b](https://github.com/YuniqueUnic/elizabeth/commit/07be38b1cb65f59065077a2077cfd7c4cae50246))
* add server info initialization module ([c91534e](https://github.com/YuniqueUnic/elizabeth/commit/c91534e18bba229da07cf78a86c62f2273fc4c0f))
* add TypeScript schema generation support ([5fb2ffa](https://github.com/YuniqueUnic/elizabeth/commit/5fb2ffa9b4e1b44861adc6c88544e220f45418ed))
* add TypeScript type definitions for board API models ([2b08201](https://github.com/YuniqueUnic/elizabeth/commit/2b082013fd6e3a555ee53991bdbbb393837c86ba))
* **api-utils:** enhance URL construction and parameter handling ([95c8a1c](https://github.com/YuniqueUnic/elizabeth/commit/95c8a1c72fb6a8a31f823f4ab0e36e6bd9e7b0e3))
* **api:** expose OpenAPI document for client generation ([8a38fad](https://github.com/YuniqueUnic/elizabeth/commit/8a38fade4a75cc4c705676c85c32fc07480de0b2))
* **api:** implement chunked file upload and permission management services ([2535a51](https://github.com/YuniqueUnic/elizabeth/commit/2535a51b73b42e126a27f3c5b63ad8f538a1da92))
* **api:** integrate backend API for room management and authentication ([6b5a057](https://github.com/YuniqueUnic/elizabeth/commit/6b5a0578a7e1c89d677144f0f47c10d0f17e5e9c))
* **auth:** add comprehensive token verification and management tests ([75ad5a5](https://github.com/YuniqueUnic/elizabeth/commit/75ad5a51c95aa7e050b14578f76b5f24148a8a43))
* **auth:** enhance token management for room access and refresh ([76d3ab5](https://github.com/YuniqueUnic/elizabeth/commit/76d3ab5d9a902e172b35bd6e8a1bc4970505e005))
* **board:** add admin token verification for room deletion ([f313927](https://github.com/YuniqueUnic/elizabeth/commit/f313927ead8616dd3ffcf0e10ea846c7527d36f0))
* **board:** add database configuration and storage layer support ([acfacd7](https://github.com/YuniqueUnic/elizabeth/commit/acfacd78f070dde22c4ad57c7bdea24ffa8fe8e3))
* **board:** add middleware support with configurable options ([3bea63c](https://github.com/YuniqueUnic/elizabeth/commit/3bea63ce588b844ee058b78685fbc9a398b63ba4))
* **board:** add room tokens table and related queries ([1d32176](https://github.com/YuniqueUnic/elizabeth/commit/1d32176cff635e16cb607e2d41b076043fdc0fef))
* **board:** add RoomPermission type annotation in SQL queries and models ([482240a](https://github.com/YuniqueUnic/elizabeth/commit/482240a3577dce337399b801f2754b8fb9963318))
* **board:** add slug column to rooms table and update related queries ([473c42e](https://github.com/YuniqueUnic/elizabeth/commit/473c42efa5ea6661020af579cfb1783b09087dad))
* **board:** add WebSocket broadcaster and real-time event broadcasting ([174c89b](https://github.com/YuniqueUnic/elizabeth/commit/174c89b194789879a30787416d2a524bcf4f6e6f))
* **board:** add WebSocket support for real-time communication ([2cbe502](https://github.com/YuniqueUnic/elizabeth/commit/2cbe502a9cbef3340cae324bbfe59a664a141387))
* **board:** enable logging feature and fix related compilation issues ([3ca8145](https://github.com/YuniqueUnic/elizabeth/commit/3ca8145eae16f62c7ccfcaec6beb5df09650e3ed))
* **board:** implement centralized application configuration and constants management ([cf814c8](https://github.com/YuniqueUnic/elizabeth/commit/cf814c868105a3c78c2295ef218a1d663d9029b4))
* **board:** implement chunked upload functionality ([e425dde](https://github.com/YuniqueUnic/elizabeth/commit/e425dde640c3ca9407041f11890082a3925f1a09))
* **board:** implement chunked upload functionality and progress tracking ([b25584a](https://github.com/YuniqueUnic/elizabeth/commit/b25584aa2fdec0138475b3059f9e41385e0fc785))
* **board:** make room GC task configurable with interval and batch limits ([0365b6c](https://github.com/YuniqueUnic/elizabeth/commit/0365b6c7ef2d0bd92b93f0d9009d312f833544ef))
* **board:** redesign database schema and enhance documentation structure ([08babfe](https://github.com/YuniqueUnic/elizabeth/commit/08babfe89075701d2152d5eb580ff219b57064b5))
* **board:** refine room status enum and remove unused attributes ([ecdd706](https://github.com/YuniqueUnic/elizabeth/commit/ecdd706cb8f0a7096adad714758444199a5e4e35))
* **board:** remove `RoomDefaults` and simplify state management ([6d8b17c](https://github.com/YuniqueUnic/elizabeth/commit/6d8b17cc3a8c789a0a83adc38dd6b0d8e8021250))
* **board:** unify Room model for DB and API layers ([cfc0a97](https://github.com/YuniqueUnic/elizabeth/commit/cfc0a97c2c1baa1ede9b11b99da6932c38e33dfb))
* **board:** update room status to use enum with sqlx type mapping ([1821790](https://github.com/YuniqueUnic/elizabeth/commit/182179014f9b14dc4e3f970f8bb3d2a9c8a46ae2))
* **build:** optimize Dockerfiles with cargo-chef and improve caching ([508b023](https://github.com/YuniqueUnic/elizabeth/commit/508b023aefdb3473e6830ee3a92a5605a9632597))
* **build:** resolve frontend build issues and enhance documentation ([7508c72](https://github.com/YuniqueUnic/elizabeth/commit/7508c72f44e7959cbb27d717092fa12e39328d57))
* **chat:** add diff mode support to markdown editor ([fa92581](https://github.com/YuniqueUnic/elizabeth/commit/fa925814028d7732c1d2510e7b10f1dd696d4d03))
* **chat:** enhance chat components with responsive design and new features ([fef4969](https://github.com/YuniqueUnic/elizabeth/commit/fef4969ae6d4b28f5ad58c7df1d29bd7f10736e6))
* **chat:** refactor theme management in chat components ([2a21b85](https://github.com/YuniqueUnic/elizabeth/commit/2a21b859aa5548fb9d24aebfb614b203a2699685))
* **chunked-upload:** implement database schema and models for chunked uploads ([5e24e5c](https://github.com/YuniqueUnic/elizabeth/commit/5e24e5cb76b4a45bbaa9e2a4abe75f0198cd2d5c))
* **chunked-upload:** implement file merging endpoint and update dependencies ([bef2275](https://github.com/YuniqueUnic/elizabeth/commit/bef2275a48fe2d6399d489443ec9e8aeb90363b5))
* **code:** implement Shiki syntax highlighting with line numbers ([2cdd8a4](https://github.com/YuniqueUnic/elizabeth/commit/2cdd8a492563396fbf014d43024b8b07286bb93d))
* **config:** add GC configuration for room cleanup ([0365b6c](https://github.com/YuniqueUnic/elizabeth/commit/0365b6c7ef2d0bd92b93f0d9009d312f833544ef))
* **config:** enhance bacon.toml with verbose logging ([8cd2438](https://github.com/YuniqueUnic/elizabeth/commit/8cd24381ba4c2fedf32980d10fa53812ed112a7b))
* **config:** extract default JWT secret to constant ([f75e5ae](https://github.com/YuniqueUnic/elizabeth/commit/f75e5aebce6160e544c10eb11f6e963e69b19750))
* **config:** implement robust API URL parsing and configuration ([95c8a1c](https://github.com/YuniqueUnic/elizabeth/commit/95c8a1c72fb6a8a31f823f4ab0e36e6bd9e7b0e3))
* **config:** implement strict config file validation and priority logic ([dfb5f5c](https://github.com/YuniqueUnic/elizabeth/commit/dfb5f5c5475dc5627576d2dd9c7d85d18a956089))
* **config:** introduce backend.yaml for centralized configuration ([b842a07](https://github.com/YuniqueUnic/elizabeth/commit/b842a07cf411e0ba7b604951626b8f15defd6f8d))
* **config:** separate internal and public API URLs for better deployment flexibility ([5fa62d2](https://github.com/YuniqueUnic/elizabeth/commit/5fa62d2a6a8f43c42ad2781ff5abcf173bdf3b00))
* **config:** update autocorrect configuration and schema ([2c0576d](https://github.com/YuniqueUnic/elizabeth/commit/2c0576dcbb99c20efbb5a67c49918eff70a55da8))
* **config:** update bacon.toml and justfile for improved background handling and database reset ([421a8ca](https://github.com/YuniqueUnic/elizabeth/commit/421a8cac2a0411b35d0f3e03de113970ecda9c76))
* **content:** add message creation API and WebSocket connection handling ([2cb8238](https://github.com/YuniqueUnic/elizabeth/commit/2cb82385d9069a7c7fcfa443e1b82fc3e1341fbf))
* **copy:** enhance message copying functionality with metadata options ([0398891](https://github.com/YuniqueUnic/elizabeth/commit/0398891ac50af76f4f938128341bc58522591076))
* **deploy:** add WebSocket URL configuration ([8a38fad](https://github.com/YuniqueUnic/elizabeth/commit/8a38fade4a75cc4c705676c85c32fc07480de0b2))
* **deploy:** enhance service management with configurable ports ([8a38fad](https://github.com/YuniqueUnic/elizabeth/commit/8a38fade4a75cc4c705676c85c32fc07480de0b2))
* **docker:** add dockerignore, backend and frontend Dockerfiles, and Makefile for deployment ([a08aff6](https://github.com/YuniqueUnic/elizabeth/commit/a08aff615eb6dc9e08e64ef0bac33a5610d32441))
* **docs:** refactor TASKs.md to detailed frontend UI testing plan ([8cd2438](https://github.com/YuniqueUnic/elizabeth/commit/8cd24381ba4c2fedf32980d10fa53812ed112a7b))
* **docs:** update integration progress and API completion reports ([96e22b2](https://github.com/YuniqueUnic/elizabeth/commit/96e22b2d7101d11eb78ab6183b804fb8d37d66f3))
* **docs:** update TASKs.md and add frontend functionality issues documentation ([735420e](https://github.com/YuniqueUnic/elizabeth/commit/735420e38cc72007e61d9f99cd88562ff00fe8b2))
* **editor:** replace react-md-editor with mdxeditor for enhanced functionality ([9e93d8a](https://github.com/YuniqueUnic/elizabeth/commit/9e93d8a7f49620143dbc6f3322692856be302704))
* Enhance authentication and token management ([242ff6d](https://github.com/YuniqueUnic/elizabeth/commit/242ff6d33da4e76506754118980201271327e3d2))
* enhance messaging system with comprehensive UI tests and data-testid attributes ([e35d7a6](https://github.com/YuniqueUnic/elizabeth/commit/e35d7a676c32340634ce53c4217f6fe906245948))
* enhance room deletion and content management in handlers ([6454734](https://github.com/YuniqueUnic/elizabeth/commit/6454734d9e1bca247a185ceef1e500f7499ded42))
* **file-service:** add support for uploading URLs as file content ([94522e6](https://github.com/YuniqueUnic/elizabeth/commit/94522e64e353b8f2b18349687b497d47b3fffa1b))
* **files:** add markdown generation and insertion capabilities ([9e93d8a](https://github.com/YuniqueUnic/elizabeth/commit/9e93d8a7f49620143dbc6f3322692856be302704))
* **files:** add protocol selection to URL upload dialog ([bbaf472](https://github.com/YuniqueUnic/elizabeth/commit/bbaf47299fc47e7ef534bc3abcc33b13ecc6c836))
* **files:** implement file content preview for text-based files ([76d3ab5](https://github.com/YuniqueUnic/elizabeth/commit/76d3ab5d9a902e172b35bd6e8a1bc4970505e005))
* **files:** implement Shiki-based syntax highlighting ([2c32236](https://github.com/YuniqueUnic/elizabeth/commit/2c322366ebe9b9516bd45df6fa546dfb721e5184))
* **files:** track active uploads in app store ([f9aec2e](https://github.com/YuniqueUnic/elizabeth/commit/f9aec2ec79e13a28aa58767fe9b03434fa1eb252))
* **frontend:** configure internal API URL for server-side rewrites ([e2915aa](https://github.com/YuniqueUnic/elizabeth/commit/e2915aa77eabcd389629ec9edc0aa363abf7e757))
* **frontend:** enhance Elizabeth with new features and responsive design ([de61eaa](https://github.com/YuniqueUnic/elizabeth/commit/de61eaa67f10e4db958cc6f1a5e7b96405e0a20e))
* **gateway:** add IPv6 support and update API routing configuration ([8afbf23](https://github.com/YuniqueUnic/elizabeth/commit/8afbf23820399a0bcd72d7878070dcbedbd99c8c))
* **gitignore:** add docs-prompt to ignored files ([2c8a50f](https://github.com/YuniqueUnic/elizabeth/commit/2c8a50fb4e32ca524f93a6c331d198f474150571))
* **help:** add help dialog component and update project configuration documentation ([7fa20c4](https://github.com/YuniqueUnic/elizabeth/commit/7fa20c4777d2c1ce4b20aabbc024b710de2792e5))
* **home-page:** add password confirmation and visibility toggle for room creation ([4b30e6c](https://github.com/YuniqueUnic/elizabeth/commit/4b30e6c3d60b85e5afd14b039bdf9910bbdb4a61))
* **image-auth:** improve image loading with retry mechanism ([d6e222d](https://github.com/YuniqueUnic/elizabeth/commit/d6e222d8c7e09547df2babc48c4b4d0c0f0981f6))
* implement comprehensive Playwright UI testing framework ([6dc1f7a](https://github.com/YuniqueUnic/elizabeth/commit/6dc1f7a7ffecb8eba847eb3982073a7796b251c7))
* implement dropdown menu component and enhance file download functionality; add fullscreen support to file preview modal ([48dff91](https://github.com/YuniqueUnic/elizabeth/commit/48dff916777df282c360254dbd6cb47cdd842104))
* Implement message service for chat operations ([e63d6bd](https://github.com/YuniqueUnic/elizabeth/commit/e63d6bd524499c87298218cb476c2e6c3e4148a7))
* integrate ESLint configuration and enhance component testing ([17a4bae](https://github.com/YuniqueUnic/elizabeth/commit/17a4bae84453a27b3b59fe1030f4218913e3cd10))
* **logrs:** replace `tracing` crate with custom `logrs` logging facade ([0c1f3ab](https://github.com/YuniqueUnic/elizabeth/commit/0c1f3ab7ee50a8ad5a8db719d3aa168c708c8eef))
* **modal:** extend fullscreen mode to all file types ([2c32236](https://github.com/YuniqueUnic/elizabeth/commit/2c322366ebe9b9516bd45df6fa546dfb721e5184))
* **protocol:** add board protocol crate with TypeScript bindings ([04cc7d1](https://github.com/YuniqueUnic/elizabeth/commit/04cc7d156ccfd34a8057abd1d329954ccfaa0849))
* **protocol:** add chunked upload and token management type definitions ([edaf021](https://github.com/YuniqueUnic/elizabeth/commit/edaf02129bab62217ea1efb11897fcac8bda0ffc))
* **protocol:** add cleanup response structure ([8a38fad](https://github.com/YuniqueUnic/elizabeth/commit/8a38fade4a75cc4c705676c85c32fc07480de0b2))
* **protocol:** add optional file hash to chunk upload request ([8a38fad](https://github.com/YuniqueUnic/elizabeth/commit/8a38fade4a75cc4c705676c85c32fc07480de0b2))
* **protocol:** add refresh token support to IssueTokenResponse ([8a38fad](https://github.com/YuniqueUnic/elizabeth/commit/8a38fade4a75cc4c705676c85c32fc07480de0b2))
* **protocol:** add refresh token support to room token claims ([8a38fad](https://github.com/YuniqueUnic/elizabeth/commit/8a38fade4a75cc4c705676c85c32fc07480de0b2))
* **protocol:** add upload status query parameters ([8a38fad](https://github.com/YuniqueUnic/elizabeth/commit/8a38fade4a75cc4c705676c85c32fc07480de0b2))
* **protocol:** add UUID dependency and DTO modules for board protocol ([9808472](https://github.com/YuniqueUnic/elizabeth/commit/98084727a79f55a116de12fb24cbe4cc6f4e36b1))
* **protocol:** enhance chunk upload status information ([8a38fad](https://github.com/YuniqueUnic/elizabeth/commit/8a38fade4a75cc4c705676c85c32fc07480de0b2))
* **protocol:** enhance reserved file information ([8a38fad](https://github.com/YuniqueUnic/elizabeth/commit/8a38fade4a75cc4c705676c85c32fc07480de0b2))
* **protocol:** enhance upload status response ([8a38fad](https://github.com/YuniqueUnic/elizabeth/commit/8a38fade4a75cc4c705676c85c32fc07480de0b2))
* **room-permissions:** update redirect handling with store integration ([c38e06c](https://github.com/YuniqueUnic/elizabeth/commit/c38e06cd635d4298f2fdf1c300c362e62751fbe8))
* **room:** add confirm password field and validation ([aea85af](https://github.com/YuniqueUnic/elizabeth/commit/aea85af0a0bb9770b1cb109846c08e46d760725e))
* **room:** adjust room expiration logic and remove never-expire option ([33d5664](https://github.com/YuniqueUnic/elizabeth/commit/33d5664065200a598eb477a295918d4309ea1c2c))
* **room:** implement automatic release of expired private room names ([b84f488](https://github.com/YuniqueUnic/elizabeth/commit/b84f488272ea62c6a326e73ed548700e52dc196e))
* **services:** add manage_services.sh for backend and frontend management ([78626b4](https://github.com/YuniqueUnic/elizabeth/commit/78626b48774022f0e88c9ec384fe915fbfc76673))
* **sqlx:** add and update SQLx query files for room and token management ([4060d6f](https://github.com/YuniqueUnic/elizabeth/commit/4060d6ff1a03a8d6fda171c49fd120c040406b83))
* **store:** add global redirect state and upload tracking ([f9aec2e](https://github.com/YuniqueUnic/elizabeth/commit/f9aec2ec79e13a28aa58767fe9b03434fa1eb252))
* **store:** add server-side message synchronization ([8a38fad](https://github.com/YuniqueUnic/elizabeth/commit/8a38fade4a75cc4c705676c85c32fc07480de0b2))
* **store:** enhance Zustand store with JSON storage and immer support ([374d1ee](https://github.com/YuniqueUnic/elizabeth/commit/374d1ee40ef32060aef66a8eef25a15bbbccdc62))
* **ui:** add real-time room synchronization ([8a38fad](https://github.com/YuniqueUnic/elizabeth/commit/8a38fade4a75cc4c705676c85c32fc07480de0b2))
* **ui:** enhance message handling and deletion confirmation dialogs ([28b7de4](https://github.com/YuniqueUnic/elizabeth/commit/28b7de416f918ea655aa77af0a079f04966096d9))
* **ui:** implement comprehensive UI/UX optimizations for Elizabeth frontend ([ee53415](https://github.com/YuniqueUnic/elizabeth/commit/ee534151b4b3999ca04be7dc305e9e6d6aa5ea0f))
* **vscode:** add VSCode settings for Tailwind CSS and linting configurations ([b80ff4f](https://github.com/YuniqueUnic/elizabeth/commit/b80ff4f43cf3317898d6aa8af3fa55a7f7420a03))
* **vscode:** update settings for tailwindcss and cva support ([2535a51](https://github.com/YuniqueUnic/elizabeth/commit/2535a51b73b42e126a27f3c5b63ad8f538a1da92))
* **web:** add getRaw method to api client for manual response handling ([21be19b](https://github.com/YuniqueUnic/elizabeth/commit/21be19bad830b3734cab2e53bb5caf6799155639))
* **web:** add roomName prop to FileContentPreview for token authentication ([21be19b](https://github.com/YuniqueUnic/elizabeth/commit/21be19bad830b3734cab2e53bb5caf6799155639))
* **web:** add useRoomEvents hook for real-time room event handling ([174c89b](https://github.com/YuniqueUnic/elizabeth/commit/174c89b194789879a30787416d2a524bcf4f6e6f))
* **web:** fix file name overflow in file card display ([8cd2438](https://github.com/YuniqueUnic/elizabeth/commit/8cd24381ba4c2fedf32980d10fa53812ed112a7b))
* **web:** implement configurable Enter key behavior in markdown editor ([8cd2438](https://github.com/YuniqueUnic/elizabeth/commit/8cd24381ba4c2fedf32980d10fa53812ed112a7b))
* **web:** improve message posting reliability ([8cd2438](https://github.com/YuniqueUnic/elizabeth/commit/8cd24381ba4c2fedf32980d10fa53812ed112a7b))
* **web:** optimize room access initialization flow ([8cd2438](https://github.com/YuniqueUnic/elizabeth/commit/8cd24381ba4c2fedf32980d10fa53812ed112a7b))
* **web:** refine file filtering logic in fileService ([8cd2438](https://github.com/YuniqueUnic/elizabeth/commit/8cd24381ba4c2fedf32980d10fa53812ed112a7b))
* **ws:** enhance WebSocket event payload structure ([8a38fad](https://github.com/YuniqueUnic/elizabeth/commit/8a38fade4a75cc4c705676c85c32fc07480de0b2))
* **ws:** implement WebSocket URL resolution utility ([8a38fad](https://github.com/YuniqueUnic/elizabeth/commit/8a38fade4a75cc4c705676c85c32fc07480de0b2))


### Bug Fixes

* allow longer room identifiers (with UUID) for private slug verification ([e5bdb3f](https://github.com/YuniqueUnic/elizabeth/commit/e5bdb3fef086e36995572e8b5f7cffdba4ccbeca))
* **api:** improve chunked upload status query parameter handling ([8a38fad](https://github.com/YuniqueUnic/elizabeth/commit/8a38fade4a75cc4c705676c85c32fc07480de0b2))
* **api:** move WebSocket endpoint under API prefix ([6c47375](https://github.com/YuniqueUnic/elizabeth/commit/6c47375e5d11fc5ced8d0536f0c79bec1dae8721))
* **auth:** add dedicated room password verification function ([0043c9a](https://github.com/YuniqueUnic/elizabeth/commit/0043c9a5d8e02400c81614ac3106d4cfc1db1a8a))
* **auth:** implement token expiration check and cleanup ([257e561](https://github.com/YuniqueUnic/elizabeth/commit/257e56143af6da3636c97ad0e266de4ea645f210))
* **backend:** update Dockerfile to use debian trixie for glibc compatibility ([8afbf23](https://github.com/YuniqueUnic/elizabeth/commit/8afbf23820399a0bcd72d7878070dcbedbd99c8c))
* **board:** add health check and status endpoints ([faf2832](https://github.com/YuniqueUnic/elizabeth/commit/faf2832c73c0bf55487b7c3efae3376f01cb2633))
* **board:** add sqlx query files for room operations ([e7b79bf](https://github.com/YuniqueUnic/elizabeth/commit/e7b79bf8950bd8bdfdf11ca22da77e3f08bbda32))
* **board:** adjust room token validation logic for refresh tokens ([5c8721b](https://github.com/YuniqueUnic/elizabeth/commit/5c8721b5d8adacc2c11ed22f6585e4906f8b9f7e))
* **board:** cast is_revoked as integer in refresh token query ([f75e5ae](https://github.com/YuniqueUnic/elizabeth/commit/f75e5aebce6160e544c10eb11f6e963e69b19750))
* **board:** rename default database file from 'database.db' to 'app.db' ([a836cf7](https://github.com/YuniqueUnic/elizabeth/commit/a836cf75a3088b7914b05ca730d1cc09f948f090))
* **board:** replace numeric permission with RoomPermission enum ([ac9504a](https://github.com/YuniqueUnic/elizabeth/commit/ac9504a9c689be2533ca8a78e77c02665f41f263))
* **board:** update default database path and use config-provided URL ([e14afb0](https://github.com/YuniqueUnic/elizabeth/commit/e14afb00c0fc363d1e0a0b47f0f68ffdcb2a1659))
* **board:** use file_name from database instead of extracting from path ([21be19b](https://github.com/YuniqueUnic/elizabeth/commit/21be19bad830b3734cab2e53bb5caf6799155639))
* bypass system proxy for local debugging and enhance web server command ([c777fc2](https://github.com/YuniqueUnic/elizabeth/commit/c777fc2c010095b69c44428488da2893800c569c))
* **cache:** resolve stale HTML caching and mixed content issues ([8a39f81](https://github.com/YuniqueUnic/elizabeth/commit/8a39f818aba64efb048b248ea125deb76fade63e))
* **chat:** correct message sending logic and prevent premature toast notification ([df9aadb](https://github.com/YuniqueUnic/elizabeth/commit/df9aadb683323b9f4b9ce0d911376d1cb824890e))
* **chat:** enhance image authentication with reactive URL computation ([fb019e5](https://github.com/YuniqueUnic/elizabeth/commit/fb019e517ee9c04879db3ee1a62e4b0f91157df9))
* **chat:** enhance markdown editor with improved styling and toolbar ([dd25bab](https://github.com/YuniqueUnic/elizabeth/commit/dd25bab781b758d2c37938769894c8f3f4be75ad))
* **ci:** fix release-please config and add cargo lock update ([8558cde](https://github.com/YuniqueUnic/elizabeth/commit/8558cdef3739b0dc581d7a951622a44abca4acd5))
* **config:** add room share disabled lock duration configuration ([b84f488](https://github.com/YuniqueUnic/elizabeth/commit/b84f488272ea62c6a326e73ed548700e52dc196e))
* **config:** correct SQLite database URL format in backend configuration ([8afbf23](https://github.com/YuniqueUnic/elizabeth/commit/8afbf23820399a0bcd72d7878070dcbedbd99c8c))
* **config:** make service script paths relative to script location ([f79ffd2](https://github.com/YuniqueUnic/elizabeth/commit/f79ffd28f5ce322788b816b8d8eb854aa6587ce7))
* docker postgresql ([dab33db](https://github.com/YuniqueUnic/elizabeth/commit/dab33db8aa8d550e071c2d048c8005480657ac45))
* **docker:** update backend URL and restrict backend access to internal network ([72bb92c](https://github.com/YuniqueUnic/elizabeth/commit/72bb92c045411915f651cd6f9a886f86fffd063c))
* **e2e:** update InputElement to handle contenteditable elements properly ([4050e89](https://github.com/YuniqueUnic/elizabeth/commit/4050e898f307773547b102291164566d07d07af2))
* **editor:** add markdown syntax support for source mode and preserve formatting characters ([f54ff6f](https://github.com/YuniqueUnic/elizabeth/commit/f54ff6f99df41e35793613f0f5f8c58e0c6350a5))
* **editor:** add source mode toggle to minimal tiptap editor ([e855aa8](https://github.com/YuniqueUnic/elizabeth/commit/e855aa88d07e59f8ccdec2338de2f182b48ba959))
* **editor:** change message sending to Ctrl/Cmd+Enter combination ([ac0f537](https://github.com/YuniqueUnic/elizabeth/commit/ac0f53757a09fd852d601e5b19662bb8ca8afebe))
* **editor:** enhance Enter key handling with proper closure management ([4050e89](https://github.com/YuniqueUnic/elizabeth/commit/4050e898f307773547b102291164566d07d07af2))
* **editor:** remove height prop and update container styling ([dd25bab](https://github.com/YuniqueUnic/elizabeth/commit/dd25bab781b758d2c37938769894c8f3f4be75ad))
* **env:** resolve mixed content by unifying frontend and backend API paths ([ca04075](https://github.com/YuniqueUnic/elizabeth/commit/ca040751d4896d327709d7d9d9c3cdd1849d9608))
* **frontend:** resolve production build asset serving and static file copying ([dfb5f5c](https://github.com/YuniqueUnic/elizabeth/commit/dfb5f5c5475dc5627576d2dd9c7d85d18a956089))
* **frontend:** resolve room creation failure and mixed content issues ([7c72df4](https://github.com/YuniqueUnic/elizabeth/commit/7c72df408f5c9b33ab51ffa67aac1170122f7adc))
* **frontend:** update healthcheck to use 127.0.0.1 instead of localhost ([8afbf23](https://github.com/YuniqueUnic/elizabeth/commit/8afbf23820399a0bcd72d7878070dcbedbd99c8c))
* **markdown:** improve file URL handling with room tokens ([9e93d8a](https://github.com/YuniqueUnic/elizabeth/commit/9e93d8a7f49620143dbc6f3322692856be302704))
* regenerate pnpm-lock.yaml ([4131ddc](https://github.com/YuniqueUnic/elizabeth/commit/4131ddc4888d9ed6908203f2b37f8df5f16a4247))
* **room:** ensure correct handling of room expiry dates ([76d3ab5](https://github.com/YuniqueUnic/elizabeth/commit/76d3ab5d9a902e172b35bd6e8a1bc4970505e005))
* **room:** handle undefined room expiration in update settings ([8a38fad](https://github.com/YuniqueUnic/elizabeth/commit/8a38fade4a75cc4c705676c85c32fc07480de0b2))
* **room:** send empty string to clear password instead of null ([2c32236](https://github.com/YuniqueUnic/elizabeth/commit/2c322366ebe9b9516bd45df6fa546dfb721e5184))
* **room:** synchronize permissions state on prop updates ([76d3ab5](https://github.com/YuniqueUnic/elizabeth/commit/76d3ab5d9a902e172b35bd6e8a1bc4970505e005))
* **sqlx:** ensure compile-time query validation passes ([3ca8145](https://github.com/YuniqueUnic/elizabeth/commit/3ca8145eae16f62c7ccfcaec6beb5df09650e3ed))
* **tests:** adjust integration tests for automatic room creation logic ([5a9f513](https://github.com/YuniqueUnic/elizabeth/commit/5a9f513d64cd23b02f76bc34d348b81efaa16b63))
* **tiptap-viewer:** improve markdown parsing and image loading ([d6e222d](https://github.com/YuniqueUnic/elizabeth/commit/d6e222d8c7e09547df2babc48c4b4d0c0f0981f6))
* **tiptap:** simplify markdown configuration and content handling ([257e561](https://github.com/YuniqueUnic/elizabeth/commit/257e56143af6da3636c97ad0e266de4ea645f210))
* **top-bar:** adjust button sizing for mobile responsiveness ([ac0f537](https://github.com/YuniqueUnic/elizabeth/commit/ac0f53757a09fd852d601e5b19662bb8ca8afebe))
* **ts:** enable TypeScript build error checking ([8a38fad](https://github.com/YuniqueUnic/elizabeth/commit/8a38fade4a75cc4c705676c85c32fc07480de0b2))
* update permission management logic to ensure DELETE permission includes all other permissions; add file_name field to RoomContent model for original file names; adjust file storage path to use room_id instead of room_name; improve file download logic to use original file names; enhance permission dependency checks in update_permissions handler. ([b2675fc](https://github.com/YuniqueUnic/elizabeth/commit/b2675fcf321ff2c6868dd0fb37ed1704db9351a3))
* use actual listener address in server startup ([d5e530e](https://github.com/YuniqueUnic/elizabeth/commit/d5e530e2b16dc5e504e993115957bfb07edde692))
* use actual listener address in server startup ([4fba44e](https://github.com/YuniqueUnic/elizabeth/commit/4fba44e895c201b7074a2a899e135cbb005f19ac))
* use validate_identifier for room identifiers in content handlers ([3c178c5](https://github.com/YuniqueUnic/elizabeth/commit/3c178c5276f7094370ab8e4d200dc41d315020f9))
* **web:** add enter key support for room creation and improve error handling ([675df01](https://github.com/YuniqueUnic/elizabeth/commit/675df01bcce82e6baeaa4533598e5e96cdbb8d43))
* **web:** copy full download URL with domain to clipboard ([21be19b](https://github.com/YuniqueUnic/elizabeth/commit/21be19bad830b3734cab2e53bb5caf6799155639))
* **web:** detect password changes in room settings form ([21be19b](https://github.com/YuniqueUnic/elizabeth/commit/21be19bad830b3734cab2e53bb5caf6799155639))
* **web:** enforce password confirmation only when password is hidden ([e0b6ac9](https://github.com/YuniqueUnic/elizabeth/commit/e0b6ac903aee4fd1b496b291c04f08a4388a7d0c))
* **web:** generate download URLs for file content in backend conversion ([21be19b](https://github.com/YuniqueUnic/elizabeth/commit/21be19bad830b3734cab2e53bb5caf6799155639))
* **web:** improve error messages for room settings updates ([21be19b](https://github.com/YuniqueUnic/elizabeth/commit/21be19bad830b3734cab2e53bb5caf6799155639))
* **web:** improve WebSocket hook callback handling ([f79ffd2](https://github.com/YuniqueUnic/elizabeth/commit/f79ffd28f5ce322788b816b8d8eb854aa6587ce7))
* **web:** pass roomName to convertFile to generate correct download URLs ([21be19b](https://github.com/YuniqueUnic/elizabeth/commit/21be19bad830b3734cab2e53bb5caf6799155639))
* **web:** refresh JWT automatically when room password is changed ([21be19b](https://github.com/YuniqueUnic/elizabeth/commit/21be19bad830b3734cab2e53bb5caf6799155639))
* **web:** update dependency array in RoomPage useEffect hook ([f79ffd2](https://github.com/YuniqueUnic/elizabeth/commit/f79ffd28f5ce322788b816b8d8eb854aa6587ce7))
* **web:** use api client for file fetching to automatically add auth token ([21be19b](https://github.com/YuniqueUnic/elizabeth/commit/21be19bad830b3734cab2e53bb5caf6799155639))


### Performance Improvements

* **web:** make sidebar skeleton widths deterministic ([f79ffd2](https://github.com/YuniqueUnic/elizabeth/commit/f79ffd28f5ce322788b816b8d8eb854aa6587ce7))


### Code Refactoring

* **board-protocol:** standardize TypeScript binding exports ([ee2c37e](https://github.com/YuniqueUnic/elizabeth/commit/ee2c37e1d55b5e2faffcb49d831a34bbb263b59b))
* **board:** remove generated TypeScript bindings ([a96d2cb](https://github.com/YuniqueUnic/elizabeth/commit/a96d2cb512014653a88c0af789cf231b2ba87c8e))
* **database:** update SQLite URL format to include mode parameter ([f79ffd2](https://github.com/YuniqueUnic/elizabeth/commit/f79ffd28f5ce322788b816b8d8eb854aa6587ce7))

## [Unreleased]

## Elizabeth Board - [0.3.0](https://github.com/YuniqueUnic/elizabeth/releases/tag/v0.3.0) - 2025-11-03

### Added

- _(board)_ redesign database schema and enhance documentation structure
- _(auth)_ enhance token management for room access and refresh
- enhance room deletion and content management in handlers
- _(ui)_ enhance message handling and deletion confirmation dialogs
- Add room identifier validation and enhance token management
- Enhance authentication and token management
- _(docs)_ update integration progress and API completion reports
- Implement message service for chat operations
- _(api)_ integrate backend API for room management and authentication
- _(logrs)_ replace `tracing` crate with custom `logrs` logging facade
- _(auth)_ add comprehensive token verification and management tests
- _(board)_ implement centralized application configuration and constants
  management
- _(board)_ remove `RoomDefaults` and simplify state management
- _(board)_ add middleware support with configurable options
- _(chunked-upload)_ implement file merging endpoint and update dependencies
- _(board)_ implement chunked upload functionality
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

- _(board)_ use file_name from database instead of extracting from path
- update permission management logic to ensure DELETE permission includes all
  other permissions; add file_name field to RoomContent model for original file
  names; adjust file storage path to use room_id instead of room_name; improve
  file download logic to use original file names; enhance permission dependency
  checks in update_permissions handler.
- use validate_identifier for room identifiers in content handlers
- allow longer room identifiers (with UUID) for private slug verification
- _(board)_ replace numeric permission with RoomPermission enum
- _(board)_ rename default database file from 'database.db' to 'app.db'
- _(board)_ update default database path and use config-provided URL
- _(board)_ add sqlx query files for room operations
- _(board)_ add OpenAPI documentation and scalar UI
- _(board)_ add status endpoint

### Other

- streamline chunked upload handling and enhance UI component testing
- _(board)_ remove unused permission validator and related code
- _(board)_ implement unified error handling and validation frameworks
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

## Elizabeth Board - [0.3.0](https://github.com/YuniqueUnic/elizabeth/releases/tag/v0.3.0) - 2025-10-27

### Added

- _(logrs)_ replace `tracing` crate with custom `logrs` logging facade
- _(auth)_ add comprehensive token verification and management tests
- _(board)_ implement centralized application configuration and constants
  management
- _(board)_ remove `RoomDefaults` and simplify state management
- _(board)_ add middleware support with configurable options
- _(chunked-upload)_ implement file merging endpoint and update dependencies
- _(board)_ implement chunked upload functionality
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

- _(board)_ remove unused permission validator and related code
- _(board)_ implement unified error handling and validation frameworks
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
