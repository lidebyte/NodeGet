fn main() {
    // ARM 32-bit targets lack native 64-bit atomic instructions.
    // QuickJS uses 64-bit atomics (js_atomics_op), which GCC emits as
    // __atomic_*_8 calls that live in libatomic.
    // Use rustc-link-arg to place -latomic at the END of the linker command,
    // after all .rlib files, so the linker can resolve the symbols.
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    if target_arch == "arm" {
        println!("cargo:rustc-link-arg=-latomic");
    }
}
