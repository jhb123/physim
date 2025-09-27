use rustc_version::version;

fn main() {
    let rustc_version = version().unwrap();
    let target = std::env::var("TARGET").unwrap(); // always set by Cargo
    let abi_info = format!("rustc:{}|target:{}", rustc_version, target);
    println!("cargo:rustc-env=ABI_INFO={}", abi_info);
}
