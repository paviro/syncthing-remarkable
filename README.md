# Syncthing for reMarkable

A [rm-appload](https://github.com/asivery/rm-appload) Syncthing application for reMarkable tablets. It will automatically install Syncthing and create a systemd service for you and allows you to monitor your syncing states. 

## Features

- 📊 **Real-time Monitoring** - View Syncthing service status and sync progress
- 🎛️ **Service Control** - Start, stop, and restart Syncthing service with a single tap
- 🚀 **Auto-Installer** - Automatically downloads and installs the latest Syncthing release

## Screenshots
<p align="center">
  <img src="https://github.com/user-attachments/assets/4e2262ab-be68-46a7-80d8-2d2a2a60940b" alt="installer" width="250"/>
  <img src="https://github.com/user-attachments/assets/91a2589b-7f66-4a47-bbff-5ac29e0c539d" alt="app" width="250"/>
  <img src="https://github.com/user-attachments/assets/848d74cf-8c02-429a-b2bb-a8a738747cb9" alt="app" width="250"/>
</p>

## Tested Devices

- reMarkable Paper Pro Move


## How to Install

### Pre-installation Steps

1. **Install XOVI**
   
   Install XOVI from [https://github.com/asivery/rm-xovi-extensions](https://github.com/asivery/rm-xovi-extensions) by using the included installation script.

2. **Install Required Extensions**
   
   Install `qt-resource-rebuilder` (from the XOVI repo) and `rm-appload`:
   
   ```bash
   # Copy the required extensions to the XOVI extensions directory
   cp qt-resource-rebuilder.so /home/root/xovi/extensions.d/
   cp appload.so /home/root/xovi/extensions.d/
   ```

3. **Rebuild Hash Table**
   
   ```bash
   xovi/rebuild_hashtable
   ```

4. **Start XOVI**
   
   Run XOVI (you must do this every time you reboot your device):
   
   ```bash
   xovi/start
   ```

### Installation Steps

1. **Download Syncthing for reMarkable**
   
   Download the [latest release](https://github.com/paviro/Syncthing-for-reMarkable/releases) of Syncthing for reMarkable and pick the archive that matches your device:
   - **reMarkable Paper Pro / Paper Pro Move** → `syncthing-rm-appload-aarch64.zip`
   - **reMarkable 2** → `syncthing-rm-appload-armv7.zip` (32-bit build, currently untested)

2. **Extract and Copy Files**
   
   Extract the archive and copy the `syncthing` folder to `/home/root/xovi/exthome/appload/` so that it remains as:
   
   ```
   /home/root/xovi/exthome/appload/syncthing
   ```
   
   > **Note:** Most users can ignore the `config.sample.json` file when using the auto-installer - it's not needed! The auto-installer handles everything automatically. This configuration file is only for advanced users who want to manually manage their Syncthing installation.

3. **Launch Syncthing**
   
   - Open the sidebar on your reMarkable
   - Touch "AppLoad"
   - Launch Syncthing from the AppLoad menu

4. **Closing the App**
   
   To close the app, swipe down from the center top of the screen to display the AppLoad window controls and tap the X button.

## Accessing the Syncthing Web Interface

### Via USB Connection (Default)

When your reMarkable device is connected via USB, you can access the Syncthing web interface at:

```
http://10.11.99.1:8384
```

Simply open this URL in your web browser while your device is connected.

### Via Network (Optional)

To access Syncthing over your local network:

1. Open the Syncthing app on your reMarkable
2. Tap the **gear icon** (⚙️) at the top right to open Settings
3. Enable **Network Access**
4. Access the web interface using your device's IP address:
   ```
   http://<device-ip>:8384
   ```
   
   Replace `<device-ip>` with your reMarkable's IP address on your local network.

> **⚠️ Security Note:** When enabling network access, it's strongly recommended to:
> - **Set a password** in the Syncthing web interface (Settings → GUI → GUI Authentication)
> - **Enable HTTPS** in the Syncthing web interface (Settings → GUI → Use HTTPS for GUI)
> 
> This ensures your Syncthing instance is protected when accessible over the network.
