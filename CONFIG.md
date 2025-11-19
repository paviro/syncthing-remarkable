# Configuration

The default config of the Syncthing Monitor can be overwritten via a `config.json` file. 
If you installed Syncthing with the app itself or use the default paths you can ignore this. 

## Location

The backend looks for the `config.json` file in the app directory.

A sample configuration file `config.sample.json` is provided that you can copy and modify:

```bash
cp config.sample.json config.json
# Edit config.json with your custom settings
```

## Configuration Options

```json
{
  "systemd_service_name": "syncthing.service",
  "syncthing_config_dir": "/home/root/.config/syncthing"
}
```

### `systemd_service_name`
- **Type**: String
- **Default**: `"syncthing.service"`
- **Description**: The name of the systemd service to monitor and control.

### `syncthing_config_dir`
- **Type**: String  
- **Default**: `"/home/root/.config/syncthing"`
- **Description**: The directory containing the Syncthing configuration files. The backend will look for `config.xml` in this directory to read the API key.

## Fallback Behavior

If the `config.json` file is not found or cannot be parsed, the backend will use the default values shown above. This ensures backward compatibility with existing installations.

## Example Usage

### Default Configuration
If no `config.json` file exists, the backend will use:
- Service name: `syncthing.service`
- Config directory: `/home/root/.config/syncthing`

### Custom Configuration
To use a custom systemd service or config directory, create a `config.json` file in the app directory:

```json
{
  "systemd_service_name": "syncthing@myuser.service",
  "syncthing_config_dir": "/home/myuser/.config/syncthing"
}
```

## Environment Variables

The following environment variables are still supported and take precedence over the config file:

- `SYNCTHING_API_KEY`: If set, this API key will be used instead of reading from `config.xml`
- `SYNCTHING_API_URL`: Custom Syncthing API URL (e.g., for non-standard ports)

## Notes

- The backend will log configuration loading status to stderr
- Invalid JSON in `config.json` will be logged and defaults will be used
- Both fields are optional in the JSON file - any missing fields will use their default values

