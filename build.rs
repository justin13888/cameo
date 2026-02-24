fn main() {
    let src = "openapi/tmdb-api.json";
    println!("cargo::rerun-if-changed={src}");

    let file = std::fs::File::open(src).unwrap();
    let spec = serde_json::from_reader(file).unwrap();

    let mut generator = progenitor::Generator::default();
    let tokens = generator.generate_tokens(&spec).unwrap();

    let ast = syn::parse2(tokens).unwrap();
    let content = prettyplease::unparse(&ast);

    let out_file =
        std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("tmdb_generated.rs");
    std::fs::write(&out_file, content).unwrap();
}
