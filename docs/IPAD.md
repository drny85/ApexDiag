# ApexDiag on iPad

The iPad app is the same Tauri app built for iOS. The Rust core runs on the
iPad; Bluetooth LE goes through CoreBluetooth (via `btleplug`), so a BLE-only
adapter such as the Veepeak OBDCheck BLE works without pairing.

## Install on your iPad (about 5 minutes the first time)

1. Plug the iPad into the Mac with a cable and unlock it. Tap **Trust** and
   enter the iPad passcode.
2. On the iPad: **Settings > Privacy & Security > Developer Mode**, turn it on.
   The iPad restarts; after it restarts, unlock it and tap **Turn On**.
   (If Developer Mode is not listed, do step 3 once and it appears.)
3. On the Mac, in the repo:

   ```bash
   ./scripts/ios-install.sh
   ```

   It finds the iPad, builds a signed release build with your Apple
   Developer team (JEBPX7GX5C), installs it and opens it.
4. If the iPad says "Untrusted Developer": **Settings > General > VPN &
   Device Management**, tap your developer certificate, tap **Trust**.

After the first time the cable is optional: in Xcode, **Window > Devices and
Simulators**, select the iPad, tick **Connect via network**.

You can also open `src-tauri/gen/apple/apexdiag.xcodeproj` in Xcode, pick the
iPad as the run destination and press Run. Xcode builds the Rust code itself
(`build-rust.sh`); the Tauri CLI is not needed.

## Connect to the car

1. Plug the Veepeak into the OBD port (BMW E70: under the dashboard, driver's
   side, left of the steering column). Ignition on (engine off is fine).
2. **Do not pair the adapter in iPad Settings.** BLE adapters connect inside
   the app.
3. Open ApexDiag > Connect > **Scan Bluetooth**. Allow Bluetooth when iPadOS
   asks. The adapter shows up (often as `VEEPEAK` or `OBDII`); tap Connect.
4. If nothing shows, tick **Show all Bluetooth devices** and scan again.
5. Open **Car Test** and run the 8 read-only steps. Nothing is written to
   the car.
6. Tap **Save**. The report (`.json`) and the raw adapter capture (`.log`)
   go to the **Files** app > **On My iPad** > **ApexDiag**. Share them from
   there (AirDrop to the Mac, or attach them to a message).

## Development

- Simulator build: `cd src-tauri/gen/apple && xcodebuild -scheme apexdiag_iOS -sdk iphonesimulator -destination 'generic/platform=iOS Simulator' build`
  (the simulator has no Bluetooth; use the practice cars).
- Live reload on a device: `bun tauri ios dev`.
- The Xcode project is generated from `src-tauri/gen/apple/project.yml` with
  `xcodegen generate`. Edit `project.yml`, not the `.pbxproj`; do not re-run
  `tauri ios init`, which overwrites the Bluetooth permission string and the
  CoreBluetooth link.
