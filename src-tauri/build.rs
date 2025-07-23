fn main() {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&["proto/debug.proto"], &["proto"])
        .expect("Failed to compile debug.proto");
    tauri_build::build()
}
