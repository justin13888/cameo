fn main() -> Result<(), Box<dyn std::error::Error>> {
    let src = "openapi/tmdb-api.json";
    println!("cargo::rerun-if-changed={src}");

    let file = std::fs::File::open(src)?;
    let spec = serde_json::from_reader(file)?;

    let mut generator = progenitor::Generator::default();
    let tokens = generator.generate_tokens(&spec)?;

    let ast = syn::parse2(tokens)?;
    let content = prettyplease::unparse(&ast);

    let out_file = std::path::Path::new(&std::env::var("OUT_DIR")?).join("tmdb_generated.rs");
    std::fs::write(&out_file, content)?;

    Ok(())
}
