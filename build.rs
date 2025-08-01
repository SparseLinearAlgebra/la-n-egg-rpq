fn main() {
    println!("cargo:rustc-link-lib=dylib=lagraphx");
    println!("cargo:rustc-link-search=native=vendor/LAGraph/build/experimental");

    println!("cargo:rustc-link-lib=dylib=lagraph");
    println!("cargo:rustc-link-search=native=vendor/LAGraph/build/src");
}
