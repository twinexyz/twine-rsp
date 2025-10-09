use sp1_build::{build_program_with_args, BuildArgs};

fn main() {
    let mut _build_args = BuildArgs::default();

    _build_args.features = vec!["mainnet".to_string()];

    build_program_with_args("../client", _build_args);
}
