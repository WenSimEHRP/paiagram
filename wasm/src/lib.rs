use anyhow::Context;
use ciborium::{from_reader, into_writer};
use typst_wasm_protocol::wasm_export;

mod collision;
mod input;
mod output;
mod types;
mod utils;
use input::{Network, NetworkConfig};
use output::Output;

#[wasm_export]
fn process(network_data: &[u8], config_data: &[u8]) -> Result<Vec<u8>, String> {
    process_internal(network_data, config_data).map_err(format_error_chain)
}

fn process_internal(network_data: &[u8], config_data: &[u8]) -> anyhow::Result<Vec<u8>> {
    let network: Network = from_reader(network_data).context("Failed to deserialize network")?;

    let config: NetworkConfig = from_reader(config_data).context("Failed to deserialize config")?;

    let mut output = Output::new(config);
    output
        .populate(network)
        .context("Failed to populate output from network and config")?;

    let mut serialized_result = Vec::new();
    into_writer(&output, &mut serialized_result).context("Failed to serialize output")?;

    Ok(serialized_result)
}

fn format_error_chain(error: anyhow::Error) -> String {
    let mut formatted_result = format!("Error: {error}");
    let mut current_error = error.source();
    let mut error_level = 1;
    while let Some(source_error) = current_error {
        formatted_result.push_str(&format!("\n  {error_level}. Caused by: {source_error}"));
        current_error = source_error.source();
        error_level += 1;
    }
    formatted_result
}
