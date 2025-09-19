## AsyncDebugger

AsyncDebugger is a desktop tool that provides **meaningful insights** into asynchronous Rust applications by leveraging
the `console-api` developed for `tokio-console`. It offers a clean, intuitive user interface to inspect tasks,
resources, and execution timelines in real time.

<hr>

## Features

- **Real-time task visualization**: See active, pending and completed asynchronous tasks
- **Resource usage inspection**: Monitor resources and CPU consumption and scheduling
- **Polling overview**: Tap into polls and get insights
- **Custom filtering**: Filter tasks by name, state or metadata tags
- **Resource overview**: Get an interactive insight into what, how and when were resources spawned

[Architecture](Architecture.md)
<hr>

## Requirements
- **Supported operating systems**: 
  - Linux
  - macOS
  - Windows


- **Software prerequisites**: 
  - npm (comes with Node.js) or yarn 
  - git
  - Rust Toolchain
<hr>

## Installation

1. Clone the repository
   ```bash
   git clone https://github.com/Wyliodrin/async-debugger.git
   cd AsyncDebugger
   ```
2. Install front-end dependencies
   ```bash
   npm install
   ```
3. Build and launch in development mode
   ```bash
   npm run tauri dev
   ```

<hr>

## Usage

1. Launch AsyncDebugger via `npm run tauri dev`.
2. Attach to one of our examples or your own Rust async application by using the Application Overview page in the UI
    - For your application, enable the console API by adding `console-subscriber` to your Cargo.toml and instrumenting
      your code:

    ```toml
    [dependencies]
    console-api = "0.4.1"
    ```

    ```rust
    #[tokio::main]
    async fn main() {
        console_subscriber::init();
        // Your application here...
    }
    ```

3. Observe tasks, timelines, and resource metrics
4. Apply filters and inspect stack traces to pinpoint performance bottlenecks or logical errors.

<hr>

## Examples

Example applications live in the `examples/` directory. To run an example:

```bash
  cargo run --example <example_name>
```

- `0s1s` spawns three asynchronous tasks:
    1. A producer that sends 1024-byte zero-filled chunks every 500 ms.
    2. A producer that sends 1024-byte one-filled chunks every 700 ms.
    3. A consumer that concurrently reads from both channels and logs incoming data.

It can be tracked at `localhost:6669`

- `tasks` waits for user input to start, then spawns three tasks that simulate work, wait on a common barrier, and then
  proceed together.

It can be tracked at `localhost:7777`
