<div align="center">
    <h1><strong>Lemmynator</strong></h1>
    <p>
	    <strong>TUI for Lemmy</strong>
    </p>
    <img src="https://codeberg.org/micielski/lemmynator/raw/branch/main/imgs/image.png" />
</div>

## Features

- **Scrolling**

## Installation

To install Lemmynator, ensure you have Rust and Cargo installed on your system, and then run:

```bash
cargo install lemmynator
```

## Usage

Launch Lemmynator in your terminal to initialize the configuration and make adjustments as needed. Subsequently, run Lemmynator again. For list of keybindings, press '?'.

## Configuration

Lemmynator stores its configuration in a TOML file located at ~/.config/lemmynator/config.toml by default. You can modify this file to
set the daemon's IP address.

```toml
[connection]
instance = "slrpnk.net"
username = "YOUR_USERNAME"
password = "YOUR_PASSWORD"

[general]
accent_color = "LightGreen"
```

## Contributing

Contributions are welcome! If you'd like to contribute to Lemmynator, please fork the repository, make your changes, and submit a pull request!
