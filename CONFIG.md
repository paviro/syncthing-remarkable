# Configuration

**Note: This configuration is only needed if you already have Syncthing installed and want to skip the auto-installer.** If you're using the app's built-in installer, you don't need to create a config file.

The default config of the Syncthing Monitor can be overwritten via a `config.json` file. 

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
  "syncthing_config_dir": "/home/root/.config/syncthing",
  "disable_syncthing_installer": true
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

### `disable_syncthing_installer`
- **Type**: Boolean
- **Default**: `false`
- **Description**: Set to `true` to disable the built-in Syncthing installer. Use this if you already have Syncthing installed and configured on your system.
