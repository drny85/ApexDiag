/**
 * Illustrated how-to guides for Repair Copilot: bulb and lamp replacement,
 * and the hands-on fixes the fault playbooks recommend.
 *
 * These are general workshop practice, not a maker's service procedure.
 * Every guide says so on screen. Anything car-specific lives in `perCar`
 * and is only filled in where we are sure of it; when it isn't, the guide
 * tells the reader where to check (owner's manual, the old part, the
 * maker's service information) instead of guessing.
 */
import type { PartId } from "@/components/car3d";

export type GuideArt =
  | "lamp_types"
  | "bulb_codes"
  | "power_off"
  | "bonnet_open"
  | "access_cover"
  | "wheel_liner"
  | "connector"
  | "spring_clip"
  | "twist_socket"
  | "no_touch"
  | "wedge_bulb"
  | "festoon_bulb"
  | "plate_lens"
  | "boot_trim"
  | "helper_check"
  | "beam_aim"
  | "xenon_hv"
  | "led_unit"
  | "maf_unplug"
  | "maf_spray"
  | "smoke_test"
  | "coil_swap"
  | "plug_gap"
  | "plug_torque"
  | "battery_neg"
  | "battery_label"
  | "terminal_brush"
  | "battery_lift"
  | "battery_register"
  | "fuel_cap"
  | "clear_codes";

export interface GuideStep {
  title: string;
  text: string;
  art: GuideArt;
  caution?: string;
  /** Turn the 3D car's lights on for this step. */
  lights?: boolean;
}

export type GuideCategory = "lights" | "engine" | "electrical" | "fuel";

export interface Guide {
  id: string;
  title: string;
  summary: string;
  category: GuideCategory;
  /** Where it is on the car, shown on the 3D model. */
  part: PartId;
  partLabel: string;
  difficulty: "diy" | "experienced" | "shop";
  time: string;
  tools: string[];
  parts: string[];
  safety: string[];
  steps: GuideStep[];
  /** How to know it worked. */
  verify: string;
  /** Repair Copilot causes (by name) that this guide carries out. */
  causes?: string[];
  /** Notes for one vehicle pack, only where we are certain. */
  perCar?: Partial<Record<string, string>>;
}

export const CATEGORY_LABEL: Record<GuideCategory, string> = {
  lights: "Lights and bulbs",
  engine: "Engine",
  electrical: "Battery and charging",
  fuel: "Fuel system",
};

const IGNITION_OFF: GuideStep = {
  title: "Switch everything off",
  text: "Turn the lights switch to off, switch the ignition off and take the key or fob out of the car. Let a lamp that was on cool for a few minutes.",
  art: "power_off",
};

const HALOGEN_NO_TOUCH = "Hold a halogen bulb by its metal or plastic base only. Skin oil on the glass makes a hot spot that cracks or dims it early. If you touch it, wipe the glass with isopropyl alcohol.";

export const GUIDES: Guide[] = [
  {
    id: "lamp-type",
    title: "Which kind of light does your car have?",
    summary: "Halogen, xenon (HID) or LED. It decides whether there is a bulb to change, what it costs, and whether it is safe to do yourself.",
    category: "lights",
    part: "headlights",
    partLabel: "Headlight",
    difficulty: "diy",
    time: "5 min",
    tools: ["Torch", "Owner's manual"],
    parts: [],
    safety: [],
    steps: [
      {
        title: "Look at the light when it is on",
        text: "Halogen light is warm and slightly yellow. Xenon (HID) is bright white with a blue edge and takes a second or two to reach full brightness. LED is instant, crisp white, and often shaped into strips or rows of small dots.",
        art: "lamp_types",
        lights: true,
      },
      {
        title: "Read the code on the old bulb",
        text: "Every replaceable bulb has its type printed on the base or the glass: H7, H11, 9005, D1S, D3S, W5W, P21W, 7443 and so on. That code is what you buy. The owner's manual lists every bulb too.",
        art: "bulb_codes",
      },
      {
        title: "No bulb? It is probably an LED unit",
        text: "Many lamps fitted from about 2015 on, and most from 2018 on, are sealed LED units with no bulb to change. When one fails the LED module or the whole lamp is replaced. Some cars must then be set up with a scan tool.",
        art: "led_unit",
      },
    ],
    verify: "You know the type and the exact bulb code before buying anything.",
  },
  {
    id: "headlight-halogen",
    title: "Replace a halogen headlight bulb",
    summary: "The most common bulb job. On most cars it is done from under the bonnet, behind the headlight, without removing the lamp.",
    category: "lights",
    part: "headlights",
    partLabel: "Headlight",
    difficulty: "diy",
    time: "10-40 min per side",
    tools: ["Gloves", "Torch", "Trim tool or flat screwdriver (some cars)"],
    parts: ["Bulb of the type printed on the old one (H7, H11, 9005...)"],
    safety: [HALOGEN_NO_TOUCH, "Replace bulbs in pairs: the other side is the same age and usually fails soon after."],
    steps: [
      IGNITION_OFF,
      {
        title: "Open the bonnet and find the back of the headlight",
        text: "Look behind the headlight for a round or square dust cover. On some cars the battery, air box or washer filler is in the way and has to be moved aside. On others the bulb is reached through a flap in the wheel-arch liner instead.",
        art: "bonnet_open",
      },
      {
        title: "Take off the dust cover",
        text: "Round covers twist anticlockwise about a quarter turn. Square ones have a clip or tab you lift. Keep the seal clean: a dust cover that does not seat lets water into the lamp.",
        art: "access_cover",
      },
      {
        title: "Unplug the connector",
        text: "Press the release tab and pull the connector straight off the bulb. Pull on the plug body, never the wires.",
        art: "connector",
      },
      {
        title: "Release the bulb",
        text: "Most bulbs are held by a wire spring clip: push it in and sideways to unhook it, then swing it out of the way. Others twist anticlockwise to unlock. Lift the bulb straight out and note which way its flange sits.",
        art: "spring_clip",
      },
      {
        title: "Fit the new bulb without touching the glass",
        text: "Line up the flange tabs with the slots so it seats flat, refit the clip or twist it home, push the connector on until it clicks, and refit the dust cover.",
        art: "no_touch",
        caution: HALOGEN_NO_TOUCH,
      },
      {
        title: "Test low and high beam, then check the aim",
        text: "Switch on and check both beams. If the beam pattern on a wall looks wrong, the bulb is not seated flat. Take it out and refit it.",
        art: "beam_aim",
        lights: true,
      },
    ],
    verify: "Both beams light, the bulb-out warning on the dash (if fitted) has cleared, and the beam cut-off is level and matches the other side.",
  },
  {
    id: "headlight-xenon",
    title: "Xenon (HID) headlight bulbs",
    summary: "Xenon bulbs run at very high voltage. Changing one is possible at home on some cars, but read the safety notes first, and on many cars it means removing the lamp.",
    category: "lights",
    part: "headlights",
    partLabel: "Headlight",
    difficulty: "experienced",
    time: "30 min to 2 h per side",
    tools: ["Gloves and eye protection", "Trim tools", "Socket set (if the lamp comes out)"],
    parts: ["Xenon bulb of the same D-type (D1S, D2S, D3S...) and colour temperature"],
    safety: [
      "The igniter makes tens of thousands of volts at switch-on. Only work with the lights switched off, the key out and, ideally, the battery disconnected.",
      "Xenon bulbs contain mercury on some types. Do not break the old one; recycle it.",
      "Change both sides together so the colour matches.",
    ],
    steps: [
      {
        title: "Make the system dead",
        text: "Lights off, ignition off, key out of the car. Disconnecting the battery's negative terminal is the safest way to be sure the lamp cannot be switched on while your hands are inside it.",
        art: "battery_neg",
        caution: "Very high voltage at the bulb and igniter when the light is switched on.",
      },
      {
        title: "Know before you start whether the lamp has to come out",
        text: "On some cars the xenon bulb is behind a cover at the back of the lamp, like a halogen one. On many it is only reachable with the bumper or headlight removed. The maker's service information says which. If the lamp has to come out, this is a shop job for most owners.",
        art: "xenon_hv",
      },
      {
        title: "Open the cover and unlock the igniter",
        text: "Remove the rear cover. The igniter sits on the back of the bulb; turn it (usually anticlockwise) to unlock it from the bulb, then release the bulb's retaining ring or clip.",
        art: "twist_socket",
      },
      {
        title: "Fit the new bulb by its base only",
        text: "Hold it by the base, line up the tabs, lock the retaining ring, refit the igniter and the cover.",
        art: "no_touch",
      },
      {
        title: "Reconnect and test",
        text: "Reconnect the battery, switch the lights on and give the new bulb about a minute to reach its full colour. If the car has automatic levelling, the beam may sweep once at start-up.",
        art: "beam_aim",
        lights: true,
      },
    ],
    verify: "Both lamps light the same colour, no lamp warning shows, and the beam cut-off is level.",
    perCar: {
      bmw_e70: "Many E70s were built with xenon low beams. Check the old bulb's code before buying; and if the car has adaptive (swivelling) headlights, leave the lamp unit's motors alone.",
    },
  },
  {
    id: "led-lamps",
    title: "LED headlights and tail lights",
    summary: "Sealed LED lamps have no bulb to change. Here is how to tell a failed LED from a wiring fault before paying for a lamp.",
    category: "lights",
    part: "headlights",
    partLabel: "LED lamp",
    difficulty: "shop",
    time: "Varies",
    tools: ["Scan tool (this app)", "Multimeter"],
    parts: ["LED module, driver, or complete lamp (as the maker supplies it)"],
    safety: ["LED headlamps have driver electronics that can hold charge. Do not open a sealed unit."],
    steps: [
      {
        title: "Read the body or lighting module for faults",
        text: "A failed LED or driver usually logs a fault in the body or lighting module naming which lamp and which function. Run a full scan and open the fault before buying anything.",
        art: "clear_codes",
      },
      {
        title: "Check the connector and the fuse",
        text: "Unplug the lamp with the lights off, look for green corrosion or a pushed-back pin, and check the lamp's fuse. A loose connector costs nothing; a lamp can cost hundreds.",
        art: "connector",
      },
      {
        title: "Replace the module or lamp",
        text: "Some LED lamps have a replaceable module or driver on the back; others are one sealed part. Fit exactly the part the maker lists for your car.",
        art: "led_unit",
      },
      {
        title: "Set it up if the car needs it",
        text: "Some cars need a new lamp coded or calibrated with a scan tool before it works fully (levelling, adaptive beam). The maker's service information says if this applies.",
        art: "battery_register",
      },
    ],
    verify: "The lamp lights in every function (running light, low, high, indicator) and the fault does not come back after clearing.",
  },
  {
    id: "tail-brake",
    title: "Replace a tail or brake light bulb",
    summary: "Reached from inside the boot or tailgate on most cars, through a trim flap or by removing the lamp with two or three nuts.",
    category: "lights",
    part: "taillights",
    partLabel: "Tail light",
    difficulty: "diy",
    time: "10-30 min",
    tools: ["Trim tool", "Socket set (if the lamp unbolts)", "A helper or a reflective wall"],
    parts: ["Bulb of the type printed on the old one (P21W, P21/5W, W16W, 7443...)"],
    safety: ["Use the exact bulb type: a single-filament bulb in a twin-filament socket can leave you with no brake light."],
    steps: [
      IGNITION_OFF,
      {
        title: "Open the trim behind the lamp",
        text: "Open the boot or tailgate. Behind the lamp there is usually a small flap or carpet section that pulls back. On some cars the lamp is held by two or three plastic nuts and comes out whole.",
        art: "boot_trim",
      },
      {
        title: "Remove the bulb holder",
        text: "Squeeze the tabs or twist the holder anticlockwise a quarter turn and pull it out of the lamp.",
        art: "twist_socket",
      },
      {
        title: "Swap the bulb",
        text: "Wedge bulbs pull straight out. Bayonet bulbs push in and twist anticlockwise. Fit the new one the same way; a twin-filament bayonet bulb has offset pins and only fits one way.",
        art: "wedge_bulb",
      },
      {
        title: "Check every function with a helper",
        text: "Refit the holder. Have someone press the brake, then check tail lights, indicators and reverse. On your own, back up to a garage door or shop window and watch the reflection.",
        art: "helper_check",
        lights: true,
      },
    ],
    verify: "Tail, brake, indicator and reverse all light on both sides, and the bulb-out warning (if fitted) has cleared.",
  },
  {
    id: "third-brake",
    title: "High-mounted brake light out",
    summary: "The third brake light is LED on most modern cars and is replaced as one unit. Check the connector first.",
    category: "lights",
    part: "third_brake",
    partLabel: "Third brake light",
    difficulty: "experienced",
    time: "20-60 min",
    tools: ["Trim tools", "Small socket set"],
    parts: ["Third brake light unit (or bulbs on older cars)"],
    safety: ["On cars where it sits in the spoiler, the washer hose for the rear wiper often runs through it. Do not pull it off."],
    steps: [
      {
        title: "Confirm it is the lamp",
        text: "Press the brake with a helper. If the main brake lights work but this one does not, the fault is in the lamp, its connector or the wire to it.",
        art: "helper_check",
        lights: true,
      },
      {
        title: "Remove the trim or cover",
        text: "It is reached either from inside (headliner or tailgate trim) or from outside under the spoiler. Use plastic trim tools to release clips without breaking them.",
        art: "boot_trim",
      },
      {
        title: "Check the connector, then replace the unit",
        text: "Unplug it and look for water or corrosion. If the connector is fine, fit the new unit and reconnect.",
        art: "connector",
      },
    ],
    verify: "The third brake light comes on with the pedal and the bulb-out warning (if fitted) has cleared.",
  },
  {
    id: "plate-light",
    title: "Replace a number plate light",
    summary: "Two small screws or a clip, and a festoon or wedge bulb. One of the easiest jobs on the car.",
    category: "lights",
    part: "plate_light",
    partLabel: "Plate light",
    difficulty: "diy",
    time: "5-15 min",
    tools: ["Small screwdriver or trim tool"],
    parts: ["Bulb (C5W festoon or W5W wedge), or an LED lens unit"],
    safety: [],
    steps: [
      IGNITION_OFF,
      {
        title: "Release the lens",
        text: "Look above the plate. Undo the two small screws, or push the lens towards its spring clip and lever it out gently.",
        art: "plate_lens",
      },
      {
        title: "Swap the bulb",
        text: "A festoon bulb (a tube with a cap at each end) clips between two springs: lever one end out. A wedge bulb pulls straight out. LED units are replaced as a whole.",
        art: "festoon_bulb",
      },
      {
        title: "Test and refit",
        text: "Switch the side lights on to test before you refit the lens.",
        art: "helper_check",
        lights: true,
      },
    ],
    verify: "Both plate lights work: an unlit plate is an easy reason to be stopped.",
  },
  {
    id: "front-indicator",
    title: "Front indicator or side-marker bulb",
    summary: "Usually in the headlight unit, reached from behind it, or in the bumper corner through the wheel-arch liner.",
    category: "lights",
    part: "headlights",
    partLabel: "Front indicator",
    difficulty: "diy",
    time: "10-40 min",
    tools: ["Torch", "Trim tool", "Screwdriver or Torx set for the liner"],
    parts: ["Amber bulb of the type printed on the old one (PY21W, WY5W, 7444NA...)"],
    safety: ["Use amber bulbs where the lens is clear: a white bulb behind a clear lens is not legal for an indicator."],
    steps: [
      IGNITION_OFF,
      {
        title: "Turn the wheel for room",
        text: "If the bulb is in the bumper corner, turn the steering wheel fully the other way and fold back the front of the wheel-arch liner (a few clips or screws).",
        art: "wheel_liner",
      },
      {
        title: "Twist out the holder and swap the bulb",
        text: "Twist the holder anticlockwise, pull it out, and change the bulb as you would a tail light bulb.",
        art: "twist_socket",
      },
      {
        title: "Check the flash rate",
        text: "Switch the indicator on. A fast flash means a bulb is still out or the wrong wattage is fitted.",
        art: "helper_check",
        lights: true,
      },
    ],
    verify: "The indicator flashes at its normal rate on both sides.",
  },
  {
    id: "maf-clean",
    title: "Clean the air-flow sensor",
    summary: "A dirty air-flow (MAF) sensor under-reads, so the engine runs lean. Cleaning it with the right spray is cheap and often fixes lean trims.",
    category: "engine",
    part: "engine",
    partLabel: "Air-flow sensor",
    difficulty: "diy",
    time: "15-30 min",
    tools: ["Screwdriver or Torx set", "Air-flow sensor cleaner (MAF cleaner)"],
    parts: ["Air filter, if it is dirty"],
    safety: ["Only use spray made for air-flow sensors. Brake or carb cleaner destroys the element.", "Never touch the sensing wires or film."],
    steps: [
      IGNITION_OFF,
      {
        title: "Find the sensor and unplug it",
        text: "It sits in the intake duct between the air filter box and the throttle. Press the connector tab and unplug it.",
        art: "maf_unplug",
      },
      {
        title: "Remove the sensor",
        text: "Undo the two screws (or the duct clamps if the sensor is part of a tube) and lift it out without touching the element.",
        art: "maf_unplug",
      },
      {
        title: "Spray it clean and let it dry",
        text: "Give the element ten to fifteen short bursts from all sides. Do not wipe or blow on it. Let it air-dry completely, at least 10 minutes.",
        art: "maf_spray",
        caution: "Never touch the element. Let it dry fully before refitting.",
      },
      {
        title: "Refit, then let the app re-measure",
        text: "Refit the sensor, plug it in, start the engine and use Repair Copilot's fuel-trim check to see if trims came back towards zero.",
        art: "clear_codes",
      },
    ],
    verify: "Total fuel trim settles within about ±5% at idle and the lean code does not return.",
    causes: ["Dirty or failing air-flow sensor", "Dirty sensor element"],
  },
  {
    id: "smoke-test",
    title: "Find a vacuum leak with smoke",
    summary: "Smoke goes into the intake and comes out wherever air is sneaking in. It finds leaks you cannot hear.",
    category: "engine",
    part: "engine",
    partLabel: "Intake",
    difficulty: "experienced",
    time: "30-90 min",
    tools: ["Smoke machine", "Torch", "Duct plugs or caps"],
    parts: ["Whatever hose, gasket or valve the smoke points to"],
    safety: ["Engine off for the smoke test.", "Use a smoke machine made for cars, with no open flame near the fuel system."],
    steps: [
      IGNITION_OFF,
      {
        title: "Seal the intake and connect the smoke",
        text: "Disconnect the intake duct after the air-flow sensor, cap it, and feed the smoke in there (or through a vacuum port). Follow the machine's instructions for pressure.",
        art: "smoke_test",
      },
      {
        title: "Watch for smoke",
        text: "Shine a torch around the intake manifold, hoses, crankcase vent valve, oil filler cap and dipstick. A thin stream of smoke marks the leak.",
        art: "smoke_test",
      },
      {
        title: "Fix it and re-measure",
        text: "Replace the split hose, gasket or valve, refit everything, clear the codes and run the fuel-trim check again.",
        art: "clear_codes",
      },
    ],
    verify: "No smoke escapes on a re-test, and fuel trims at idle settle within about ±5%.",
    causes: ["Unmetered air (vacuum or intake leak)", "Vacuum leak", "Air leak after the sensor"],
  },
  {
    id: "coil-swap",
    title: "Find a bad ignition coil by swapping it",
    summary: "Move the coil from the misfiring cylinder to a neighbour. If the misfire moves with it, the coil is the problem.",
    category: "engine",
    part: "engine",
    partLabel: "Ignition coils",
    difficulty: "diy",
    time: "15-45 min",
    tools: ["Socket set or Torx set", "Trim tool for the engine cover"],
    parts: ["Ignition coil (only once proven bad)"],
    safety: ["Engine off. Coils carry very high voltage when the engine runs.", "Let the engine cool before working on it."],
    steps: [
      IGNITION_OFF,
      {
        title: "Remove the engine cover and find the coils",
        text: "Coils sit on top of the spark plugs, one per cylinder. The maker's service information shows how the cylinders are numbered.",
        art: "bonnet_open",
      },
      {
        title: "Swap the suspect coil with its neighbour",
        text: "Unplug both coils, pull them straight up, swap them, push them fully home and reconnect. Write down which went where.",
        art: "coil_swap",
      },
      {
        title: "Clear the codes and drive",
        text: "Clear the misfire codes and drive until the misfire shows again. If the code's cylinder number followed the coil, replace that coil. If it stayed, check the spark plug and injector on that cylinder.",
        art: "clear_codes",
      },
    ],
    verify: "After replacing the faulty coil, the misfire counter stays at zero and the code does not return.",
    causes: ["Ignition coil"],
  },
  {
    id: "spark-plugs",
    title: "Replace the spark plugs",
    summary: "Worn plugs misfire under load first. Replace them as a set, with the gap and torque the maker gives.",
    category: "engine",
    part: "engine",
    partLabel: "Spark plugs",
    difficulty: "diy",
    time: "30 min to 2 h",
    tools: ["Spark plug socket and extension", "Torque wrench", "Feeler or wire gap gauge", "Compressed air"],
    parts: ["Spark plugs of the exact type the maker lists"],
    safety: ["Engine cold: aluminium cylinder heads strip threads when hot.", "Torque to the maker's figure. Over-tightening cracks plugs or damages the head."],
    steps: [
      IGNITION_OFF,
      {
        title: "Remove the coils and blow out the wells",
        text: "Take out the coils, then blow dirt out of each plug well with compressed air so nothing falls into the cylinder.",
        art: "coil_swap",
      },
      {
        title: "Read each old plug",
        text: "Compare them as they come out. Oily, wet, white or eroded tips each point to a different problem; the misfiring cylinder's plug often looks different from the rest.",
        art: "plug_gap",
      },
      {
        title: "Check the gap and fit the new plugs",
        text: "Check the gap matches the maker's figure (many plugs come pre-gapped: do not bend iridium tips). Start each plug by hand to avoid cross-threading.",
        art: "plug_gap",
      },
      {
        title: "Torque, refit coils, test",
        text: "Tighten to the maker's torque, refit the coils and start the engine. Let the app watch the idle.",
        art: "plug_torque",
      },
    ],
    verify: "The engine idles steadily, and misfire counters stay at zero on a drive.",
    causes: ["Spark plug"],
  },
  {
    id: "battery-terminals",
    title: "Clean and tighten the battery terminals",
    summary: "A loose or corroded clamp looks like a weak battery to every module in the car. Free to fix.",
    category: "electrical",
    part: "battery",
    partLabel: "Battery",
    difficulty: "diy",
    time: "15-30 min",
    tools: ["Spanner or socket for the clamps", "Terminal brush or wire brush", "Gloves and eye protection"],
    parts: ["Terminal protector spray (optional)"],
    safety: ["Negative cable off first, on last. A spanner touching the positive post and the body makes a big spark.", "Batteries give off hydrogen: no flames or sparks nearby."],
    steps: [
      {
        title: "Disconnect the negative cable first",
        text: "Ignition off, key out. Loosen the negative (-) clamp and lift it off the post. Tuck it away so it cannot spring back. Then do the positive (+).",
        art: "battery_neg",
      },
      {
        title: "Brush the posts and clamps clean",
        text: "Scrub white or green corrosion off the posts and inside the clamps until the metal is bright. A paste of baking soda and water neutralises the acid.",
        art: "terminal_brush",
      },
      {
        title: "Refit positive, then negative, and tighten",
        text: "Clamps should not turn by hand once tightened. Check the ground strap from the battery or engine to the body too.",
        art: "battery_neg",
      },
      {
        title: "Let the app check the voltage",
        text: "Start the engine and run the battery and charging check: running voltage should sit between about 13.5 and 14.8 V.",
        art: "clear_codes",
      },
    ],
    verify: "Clamps are tight, and the charging check passes with lights and blower on.",
    causes: ["Loose or corroded battery cables or ground"],
    perCar: {
      bmw_e70: "On the E70 the battery is in the cargo area under the floor panel, on the right. The jump-start posts are under the bonnet.",
    },
  },
  {
    id: "battery-replace",
    title: "Replace the 12 V battery",
    summary: "Swap the battery for one of the same type (AGM or flooded) and size, and on cars that need it, register it so the charging system knows.",
    category: "electrical",
    part: "battery",
    partLabel: "Battery",
    difficulty: "diy",
    time: "20-45 min",
    tools: ["Sockets and extension", "Memory saver (optional)", "Gloves and eye protection"],
    parts: ["Battery of the same technology (AGM or flooded), capacity and size"],
    safety: [
      "A battery is heavy (15-30 kg). Lift with your legs and keep it upright.",
      "Negative off first, on last. No flames or sparks.",
      "Fit the same technology: an ordinary battery where the car expects AGM wears out fast.",
    ],
    steps: [
      {
        title: "Note the old battery's label",
        text: "Write down its type (AGM or flooded), capacity in Ah, and cold cranking amps. Buy the same.",
        art: "battery_label",
      },
      {
        title: "Disconnect negative, then positive",
        text: "Ignition off, key out. A memory saver plugged into the diagnostic port keeps radio and window settings.",
        art: "battery_neg",
      },
      {
        title: "Undo the hold-down and lift it out",
        text: "Remove the clamp or bracket at the base, disconnect the vent tube if there is one, and lift the battery out upright.",
        art: "battery_lift",
      },
      {
        title: "Fit the new battery",
        text: "Seat it fully, refit the hold-down so it cannot move, reconnect the vent tube, then positive and finally negative.",
        art: "battery_lift",
      },
      {
        title: "Register the battery if the car needs it",
        text: "Many cars with intelligent charging (most BMWs, many Mercedes, some others) must be told a new battery is fitted, or they keep charging it like the old worn one. ApexDiag's Special Functions has battery registration for the packs that support it.",
        art: "battery_register",
      },
    ],
    verify: "Engine starts easily, running voltage is 13.5-14.8 V, and on registered cars the battery history shows the new fit.",
    causes: ["Weak or ageing battery", "Auxiliary battery (stop-start cars)"],
    perCar: {
      bmw_e70: "On the E70 the battery is in the cargo area under the floor panel, on the right, and it must be registered after replacement (Special Functions, Battery registration).",
    },
  },
  {
    id: "fuel-cap",
    title: "Check the fuel cap",
    summary: "A loose or cracked fuel cap is the cheapest cause of an evaporative (EVAP) leak code.",
    category: "fuel",
    part: "filler",
    partLabel: "Fuel filler",
    difficulty: "diy",
    time: "5 min",
    tools: [],
    parts: ["Fuel cap (only if the seal is cracked)"],
    safety: ["No smoking or sparks near an open filler."],
    steps: [
      {
        title: "Find the filler side",
        text: "The small arrow next to the fuel symbol on the gauge points to the side the filler is on.",
        art: "fuel_cap",
      },
      {
        title: "Inspect the seal",
        text: "Take the cap off and look at the rubber seal for cracks, flat spots or dirt. Wipe the filler neck clean.",
        art: "fuel_cap",
      },
      {
        title: "Refit until it clicks",
        text: "Turn it clockwise until it clicks at least once. Replace it if the seal is damaged.",
        art: "fuel_cap",
      },
      {
        title: "Give it time",
        text: "The car re-runs its leak test only under certain conditions, often after a cold start with the tank a quarter to three quarters full. It can take several days of normal driving to confirm.",
        art: "clear_codes",
      },
    ],
    verify: "After a few days of driving the EVAP monitor shows complete and the code has not returned.",
    causes: ["Loose or worn fuel cap"],
  },
];

export function guideById(id: string) {
  return GUIDES.find((g) => g.id === id);
}

/** The guide that carries out a Repair Copilot cause, if there is one. */
export function guideForCause(cause: string) {
  return GUIDES.find((g) => g.causes?.includes(cause));
}
