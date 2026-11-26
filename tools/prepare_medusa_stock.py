"""Bind the original Medusa empty-dry source and stock configuration to Rust."""

import argparse
from gzip import decompress
from hashlib import sha256
import json
from pathlib import Path
import tomllib

ROOT = Path(__file__).resolve().parents[1]
PROOF = ROOT / "integration/sh171-medusa"
CONTRACT = ROOT / "assets/fleet/sh171/contract.toml"
RUST = ROOT / "crates/meridian-core/src/medusa.rs"
PINS = {
    "FINITE-STOCK.json": "2c0b9e0356183d894f375c6a30e5f70133fe4574f43b40ccf73400d17458ca27",
    "FINITE-STOCK-INPUTS.json": "5eb0f9dea0922aa2ae9806f2b22a18bb0084e5f784c24fc5ef326b9d80eefe9f",
    "FINITE-STOCK-PLAN.json": "6cef7eb607ec1d83d69206fc4721e515054d71965d356e7ff9e5fe8102a1fc56",
    "LEGACY-FINITE-BUILDER.py.txt": "ec5c62d1333e1f4ca5dc0b308d0f37ac2590bf4d1a42b56467f84d48e5549935",
    "LEGACY-FINITE-TESTS.py.txt": "112bfda3e45842d49fe0a2827ec1f53462a0ab10388f54f5e8cae5ca38df0eb1",
    "MEDUSA-FLAGSHIP.json": "0e2e6639721f77392cdc5566a0df3b9eb41b038d61564d7753b7f64d760b8771",
}
R3_SHA = "f1e1365586a3723bed5a87f1041d086ab7a7dd3c4ce9d665c6fedce1b39eb48f"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def proof_bytes(path, expected):
    data = path.read_bytes()
    require(sha256(data).hexdigest() == expected, f"Changed Medusa stock source: {path.name}")
    return data


def reference():
    sources = {name: proof_bytes(PROOF / name, expected) for name, expected in PINS.items()}
    model = json.loads(sources["FINITE-STOCK.json"])
    classes = json.loads(sources["MEDUSA-FLAGSHIP.json"])["configurations"]
    contract = tomllib.loads(proof_bytes(CONTRACT, R3_SHA).decode())
    require(model["id"] == contract["id"] == "SH171" and model["r3SHA256"] == R3_SHA,
            "Finite source identifies a different stock hull")
    require(model["planSHA256"] == PINS["FINITE-STOCK-PLAN.json"]
            and model["generatorSHA256"] == PINS["LEGACY-FINITE-BUILDER.py.txt"]
            and model["inputManifestSHA256"] == PINS["FINITE-STOCK-INPUTS.json"],
            "Changed finite plan, builder or input identity")
    require(model["preservedR3"] == {key: contract[key] for key in ("accommodation", "fitting", "acquisition")},
            "Changed finite record crew, fitting or purchase sources")
    require(classes["missionInstance"]["fullyUpgraded"] is False
            and classes["plughShowroom"]["freeMaximumFit"] is False
            and not contract["fitting"]["roleKitIncludedInBareHull"]
            and contract["medusaConfigurations"]["purchaseGrantsMaximumFit"] is False
            and contract["medusaConfigurations"]["expansionStatus"].startswith("proposal only"),
            "Stock reference grants mission upgrades, role kit or maximum equipment")
    require(model["nativeAdmissionBlocked"] is True and model["nativeFaceLimit"] == 512
            and model["nativeSourceTriangles"] == 76228 and model["operatingAuthorityGranted"] is False
            and model["occupiedPeopleOrPaidEquipmentGranted"] is False,
            "Stock reference grants flight or possession")
    parts = model["finiteIntrinsicHardware"]
    require(len(parts) == 14 and sum(part["massKg"] for part in parts) == 8640
            and all(part["certificates"][mesh]["noBoundaryContact"]
                    for part in parts for mesh in ("design", "nativeDesign")),
            "Changed source-box count, original intrinsic mass or containment")
    return model


def rust_source():
    model = reference()
    dry = model["emptyDryReference"]
    mass = int(dry["massKg"])
    require(mass == dry["massKg"] == 40712, "Changed stock dry mass")
    numerator, denominator = map(int, model["sourceScaleExact"].split("/"))

    def vector(values):
        return "".join(f"        {value!r},\n" for value in values)

    return (
        "//! Source-bound SH171 empty stock engineering.\n"
        "//! Prepared by tools/prepare_medusa_stock.py from the reviewed finite-equivalent model.\n\n"
        "use crate::mule::MuleEngineering;\n\n"
        "/// Empty dry reference for inspection and later hull admission.\n"
        "pub const STOCK_EMPTY_DRY: MuleEngineering = MuleEngineering {\n"
        f"    dry_mass_kilograms: {mass:_},\n"
        "    center_of_mass_metres: [\n"
        f"{vector(dry['centreMetres'])}"
        "    ],\n"
        "    inertia_about_center_kg_m2: [\n"
        f"{vector(dry['inertiaAboutCentreOfMassKgM2'])}"
        "    ],\n"
        f"    scale_numerator: {numerator},\n"
        f"    scale_denominator: {denominator},\n"
        "};\n"
    ).encode()


def stage_legacy(directory):
    """Restore the pinned authoring inputs without using a live Elite checkout."""
    copies = {
        "INPUTS.json": "FINITE-STOCK-INPUTS.json",
        "MEDUSA-FINITE-PLAN.json": "FINITE-STOCK-PLAN.json",
        "build-medusa-finite-source.py": "LEGACY-FINITE-BUILDER.py.txt",
        "test-medusa-finite-source.py": "LEGACY-FINITE-TESTS.py.txt",
    }
    files = {target: proof_bytes(PROOF / source, PINS[source]) for target, source in copies.items()}
    asset = CONTRACT.parent
    inputs_by_name = {
        "SH171-resource.toml": decompress((asset / "resource.toml.gz").read_bytes()),
        "SH171-model.json": decompress((asset / "native-export.json.gz").read_bytes()),
        "SH171-engineering.json": (asset / "engineering.json").read_bytes(),
        "SH171-ownership.json": (PROOF / "original-ownership.json").read_bytes(),
        "SH171-R3.toml": CONTRACT.read_bytes(),
    }
    inputs = json.loads(files["INPUTS.json"])
    require({row["file"] for row in inputs} == inputs_by_name.keys(), "Changed legacy input file inventory")
    for row in inputs:
        data = inputs_by_name[row["file"]]
        require(len(data) == row["bytes"] and sha256(data).hexdigest() == row["sha256"],
                f"Changed legacy input {row['file']}")
    directory.mkdir()
    (directory / "inputs").mkdir()
    for name, payload in files.items():
        (directory / name).write_bytes(payload)
    for name, payload in inputs_by_name.items():
        (directory / "inputs" / name).write_bytes(payload)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--stage-legacy", type=Path)
    args = parser.parse_args()
    if args.stage_legacy is not None:
        stage_legacy(args.stage_legacy)
        return
    output = rust_source()
    if args.check:
        require(RUST.read_bytes() == output, "Rust stock reference differs from the finite source")
    else:
        RUST.write_bytes(output)


if __name__ == "__main__":
    main()
