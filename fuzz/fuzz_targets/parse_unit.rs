#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        if let Ok(reg) = dimvexa_data::registry() {
            // Must never panic; a parsed expression must print and re-parse identically.
            if let Ok(e) = reg.parse(text) {
                let printed = e.to_string();
                let again = reg.parse(&printed).map(|x| x.to_string());
                assert_eq!(again.as_deref(), Ok(printed.as_str()));
            }
        }
    }
});
