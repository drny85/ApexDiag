# 3D cars and fix guides

## 3D cars (`src/components/car3d/`)

Every car is built in code at run time from a short list of measurements
(`specs.ts`): length, width, wheelbase, overhangs, tyre size, and side
profiles for the bonnet, sill and roof line. `build.ts` lofts the body from
those profiles, cuts real wheel arches, adds the glasshouse with
tumblehome, and lays lamps, grille, trim and shut lines as thin patches that
follow the body surface.

**Licensing.** No 3D model files are downloaded or bundled. The shapes are
our own geometry, so there is no third-party asset licence to track. The
only new dependency is [three.js](https://github.com/mrdoob/three.js)
(MIT licence). The cars follow each vehicle's published dimensions so the
proportions read right, but the styling is deliberately generic for the
body style: no maker logos, badges, grilles or other trade dress. Brands are
told apart by name, proportions and paint colour only. The number plate
reads "APX 001", which is made up.

**Performance.** three.js loads on demand (its own chunk), so it is not part
of the first paint. Lists of cars (Connect's practice cars, the Full Scan
verdict) use `CarStill`, which renders a PNG once with a single shared
off-screen renderer and caches it. Only one live, turnable `Car3D` is on a
screen at a time (two when a guide is open over Full Scan); each pauses
when off screen or when the app is in the background, and gives its WebGL
context back when it unmounts. This keeps us well under iPad Safari's limit
on live WebGL contexts. If WebGL is missing, every component falls back to
the earlier SVG drawing.

**Parts and anchors.** `buildCar` returns the meshes for each part
(headlights, tail lights, fog lamps, third brake light, plate light,
mirrors, fuel filler, wheels) and an anchor point for parts that are inside
the car (engine, battery, OBD port). `Car3D` takes `highlight` and `label`,
pulses the part in cyan, pins the label to it and turns the camera to face
it.

Battery and filler positions are only car-specific where we are sure (BMW
E70: battery in the cargo area, right side). Otherwise the anchor is a
generic engine-bay spot and the guide text says where to check.

## Fix guides (`src/lib/guides.ts`)

Fifteen guides in four groups: lights and bulbs (lamp type, halogen
headlight, xenon, LED units, tail/brake, third brake light, plate light,
front indicator), engine (air-flow sensor clean, smoke test, coil swap,
spark plugs), battery (terminals, replacement and registration) and fuel
(fuel cap). Each step has its own animated drawing in
`src/components/illustrations/GuideArt.tsx`.

All guides are general workshop practice and are labelled "General
guidance" on screen. They do not give part numbers, torques or bulb codes
for a specific car; they tell the reader to read the code on the old bulb,
the owner's manual, or the maker's service information. Car-specific notes
go in `perCar` only when verified.

Guides open from Repair Copilot: from the "Fix guides" list in the side
column, from the "Step-by-step guide" button on a matching common fix, and
as a card library when no car is connected.

Spanish for the guide text is part of the Repair Copilot translation
(Spanish part 2).
