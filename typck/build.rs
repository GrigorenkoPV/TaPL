fn main() -> Result<(), Box<dyn std::error::Error>> {
    lalrpop::Configuration::new()
        .emit_rerun_directives(true)
        .force_build(true)
        .emit_report(true)
        .log_info()
        .set_in_dir("src")
        .set_out_dir("src")
        .process()
}
