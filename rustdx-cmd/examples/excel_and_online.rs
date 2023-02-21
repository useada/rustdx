use anyhow::Result;
use rustdx_cmd::fetch_code::*;

macro_rules! get {
    (sz) => {{
        let mut set = ::std::collections::HashSet::with_capacity(3000);
        get_sz_stocks(&mut set)?;
        set.into_iter().collect::<Vec<_>>()
    }};
    (sh, $a:literal, $b:literal) => {{
        let mut set = ::std::collections::HashSet::with_capacity(3000);
        get_sh_stocks(&mut set, $a, $b)?;
        set.into_iter().collect::<Vec<_>>()
    }};
}

/// sh8: 334
/// ["sh688001", "sh688002", "sh688003", "sh688004", "sh688005", "sh688006", "sh688007",
///  "sh688008", "sh688009", "sh688010"]
/// ["sh688787", "sh688788", "sh688789", "sh688793", "sh688798", "sh688799", "sh688800",
///  "sh688819", "sh688981", "sh689009"]
/// sh1: 1639
/// ["sh600000", "sh600004", "sh600006", "sh600007", "sh600008", "sh600009", "sh600010",
///  "sh600011", "sh600012", "sh600015"]
/// ["sh605398", "sh605399", "sh605488", "sh605499", "sh605500", "sh605507", "sh605577",
///  "sh605580", "sh605588", "sh605589"]
/// sz: 2488
/// ["sz000001", "sz000002", "sz000004", "sz000005", "sz000006", "sz000007", "sz000008",
///  "sz000009", "sz000010", "sz000011"]
/// ["sz301045", "sz301046", "sz301047", "sz301048", "sz301049", "sz301050", "sz301051",
///  "sz301052", "sz301053", "sz301055"]
fn main() -> Result<()> {
    let (sh8, sh1, sz) = (get!(sh, "8", "400"), get!(sh, "1", "1700"), get!(sz));
    println!(
        "sh8: {}\n{:?}\n{:?}",
        sh8.len(),
        &sh8[..10],
        &sh8[sh8.len() - 10..]
    );
    println!(
        "sh1: {}\n{:?}\n{:?}",
        sh1.len(),
        &sh1[..10],
        &sh1[sh1.len() - 10..]
    );
    println!(
        "sz: {}\n{:?}\n{:?}",
        sz.len(),
        &sz[..10],
        &sz[sz.len() - 10..]
    );

    Ok(())
}
