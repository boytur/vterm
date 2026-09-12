fn main() {
    #[cfg(windows)]
    let _ = embed_resource::compile("vterm.rc", embed_resource::NONE);
}
