# devicectrl-input

Client for sending update requests generated from input events from physical devices like keyboards.

## Configuration

Example:

```json
{
    "server_connection": {
        "server_addr": "10.0.1.1:8894",
        "server_domain": "something.local",
        "server_ca_path": "/usr/local/share/ca-certificates/ca.crt",
        "cert_path": "/etc/ssl/certs/local.pem",
        "key_path": "/etc/ssl/private/local.key"
    },
    "response_timeout": { "secs": 1, "nanos": 0 },
    "actions": [
        [
            {
                "key": "BTN_LEFT",
                "value": 1,
                "device_names": ["Kensington      Kensington USB/PS2 Orbit"]
            },
            [
                {
                    "device_id": "lights",
                    "change_to": { "LedStrip": { "brightness": 100 } }
                }
            ]
        ],
        [
            {
                "key": "KEY_LEFT",
                "value": 1,
                "device_names": ["Kensington      Kensington USB/PS2 Orbit"]
            },
            [
                {
                    "device_id": "switch",
                    "change_to": { "Switch": { "power": true } }
                },
                {
                    "device_id": "small-palmtree",
                    "change_to": { "LedStrip": { "brightness": 1 } }
                }
            ]
        ]
    ]
}
```

## Running

Simply execute:

`CONFIG_PATH=config.toml cargo run`

Or use the provided systemd service:

```bash
cargo build --release
sudo install -m 755 ./target/release/devicectrl-input /usr/local/bin/
sudo install -m 644 devicectrl-input.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now devicectrl-input
```
