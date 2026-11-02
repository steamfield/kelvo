//! Writes `src/core/generated/bindings.ts` without launching the app. Run by
//! `make bindings` and by CI's drift check.

use anyhow::Context;

fn main() -> anyhow::Result<()> {
    statglass_lib::export_bindings(&statglass_lib::specta_builder())
        .with_context(|| format!("exporting bindings to {}", statglass_lib::BINDINGS_PATH))?;
    println!("wrote {}", statglass_lib::BINDINGS_PATH);
    Ok(())
}
