# Set up your Tessera

You need a JC3248W535 panel, a USB **data** cable, a Mac for the companion, and Wi‑Fi for weather and device configuration. The current release is intended for contributors comfortable flashing an ESP32. There is no one-click firmware installer yet.

## 1. Prepare the device

If your panel already runs Tessera, skip flashing. Otherwise follow [developer setup](../getting-started.md), connect the board and run `make finish-host`, then `make flash`. Do not use firmware for a different ESP32 board.

On first boot the panel shows its Wi‑Fi setup instructions. Join the network shown on the display using your phone, open the captive portal and provide your home Wi‑Fi details. Once connected, open `http://tessera.local` from the same network. If the name does not resolve, use the IP shown in the device's network settings.

## 2. Install the Mac Agent

Open [the Tessera website](https://gilsondev.github.io/Tessera/), download the installer, and follow its instructions. The installer installs **both** the background daemon and Tessera.app. Moving only Tessera.app from the ZIP does not install the background service.

The current CI release targets Apple Silicon on macOS 13 or newer. Intel Macs can build from source; a tested Intel release is not published by the existing workflow. Builds are not notarized; macOS may require **System Settings → Privacy & Security → Open Anyway**.

## 3. Choose what to show

Open Tessera, choose **Start on this Mac**, select integrations, and grant only the permissions they need. Connect the USB data cable; the last setup step confirms the actual connection. You can also finish setup and connect later.

Choose your Mac calendars from **Integrations → Configure Agenda**. See [Calendar setup](calendar-setup.md). Music and Focus update from the Mac; their widgets do not control the Mac.

## 4. Make the panel yours

Use the panel for quick changes such as brightness and screen rest. Use the Agent's **Panel / Widgets** tabs or the local web portal for longer configuration. These device settings require the panel on the same Wi‑Fi; USB carries Mac integrations. See [where settings belong](settings-surfaces.md).

If the USB is disconnected, the clock and weather keep working. Mac content becomes stale until reconnection. For connection and permission problems, open the Agent's diagnostics.
