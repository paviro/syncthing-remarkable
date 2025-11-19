# Syncthing Monitor

A [rm-appload](https://github.com/asivery/rm-appload) Syncthing application for reMarkable tablets. It will automatically install Syncthing and create a systemd service for you and allows you to monitor your syncing states. 

## Features

- 📊 **Real-time Monitoring** - View Syncthing service status and sync progress
- 🎛️ **Service Control** - Start, stop, and restart Syncthing service with a single tap
- 🚀 **Auto-Installer** - Automatically downloads and installs the latest Syncthing release

## Tested Devices

- reMarkable Paper Pro Move

## Screenshots

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
   
   Download the latest release of Syncthing for reMarkable.

2. **Extract and Copy Files**
   
   Extract the archive and copy the `syncthing` folder to `/home/root/xovi/exthome/appload/` so that it remains as:
   
   ```
   /home/root/xovi/exthome/appload/syncthing
   ```

3. **Launch Syncthing**
   
   - Open the sidebar on your reMarkable
   - Touch "AppLoad"
   - Launch Syncthing from the AppLoad menu

4. **Closing the App**
   
   To close the app, swipe down from the center top of the screen to display the AppLoad window controls and tap the X button.

