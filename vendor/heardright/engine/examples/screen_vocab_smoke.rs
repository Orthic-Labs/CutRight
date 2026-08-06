//! Live smoke for the AX screen-vocab harvest: prints term COUNT and timing
//! against the current frontmost window, plus the first few terms (dev-only
//! diagnostic; the product never logs term contents).
fn main() {
    println!(
        "ax_trusted={}",
        heardright_platform::macos::accessibility_trusted(false)
    );
    let t0 = std::time::Instant::now();
    let texts = heardright_platform::macos::window_text_harvest(12_000);
    let walk_ms = t0.elapsed().as_millis();
    let t1 = std::time::Instant::now();
    let terms = heardright_engine::screen_vocab::extract_terms(&texts);
    println!(
        "elements={} walk_ms={} extract_us={} terms={}",
        texts.len(),
        walk_ms,
        t1.elapsed().as_micros(),
        terms.len()
    );
    println!("sample: {:?}", &terms[..terms.len().min(15)]);
}
