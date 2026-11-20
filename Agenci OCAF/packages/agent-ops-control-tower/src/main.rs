fn main() -> Result<(), Box<dyn std::error::Error>> {
    ocaf_import_core::cli(&ocaf_ext_agent_ops_control_tower::PROFILE)
}
