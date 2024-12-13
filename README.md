<img align="left" width="100" height="100" src="https://github.com/user-attachments/assets/57aa5eaa-3d7a-4038-9ece-c3ebd9ca46d2">

**Lemmynator**

Performant TUI for Lemmy that looks cool and one that might become your daily-driver one day.  
Currently it's still in early development, but it's getting there.

# 
<div align="center">
    <img src="https://github.com/user-attachments/assets/52301f5c-0cbb-40a0-9ef7-e457e684bf76"/>
    <p>
        <small>Posts shown here were chosen randomly</small>
    </p>
</div>

## Features

- **Scrolling**: Whether you want to see local posts or all, it's all here

## Requirements

- [Account on Lemmy](https://join-lemmy.org/)
- [Nerd Fonts](https://www.nerdfonts.com/)


## Installation

To install Lemmynator, ensure you have Rust and Cargo installed on your system, and then run:

```bash
cargo install lemmynator
```

## Usage

Launch Lemmynator in your terminal to initialize the configuration and make adjustments as needed. Subsequently, run Lemmynator again. For list of keybindings, press '?'.

## Configuration

Lemmynator stores its configuration in a TOML file located at ~/.config/lemmynator/config.toml by default.

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
