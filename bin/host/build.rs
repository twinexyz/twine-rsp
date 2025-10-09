use sp1_build::{build_program_with_args, BuildArgs};

fn main() {
    let mut _build_args = BuildArgs::default();

    println!("running build script");

    #[cfg(feature = "devnet")]
    {
        _build_args.features = vec!["devnet".to_string()];
    }

    #[cfg(feature = "testnet")]
    {
        _build_args.features = vec!["testnet".to_string()];
    }

    #[cfg(feature = "mainnet")]
    {
        _build_args.features = vec!["mainnet".to_string()];
    }

    build_program_with_args("../client", _build_args.clone());
    build_program_with_args("../client-op", _build_args);
}
