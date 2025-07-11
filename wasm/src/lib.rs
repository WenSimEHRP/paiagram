use anyhow::Context;
use ciborium::{from_reader, into_writer};
use typst_wasm_protocol::wasm_export;

mod input;
mod output;
mod types;
mod coillision;
use input::{Network, NetworkConfig};
use output::Output;

#[wasm_export]
fn process(network: &[u8], config: &[u8]) -> Result<Vec<u8>, String> {
    process_internal(network, config).map_err(format_error_chain)
}

fn process_internal(network: &[u8], config: &[u8]) -> anyhow::Result<Vec<u8>> {
    let network: Network = from_reader(network).context("Failed to deserialize network")?;

    let config: NetworkConfig = from_reader(config).context("Failed to deserialize config")?;

    let output = Output::new(network, &config)
        .context("Failed to create output from network and config")?;

    let mut result = Vec::new();
    into_writer(&output, &mut result).context("Failed to serialize output")?;

    Ok(result)
}

fn format_error_chain(error: anyhow::Error) -> String {
    let mut result = format!("Error: {}", error);
    let mut current = error.source();
    let mut level = 1;
    while let Some(source) = current {
        result.push_str(&format!("\n  {}. Caused by: {}", level, source));
        current = source.source();
        level += 1;
    }
    result
}
