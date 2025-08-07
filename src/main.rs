// Write a CLI program that call the bash file rails-new inside the bin folder.

// use std::process::Command;
mod docker_client;
mod rails_new;
use rails_new::{Cli, Commands};
use std::{io::Write, process::Command, io::ErrorKind};

use clap::Parser;

use crate::docker_client::DockerClient;

#[cfg_attr(all(unix, not(target_os = "macos")), path = "unix.rs")]
#[cfg_attr(any(windows, target_os = "macos"), path = "windows.rs")]
mod os_specific;

fn main() {
    let cli = Cli::parse();

    let ruby_version = cli.ruby_version;
    let rails_version = cli.rails_version.as_deref();
    let rebuild = cli.rebuild;

    // Run docker build --build-arg RUBY_VERSION=$RUBY_VERSION --build-arg RAILS_VERSION=$RAILS_VERSION -t rails-new-$RUBY_VERSION-$RAILS_VERSION
    // passing the content of DOCKERFILE to the command stdin
    let child = DockerClient::build_image(
        &ruby_version,
        rails_version,
        os_specific::get_user_id(),
        os_specific::get_group_id(),
        rebuild,
    )
    .spawn();

    let mut child_ref = match child {
        Ok(child) => child,
        Err(error) => {
            if error.kind() == ErrorKind::NotFound {
                println!("Docker is not installed");
            } else {
                println!("Failed to execute process: {}", error);
            }
            std::process::exit(1);
        }
    };

    let mut stdin = child_ref.stdin.take().expect("Failed to open stdin");
    std::thread::spawn(move || {
        stdin.write_all(os_specific::dockerfile_content()).unwrap();
    });

    let status = child_ref.wait().expect("failed to wait on child");

    assert!(status.success());

    let mut command: Command;

    match &cli.command {
        Some(Commands::RailsHelp {}) => {
            command = DockerClient::get_help(&ruby_version, rails_version)
        }

        None => {
            // Run the image with docker run -v $(pwd):/$(pwd) -w $(pwd) rails-new-$RUBY_VERSION-$RAILS_VERSION rails new $@
            command = DockerClient::run_image(&ruby_version, rails_version, cli.args)
        }
    }

    let status = command.status().expect("Failed to execute process");

    assert!(status.success());
}
