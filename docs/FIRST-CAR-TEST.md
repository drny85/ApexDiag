# First real car test

**Car:** 2008 BMW X5 (E70). **Adapter:** Veepeak OBDCheck BLE. **Device:** iPad with ApexDiag installed (see [IPAD.md](IPAD.md)).

About 10 minutes. **Everything in this test only reads.** Nothing on the car is cleared, coded or changed.

## Before you go to the car

- [ ] The iPad is charged and ApexDiag opens.
- [ ] Close any other OBD app (the Veepeak app, Car Scanner, OBD Fusion). A BLE adapter talks to one app at a time.
- [ ] Don't pair the Veepeak in the iPad's Bluetooth settings. If it is already paired there, tap it and choose **Forget This Device**.

## At the car

1. **Plug in the adapter.** The OBD port is under the dashboard on the driver's side, left of the steering column, near the bonnet release. It may sit behind a small flap. Push the adapter fully home; its light should come on.
2. **Ignition on, engine off.** Sit in the car with the door closed. Press Start once *without* your foot on the brake. The car must be in Park with the parking brake on.
3. **Connect.** In ApexDiag open **Connect**, then tap **Scan Bluetooth**. Allow Bluetooth if the iPad asks. The adapter usually appears as `VEEPEAK`, `OBDII` or `OBDBLE`. Tap **Connect** and wait for the VIN.
   - Expected: a VIN starting with `5UX` or `WBA`, and the vehicle pack **BMW X5 (E70)**.
4. **Run the test.** Open **Car Test**, then tap **Start the test**. Eight steps run in about one to two minutes. Leave the iPad still and close to the adapter while they run.
5. **Run it again with the engine running.** Start the engine (outdoors or in a ventilated space), then tap **Run the test again**. Now the live data shows real engine speed and temperature.
6. **Save.** Tap **Save report and capture**. Two files are written: `car-test-<VIN>-<number>.json` (the results) and `.log` (every byte the adapter said).
7. **Send both files back.** Open the **Files** app, then **On My iPad**, then **ApexDiag**. Long-press each file, then Share. Attach them in the ApexDiag project chat, or AirDrop them to the Mac.

That's the whole test. If you want to look around afterwards, **Quick Test**, **Fault Codes** and **Live Data** are safe. Leave **Clear codes** and **Special Functions** alone on this first visit; we turn them on for this car once the capture confirms its modules.

## What the results mean

| Step | What a tick proves |
|---|---|
| Adapter and battery | The Bluetooth link works both ways and the car powers the adapter. |
| Vehicle identity | The adapter reaches the car's diagnostic network. |
| Engine fault codes | Generic OBD-II codes read correctly. |
| Emissions readiness | The emissions self-tests can be read. |
| Live data | Gauges will update. |
| Freeze frame | The engine's snapshot from its first code can be read (it may say "none stored"). |
| Engine module, direct | ApexDiag reaches the BMW DME the way the dealer tool does (extended addressing). |
| All modules | Which of the 13 known E70 modules answer through this adapter. |

## If something goes wrong

| What you see | Try this |
|---|---|
| Scan finds nothing | Check the adapter's light is on and the ignition is on. Turn the iPad's Bluetooth off and on. Close other OBD apps. Tick **Show all Bluetooth devices** and scan again. |
| Connects, then "the adapter stopped answering" | Move the iPad closer, tap **Disconnect**, connect again, rerun the test. |
| No VIN | Check the ignition is on (not just accessory). The rest of the test still runs. |
| "Description: not supported by this adapter" | Normal for many adapters. Nothing to do. |
| **Engine module, direct** fails | The adapter may not pass BMW extended addressing through. Still save and send the files: the capture shows exactly where it stopped. Engine OBD-II still works. |
| Only a few modules answer | Expected on a first run. Send the files: that's what confirms this car's module map. |
| The app closes when you scan | Delete and reinstall it, then allow Bluetooth when asked. Send a note of what you tapped. |

## Battery

With the engine off, keep the ignition-on time under about 20 minutes, or start the engine. The E70 draws a lot with the ignition on.
