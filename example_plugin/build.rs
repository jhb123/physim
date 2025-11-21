use rustc_version::version;

fn main() {
    let rustc_version = version().expect("Failed to get rustc version");
    let target =
        std::env::var("TARGET").expect("Cargo did not set TARGET (this should never happen)");
    let abi_info = format!("rustc:{}|target:{}", rustc_version, target);
    println!("cargo:rustc-env=ABI_INFO={}", abi_info);
}
