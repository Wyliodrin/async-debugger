 # Architecture
 ```
                                                                                                                                                            ┌── common.rs/get_pid_hosting_at ────────┐
                                                                                                                           ┌── domain/application.rs/new ─> │                                        │
                                                                                                                           │                                └── common.rs/get_process_start_time ────┼───> state_manager/mod.rs/emit_update_applications
The user adds an app using the UI ─> commands/applications.rs/add_application ─> state_manager/mod.rs/add_application  ─> ├── state_manager/connection_manager.rs/connect_app ──────────────────────┤
                                                                                                                           └── state_manager/state.rs/store_app ─> the app is written into storage ──┤

```
# Repo Hierarchy

```
├── .cargo
├── Cargo.toml
├── examples
│   ├── 0s1s.rs -> example demonstrating timing utilities
│   ├── resources.rs -> example showing resource collection usage
│   └── tasks.rs -> example showing task management usage
├── .git
├── .gitignore
├── index.html
├── LICENSE
├── package.json
├── public
│   ├── tauri.svg
│   └── vite.svg
├── README.md
├── src
│   ├── App.vue -> Root Vue component
│   ├── assets
│   │   ├── logo.png
│   │   ├── logo-white.png
│   │   └── vue.svg
│   ├── layout
│   │   ├── AppLayout.vue -> Main application layout (header + sidebar)
│   │   ├── header -> Header components (UI controls, title)
│   │   └── sidebar -> Sidebar components (navigation)
│   ├── main.rs -> (Rust) optional native entry used by Tauri (if present)
│   ├── main.ts -> Frontend entry: mounts Vue app, installs plugins/router
│   ├── plugins
│   │   └── vuetify.ts -> Vuetify setup and theme configuration
│   ├── router
│   │   ├── index.ts -> Router instance creation
│   │   └── MainRoutes.ts -> Route definitions
│   ├── stores
│   │   ├── application.ts -> App-level state (meta, status)
│   │   ├── data.ts -> Domain data store (resources, metrics)
│   │   └── layout.ts -> UI layout state (sidebar open/closed)
│   ├── styles
│   │   └── timestamps.css -> Timestamp styling utilities
│   ├── types
│   │   ├── applications.d.ts
│   │   ├── async_ops.d.ts
│   │   ├── polls.d.ts
│   │   ├── resources.d.ts
│   │   └── tasks.d.ts
│   ├── views
│   │   ├── CPU.vue -> UPlot CPU usage view
│   │   ├── Polls.vue -> Polling configuration/view
│   │   ├── Resources.vue -> Resource overview view
│   │   ├── SystemInformation.vue -> System info view
│   │   └── Tasks.vue -> Task/process view
│   └── vite-env.d.ts
├── src-tauri
│   ├── build.rs -> Build script for Tauri/native build tasks
│   ├── capabilities
│   │   └── default.json -> Platform capability declarations
│   ├── Cargo.lock
│   ├── Cargo.toml -> Tauri backend crate manifest
│   ├── gen
│   │   └── schemas -> Generated schemas (IPC/config)
│   ├── .gitignore
│   ├── icons -> App icons for various platforms/sizes
│   └── src
│       ├── commands -> **commands that can be called by the front-end component.**
│       │   ├── applications.rs
│       │   └── tasks.rs
│       ├── common.rs -> Shared helpers and types used across backend
│       ├── domain -> Domain models and business logic
│       ├── error.rs -> Error types and handling utilities
│       ├── infra -> Infrastructure/OS adapters and platform-specific code
│       │   ├── guard.rs -> A guard that auto-writes a database file on drop
│       │   └── storage.rs -> Defines the `Storage` trait to unify reads/writes of all domain data.
│       ├── lib.rs -> Backend library entry (shared logic)
│       ├── main.rs -> Tauri bootstrap (registers commands, initializes app)
│       ├── mappers -> Converters between console_api types and our domain type
│       │   ├── async_ops.rs -> maps asyncronous operations
│       │   ├── poll.rs -> maps polls
│       │   ├── resource.rs -> maps resources
│       │   └── tasks.rs -> maps tasks
│       ├── state_manager -> In-memory or persistent state utilities
│       │   ├── connection_manager.rs -> Connection manager for remote applications
│       │   ├── database.rs -> Persistent, in-memory database backed by disk storage
│       │   └── state.rs -> Manages access to persistent storage and provides high-level methods
│       └── warnings.rs -> Warning definitions/emitters
│   └── tauri.conf.json -> Tauri configuration (windows, bundle, security)
├── tsconfig.json
├── tsconfig.node.json
├── vite.config.ts
├── .vscode
└── .zed
```