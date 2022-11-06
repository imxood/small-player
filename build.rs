#[cfg(target_os = "windows")]
fn main() {
    /* 为了解决这个错误:

    atvis" "/NATVIS:C:\\Users\\maxu\\.rustup\\toolchains\\nightly-x86_64-pc-windows-msvc\\lib\\rustlib\\etc\\libstd.natvis"
      = note: rust-lld: error: undefined symbol: IID_ICodecAPI
              >>> referenced by D:\programs\vcpkg\buildtrees\ffmpeg\src\n4.4.1-d28c997f6f.clean\libavcodec\mfenc.c:1068
              >>>               avcodec.lib(mfenc.o):(mf_init)

              rust-lld: error: undefined symbol: IID_IMFMediaEventGenerator
              >>> referenced by D:\programs\vcpkg\buildtrees\ffmpeg\src\n4.4.1-d28c997f6f.clean\libavcodec\mfenc.c:996
              >>>               avcodec.lib(mfenc.o):(mf_unlock_async)

              rust-lld: error: undefined symbol: IID_IMFTransform
              >>> referenced by D:\programs\vcpkg\buildtrees\ffmpeg\src\n4.4.1-d28c997f6f.clean\libavcodec\mf_utils.c:633
              >>>               avcodec.lib(mf_utils.o):($LN56)

    */
    println!("cargo:rustc-link-lib=static=Mfuuid");
    println!("cargo:rustc-link-lib=static=Strmiids");
}

#[cfg(not(target_os = "windows"))]
fn main() {}
