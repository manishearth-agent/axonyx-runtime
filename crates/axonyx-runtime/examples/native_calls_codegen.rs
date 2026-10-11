//! Emit the isolated fixture backend; never overwrites an application's sources.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = include_str!("../tests/fixtures/native-backend.ax");
    let module = axonyx_core::ax_backend_codegen::compile_backend_ax_to_module(source)?;
    print!("{module}");
    Ok(())
}
